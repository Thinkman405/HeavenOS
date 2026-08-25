//! Physics assertions for the substrate.
//!
//! Doctrine: `_mkb/test-doctrine.md`. Plan:
//! `subsystems/substrate/03_tests/output/test-plan.md`.
//!
//! **[D]** marks assertions a conventional flat-memory, byte-addressed
//! implementation could not pass.

use substrate::constants::*;
use substrate::memory::{CellOffset, LatticeAddress, MemoryPool};
use substrate::translation as tr;
use substrate::{Hypervisor, SubstrateError, TrapAction, CARRIER};

// ------------------------------------------- Group 1: the flat/curved boundary

/// 1.2 [D] - address distance is hyperbolic, cross-checked against `lattice`.
#[test]
fn address_distance_is_hyperbolic() {
    let pool = MemoryPool::new(31, 256);
    let a = pool.address_at(0).unwrap();
    let b = pool.address_at(7).unwrap();
    let d = pool.distance(a, b);

    assert!(d > 0.0 && d.is_finite());
    // Independently, via lattice's own metric.
    let t = lattice::Tiling::grow(4);
    let ca = t.get(&a.cell()).unwrap().centre();
    let cb = t.get(&b.cell()).unwrap().centre();
    let expected = ca.distance_to(&cb);
    assert!(
        (d - expected).abs() < 1e-12,
        "pool reported {d}, lattice metric says {expected}"
    );
}

/// 1.3 [D] - distance is not any linear function of the addresses.
///
/// A flat implementation returns `|a - b|`; adjacent cells here sit at
/// `2 x inradius ~ 1.2537`, which no offset difference reproduces.
#[test]
fn distance_is_not_arithmetic_on_offsets() {
    let pool = MemoryPool::new(31, 256);
    let a = pool.address_at(0).unwrap();
    let b = pool.address_at(1).unwrap();
    let d = pool.distance(a, b);
    assert!(
        (d - 1.253_739_325_812_356).abs() < 1e-9,
        "adjacent cells must sit at 2*inradius, got {d}"
    );
    // Offsets are both zero, so any offset-difference metric would give 0.
    assert_eq!(a.offset(), CellOffset(0));
    assert_eq!(b.offset(), CellOffset(0));
    assert!(d > 1.0, "a linear metric on equal offsets would give 0");
}

/// 1.4 - an address is zero distance from itself.
#[test]
fn distance_to_self_is_zero() {
    let pool = MemoryPool::new(7, 128);
    let a = pool.address_at(0).unwrap();
    assert_eq!(pool.distance(a, a), 0.0);
}

// ------------------------------------------------ Group 2: allocation locality

/// 2.1 [D] - a multi-cell allocation occupies **adjacent** cells.
///
/// This is the property `ftg` routing depends on. A flat allocator hands back
/// consecutive indices, which are not adjacent in the lattice.
#[test]
fn multi_cell_allocations_are_lattice_adjacent() {
    let mut pool = MemoryPool::new(31, 64);
    let alloc = pool.allocate(64 * 6).expect("fits");
    let cells = alloc.cells();
    assert!(cells.len() >= 2, "expected a multi-cell allocation");

    for (i, cell) in cells.iter().enumerate().skip(1) {
        let touches = cells[..i].iter().any(|prev| pool.adjacent(*prev, *cell));
        assert!(
            touches,
            "cell {i} of the allocation is not adjacent to any earlier cell"
        );
    }
}

/// 2.2 - allocation cells are distinct.
#[test]
fn allocation_cells_are_distinct() {
    let mut pool = MemoryPool::new(31, 64);
    let alloc = pool.allocate(64 * 5).expect("fits");
    let mut sorted = alloc.cells().to_vec();
    sorted.sort();
    let before = sorted.len();
    sorted.dedup();
    assert_eq!(sorted.len(), before);
}

/// 2.3 - write then read returns the same bytes.
#[test]
fn write_read_round_trip() {
    let mut pool = MemoryPool::new(7, 256);
    let alloc = pool.allocate(16).expect("fits");
    let data = b"hyperbolic bytes";
    pool.write(alloc.start(), data).unwrap();
    assert_eq!(pool.read(alloc.start(), data.len()).unwrap(), data);
}

/// 2.4 - writes to one cell do not disturb another.
#[test]
fn writes_are_isolated_between_cells() {
    let mut pool = MemoryPool::new(7, 64);
    let a = pool.address_at(0).unwrap();
    let b = pool.address_at(1).unwrap();
    pool.write(a, &[0xAA; 8]).unwrap();
    pool.write(b, &[0x55; 8]).unwrap();
    assert_eq!(pool.read(a, 8).unwrap(), vec![0xAA; 8]);
    assert_eq!(pool.read(b, 8).unwrap(), vec![0x55; 8]);
}

/// 2.5 - an over-capacity request fails cleanly.
#[test]
fn oversized_allocation_is_refused() {
    let mut pool = MemoryPool::new(7, 64);
    assert!(matches!(
        pool.allocate(7 * 64 + 1),
        Err(SubstrateError::Exhausted { .. })
    ));
}

/// 2.6 - writing past the end of a cell is refused.
#[test]
fn offset_beyond_cell_is_refused() {
    let mut pool = MemoryPool::new(7, 32);
    let a = pool.address_at(0).unwrap();
    let at = LatticeAddress::new(a.cell(), CellOffset(30));
    assert!(matches!(
        pool.write(at, &[0u8; 8]),
        Err(SubstrateError::OffsetOutOfCell { .. })
    ));
}

/// 2.7 - freeing makes space reusable.
#[test]
fn freed_cells_are_reusable() {
    let mut pool = MemoryPool::new(7, 64);
    let all = pool.allocate(7 * 64).expect("exactly fits");
    assert!(pool.allocate(1).is_err());
    pool.free(&all);
    assert!(pool.allocate(64).is_ok());
}

/// 2.7b [D] — **freeing one allocation must not free a sibling that shares
/// its cell, and must free exactly, immediately, at sub-cell granularity.**
/// No concurrency involved: found while building `symphony-kernel::ConcurrentPool`,
/// but reproduces in a single thread. `free` used to reset a cell's `used`
/// count to zero unconditionally, regardless of whether another still-live
/// allocation also had bytes in that same cell — freeing the first of two
/// allocations sharing a cell reported the *whole* cell available again, and
/// a subsequent `allocate` would then hand the second allocation's
/// still-live bytes to someone else. The first fix closed that safely but
/// pessimistically (nothing reclaimed until the whole cell emptied); this
/// is the sharper version — b's own bytes are reusable the instant it frees,
/// with no wait on a.
#[test]
fn freeing_one_allocation_does_not_free_a_sibling_sharing_its_cell() {
    let mut pool = MemoryPool::new(1, 256); // one cell, room for four 64-byte spans
    let a = pool.allocate(64).unwrap();
    let b = pool.allocate(64).unwrap();
    assert_eq!(a.cells(), b.cells(), "premise: both must land in the same cell");

    pool.write(a.start(), &[1u8; 64]).unwrap();
    pool.write(b.start(), &[2u8; 64]).unwrap();

    pool.free(&b);
    assert_eq!(
        pool.available(),
        256 - 64,
        "b's own 64 bytes must be reclaimed immediately -- sub-cell reuse, \
         not waiting on a (still live) to also free"
    );

    // A third allocation should land exactly where b was -- proof the freed
    // sub-cell hole is genuinely reused, not merely tolerated.
    let c = pool.allocate(64).unwrap();
    assert_eq!(
        c.start(),
        b.start(),
        "c should reuse b's exact freed hole (first-fit)"
    );
    assert_ne!(
        c.start(),
        a.start(),
        "c must not overlap a, which is still alive"
    );
    pool.write(c.start(), &[3u8; 64]).unwrap();

    assert_eq!(
        pool.read(a.start(), 64).unwrap(),
        vec![1u8; 64],
        "a's data must survive freeing its sibling and a subsequent allocation"
    );
}

/// 2.7c [D] — a gap freed in the **middle** of a cell (not just the most
/// recently allocated span) is reused first-fit, ahead of the untouched
/// tail — proof this is real interval bookkeeping, not merely "the last
/// thing freed happens to be reusable."
#[test]
fn freeing_a_middle_allocation_creates_a_reusable_gap() {
    let mut pool = MemoryPool::new(1, 256); // room for four 64-byte spans
    let a = pool.allocate(64).unwrap(); // offset 0
    let b = pool.allocate(64).unwrap(); // offset 64
    let c = pool.allocate(64).unwrap(); // offset 128
    // offset 192..256 is untouched tail space.

    pool.write(a.start(), &[1u8; 64]).unwrap();
    pool.write(b.start(), &[2u8; 64]).unwrap();
    pool.write(c.start(), &[3u8; 64]).unwrap();

    pool.free(&b); // frees exactly the middle 64 bytes, offset 64..128

    let d = pool.allocate(64).unwrap();
    assert_eq!(
        d.start(),
        b.start(),
        "the middle gap must be offered before the untouched 192..256 tail"
    );

    pool.write(d.start(), &[4u8; 64]).unwrap();
    assert_eq!(pool.read(a.start(), 64).unwrap(), vec![1u8; 64], "a undisturbed");
    assert_eq!(pool.read(c.start(), 64).unwrap(), vec![3u8; 64], "c undisturbed");
}

// ------------------------------------- Group 2b: LogicalArea size reporting
//
// `lattice::LogicalArea` proved fragmentation structurally zero in
// isolation — a bare cell count, never a real pool. These wire that
// property to `MemoryPool`'s actual allocation lifecycle.

/// 2.8 — the pool's total footprint is exactly its cell count, in
/// `LogicalArea` terms.
#[test]
fn total_area_matches_cell_count() {
    let pool = MemoryPool::new(31, 64);
    let total = pool.total_area();
    assert_eq!(total.cells(), 31);
    assert!((total.area() - 31.0 * lattice::LogicalArea::unit_area()).abs() < 1e-12);
}

/// 2.9 — a fresh pool has nothing occupied; every cell is available.
#[test]
fn fresh_pool_is_entirely_available() {
    let pool = MemoryPool::new(11, 64);
    assert_eq!(pool.occupied_area().cells(), 0);
    assert_eq!(pool.available_area().cells(), pool.total_area().cells());
}

/// 2.10 [D] — an allocation's own `logical_area` matches exactly how many
/// cells it actually occupies, and the pool's `occupied_area` reflects it —
/// then `free` returns the pool to fully available.
///
/// Sized to whole multiples of `cell_capacity` deliberately, so the
/// allocation cannot share a partially-used cell with anything else — the
/// geometric area of *this specific allocation* would otherwise be
/// ambiguous when a cell's byte capacity is split across two allocations.
#[test]
fn allocation_occupies_exactly_its_own_cells_then_frees_them() {
    let mut pool = MemoryPool::new(31, 64);
    let alloc = pool.allocate(64 * 5).expect("fits, exact cell multiple");

    assert_eq!(alloc.logical_area().cells(), alloc.cells().len());
    assert_eq!(pool.occupied_area().cells(), alloc.cells().len());
    assert_eq!(
        pool.available_area().cells(),
        pool.total_area().cells() - alloc.cells().len()
    );

    pool.free(&alloc);
    assert_eq!(pool.occupied_area().cells(), 0, "freeing must return every cell to available");
    assert_eq!(pool.available_area().cells(), pool.total_area().cells());
}

/// 2.10b [D] — **a partially-used cell still counts as fully occupied.**
///
/// `LogicalArea` has no notion of a fractional cell, matching the pool's own
/// zero-fragmentation invariant: a cell holding even one byte is "spoken
/// for" at the geometric level, identically to a cell holding
/// `cell_capacity` bytes. Every other test in this group uses exact
/// multiples of `cell_capacity` to avoid a *different* ambiguity (two
/// allocations sharing one cell) — which means, left unchecked, they could
/// not tell "any bytes" from "completely full" apart. This is the test that
/// can.
#[test]
fn a_partially_used_cell_counts_as_fully_occupied() {
    let mut pool = MemoryPool::new(11, 64);
    let alloc = pool.allocate(30).expect("well under one cell");
    assert_eq!(alloc.cells().len(), 1, "premise: this must land in exactly one cell");

    assert_eq!(
        pool.occupied_area().cells(),
        1,
        "a cell with 30 of 64 bytes used must still count as occupied"
    );
    assert_eq!(pool.available_area().cells(), pool.total_area().cells() - 1);
}

/// 2.11 [D] — occupied and available cell counts always partition the
/// pool's total exactly, across a sequence of allocations and frees, not
/// just at the two endpoints already checked above.
#[test]
fn occupied_and_available_cell_counts_always_partition_exactly() {
    let mut pool = MemoryPool::new(31, 64);
    let mut live = Vec::new();

    for _ in 0..4 {
        live.push(pool.allocate(64 * 3).expect("fits"));
        assert_eq!(
            pool.occupied_area().cells() + pool.available_area().cells(),
            pool.total_area().cells(),
            "occupied + available must exactly equal total after allocating"
        );
    }
    while let Some(alloc) = live.pop() {
        pool.free(&alloc);
        assert_eq!(
            pool.occupied_area().cells() + pool.available_area().cells(),
            pool.total_area().cells(),
            "occupied + available must exactly equal total after freeing"
        );
    }
}

/// 2.12 [D] — **area is conserved only up to floating point, and that is
/// checked rather than assumed.**
///
/// `occupied.area() + available.area()` is not bit-identical to
/// `total.area()` in general — `(a+b)*c != a*c+b*c` for arbitrary integer
/// splits in `f64`. Verified separately before this test was written: worst
/// observed absolute gap was `~1.8e-12` on a several-thousand-cell split,
/// which is `~1.5e-16` relative — at the `f64` epsilon floor, not a real
/// divergence. This test uses a relative tolerance for exactly that reason,
/// on a pool large enough to actually exercise the effect.
#[test]
fn area_is_conserved_across_allocation_and_freeing() {
    let mut pool = MemoryPool::new(441, 64);
    let total = pool.total_area().area();

    let mut live = Vec::new();
    for _ in 0..30 {
        live.push(pool.allocate(64 * 7).expect("fits"));
        let sum = pool.occupied_area().area() + pool.available_area().area();
        assert!(
            ((sum - total) / total).abs() < 1e-9,
            "area must be conserved (relatively) after allocating: {sum} vs {total}"
        );
    }
    for alloc in live {
        pool.free(&alloc);
        let sum = pool.occupied_area().area() + pool.available_area().area();
        assert!(
            ((sum - total) / total).abs() < 1e-9,
            "area must be conserved (relatively) after freeing: {sum} vs {total}"
        );
    }
}

// -------------------------------------------------- Group 3: pool split (A1)

/// 3.1 [D] - the unit split is exactly 2. Scalar duplication gives 1.
#[test]
fn unit_pool_split_is_exactly_two() {
    let mut pool = MemoryPool::new(7, 64);
    assert_eq!(pool.extent(), 1.0);
    assert_eq!(pool.split().unwrap(), 2.0);
    assert_eq!(pool.extent(), 2.0);
}

/// 3.2 [D] - splitting is geometric, not doubling.
///
/// After the first split the extent is 2; a copy-based implementation would
/// then give 4. The `(x)` operator gives something else entirely.
#[test]
fn split_is_geometric_not_doubling() {
    let mut pool = MemoryPool::new(7, 64);
    pool.split().unwrap();
    let second = pool.split().unwrap();
    assert_ne!(second, 4.0, "doubling would give 4; (x) does not");
    assert!(second > 4.0, "geometric split exceeds naive doubling");
}

/// 3.3 - the operator's domain is enforced.
#[test]
fn split_respects_operator_domain() {
    let mut pool = MemoryPool::new(7, 64);
    let mut last = Ok(0.0);
    for _ in 0..8 {
        last = pool.split();
        if last.is_err() {
            break;
        }
    }
    assert!(
        matches!(last, Err(SubstrateError::SplitDomain { .. })),
        "repeated splitting must eventually leave the domain, got {last:?}"
    );
    assert!(OTIMES_DOMAIN_MAX_PRODUCT > 800.0);
}

// -------------------------------------------- Group 4: binary/wave translation

/// 4.1 - the bit/phase mapping is A2's pair, from the MKB.
#[test]
fn bits_map_to_the_permitted_phases() {
    let phases = tr::bits_to_phases(&[0b1000_0000]);
    assert_eq!(phases.len(), 8);
    assert!((phases[0] - PHASE_TRUE).abs() < 1e-15);
    assert!((phases[1] - PHASE_FALSE).abs() < 1e-15);
    assert!((PHASE_TRUE - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
    assert!((PHASE_FALSE + std::f64::consts::FRAC_PI_2).abs() < 1e-15);
}

/// 4.2 - the round trip is lossless over varied patterns.
#[test]
fn bit_phase_round_trip_is_lossless() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0x00],
        vec![0xFF],
        vec![0xAA, 0x55],
        b"NEOS substrate".to_vec(),
        (0..=255u8).collect(),
    ];
    for bytes in cases {
        let phases = tr::bits_to_phases(&bytes);
        assert_eq!(tr::phases_to_bits(&phases).unwrap(), bytes);
    }
}

/// 4.3 [D] - **a zero crossing recovers nothing, and must say so.**
///
/// Both bit states evaluate to exactly zero at `t = 0` and every half period.
/// Returning bits there would be fabricating them - and would surface as
/// intermittent corruption at every layer above, with no local cause.
#[test]
fn zero_crossing_recovers_nothing() {
    let phases = tr::bits_to_phases(b"x");
    assert!(matches!(
        tr::demodulate(&phases, 0.0),
        Err(SubstrateError::ZeroCrossing { .. })
    ));

    let half = CARRIER.period() / 2.0;
    assert!(matches!(
        tr::demodulate(&phases, half),
        Err(SubstrateError::ZeroCrossing { .. })
    ));
    assert!(tr::is_zero_crossing(0.0));
    assert!(tr::is_zero_crossing(CARRIER.period()));

    // The measurement behind the rule: separation is exactly zero there.
    let sep = (tr::carrier_at(PHASE_TRUE, 0.0, 1.0) - tr::carrier_at(PHASE_FALSE, 0.0, 1.0)).abs();
    assert!(sep < 1e-12, "expected zero separation at t=0, got {sep}");
}

/// 4.4 [D] - quarter-period separation is maximal, exactly 2.0.
#[test]
fn quarter_period_separation_is_maximal() {
    let t = tr::safe_sample_instant(0);
    assert!(!tr::is_zero_crossing(t));
    let sep = (tr::carrier_at(PHASE_TRUE, t, 1.0) - tr::carrier_at(PHASE_FALSE, t, 1.0)).abs();
    assert!(
        (sep - 2.0).abs() < 1e-12,
        "quarter-period separation should be 2.0, got {sep}"
    );
    assert!((t - CARRIER.quarter_period()).abs() < 1e-22);
}

/// 4.5 - the full pipeline round trips at safe instants.
#[test]
fn demodulation_round_trips_at_safe_instants() {
    let msg = b"New Earth";
    let phases = tr::bits_to_phases(msg);
    for k in 0..6 {
        let t = tr::safe_sample_instant(k);
        assert_eq!(tr::demodulate(&phases, t).unwrap(), msg, "failed at k = {k}");
    }
}

/// 4.6 [D] - opposed phases cancel at **every** `t`, not just sample points.
#[test]
fn opposed_phases_cancel_continuously() {
    for i in 1..200 {
        let t = CARRIER.period() * (i as f64) / 200.0;
        let s = tr::superpose(PHASE_TRUE, PHASE_FALSE, t, 1.0);
        assert!(s.abs() < 1e-15, "residual {s} at t = {t}");
    }
}

/// 4.7 - an off-axis phase is not a logic state.
#[test]
fn off_axis_phase_is_refused() {
    assert!(matches!(
        tr::phases_to_bits(&[0.0; 8]),
        Err(SubstrateError::IndeterminatePhase { .. })
    ));
    assert!(matches!(
        tr::phases_to_bits(&[PHASE_TRUE; 3]),
        Err(SubstrateError::IndeterminatePhase { .. })
    ));
}

// ------------------------------------------------ Group 5: clock and frequency

/// 5.1 / 5.3 - the carrier and its quarter period.
#[test]
fn carrier_matches_the_mkb() {
    assert!((CARRIER.get() - CARRIER_RAD_PER_SEC).abs() < 1e-6);
    assert!((CARRIER.get() - 6_283_185_307.179_586).abs() < 1e-6);
    assert!((CARRIER.quarter_period() - 2.5e-10).abs() < 1e-22);
}

/// 5.4 - `tick` advances exactly one quarter period.
#[test]
fn tick_advances_one_quarter_period() {
    let mut hv = Hypervisor::boot(7, 64);
    assert_eq!(hv.uptime_seconds(), 0.0);
    let after = hv.tick();
    assert!((after - CARRIER.quarter_period()).abs() < 1e-22);
    for _ in 0..3 {
        hv.tick();
    }
    assert_eq!(hv.ticks(), 4);
    assert!((hv.uptime_seconds() - CARRIER.period()).abs() < 1e-20);
}

/// 5.5 - **one home for the frequency types.**
///
/// `symphony_kernel` re-exports substrate's types rather than defining its own.
/// If they were distinct types this assignment would not compile.
#[test]
fn frequency_types_have_exactly_one_home() {
    let f: substrate::Frequency = symphony_kernel::Frequency::hertz(2.5e9);
    let w: substrate::AngularFrequency = symphony_kernel::AngularFrequency::rad_per_sec(1.0);
    assert_eq!(f.get(), 2.5e9);
    assert_eq!(w.get(), 1.0);

    // And the carrier still cannot be priced as an ordinary frequency.
    let energy = symphony_kernel::energy(CARRIER.to_ordinary());
    assert!(energy.0 > 0.0);
}

// ------------------------------------------------------- Group 6: integration

/// The hypervisor boots, allocates, translates, and splits.
#[test]
fn hypervisor_bootstraps() {
    let mut hv = Hypervisor::boot(31, 1024);
    assert_eq!(hv.pool().cell_count(), 31);
    assert_eq!(hv.pool().total_capacity(), 31 * 1024);

    let alloc = hv.pool_mut().allocate(4096).expect("fits");
    assert!(alloc.cells().len() >= 4);

    let msg = b"boot";
    let phases = tr::bits_to_phases(msg);
    let t = tr::safe_sample_instant(1);
    assert_eq!(tr::demodulate(&phases, t).unwrap(), msg);

    assert_eq!(hv.pool_mut().split().unwrap(), 2.0);
}

// -------------------------------------------- Group 7: curved address resolution
//
// `resolve_path` is the join this pool and `lattice::addressing` were each
// missing half of: addressing folds `(x)` to a scalar and stops; the pool
// allocates over `CellId`s and stops. This is the front door that finally
// turns a path someone actually wrote into an address this pool holds bytes
// for.

/// 7.1 — the identity path resolves to the pool's own start cell.
///
/// Ring 0 of any tiling is the origin alone, so `address_at(0)` is always
/// `CellId::ORIGIN` for a non-empty pool, and the empty `AddressPath` folds to
/// exactly `0.0` - the two must agree without any special-casing.
#[test]
fn identity_path_resolves_to_the_pool_start() {
    let pool = MemoryPool::new(31, 64);
    let identity = lattice::AddressPath::new(0.0, &[]);
    assert_eq!(pool.resolve_path(&identity).unwrap(), pool.address_at(0).unwrap());
}

/// 7.2 [D] - **a path can name a point outside this pool's backing store.**
///
/// The pool's own `Tiling` is grown to a depth that comfortably exceeds the
/// handful of cells the pool actually maps (`MemoryPool::new` grows by ring
/// until `cells` are covered, then keeps growing to the depth boundary). A
/// path resolving to one of those unmapped-but-grown cells names a real point
/// in the address space that is not part of this pool - a flat allocator has
/// no such distinction, since every in-bounds index is mapped by definition.
#[test]
fn a_resolvable_path_can_still_be_unmapped_in_a_small_pool() {
    let pool = MemoryPool::new(1, 64);
    let path = lattice::AddressPath::new(1.0, &[1.0]); // scalar = 2.0, off-origin
    assert!(
        matches!(
            pool.resolve_path(&path),
            Err(SubstrateError::Unmapped { .. })
        ),
        "a single-cell pool must not accept an address outside its one cell"
    );
}

/// 7.3 - resolve, then write and read through the *same* resolved address -
/// the curved front door and the byte-level back door agree on what they
/// named.
#[test]
fn resolved_address_writes_and_reads_back() {
    let mut pool = MemoryPool::new(31, 64);
    let path = lattice::AddressPath::new(0.0, &[]);
    let addr = pool.resolve_path(&path).unwrap();

    let payload = b"curved";
    pool.write(addr, payload).unwrap();
    assert_eq!(pool.read(addr, payload.len()).unwrap(), payload);
}

/// 7.4 [D] - **domain refusal surfaces as a named substrate error, not a
/// panic**, and carries the underlying `lattice` failure rather than losing
/// it.
#[test]
fn dissonant_path_is_refused_not_panicked() {
    let pool = MemoryPool::new(31, 64);
    let too_far = lattice::AddressPath::new(1.0, &[1.0, 1.0, 1.0, 1.0, 1.0]);
    match pool.resolve_path(&too_far) {
        Err(SubstrateError::AddressUnresolvable(lattice::LatticeError::Dissonant { .. })) => {}
        other => panic!("expected a wrapped Dissonant refusal, got {other:?}"),
    }
}

/// 7.5 - resolution is deterministic through the pool, same as through
/// `lattice` directly.
#[test]
fn pool_resolution_is_deterministic() {
    let pool = MemoryPool::new(31, 64);
    let path = lattice::AddressPath::new(1.3, &[0.4]);
    let first = pool.resolve_path(&path).unwrap();
    for _ in 0..20 {
        assert_eq!(pool.resolve_path(&path).unwrap(), first);
    }
}

/// 7.6 - a large enough pool maps the cell an ordinary path resolves to,
/// completing the round trip end to end: source path -> real cell -> real
/// bytes, with no cell named that the pool cannot actually back.
#[test]
fn a_large_enough_pool_maps_an_ordinary_resolved_cell() {
    let mut pool = MemoryPool::new(200, 64);
    let path = lattice::AddressPath::new(1.0, &[1.0]);
    let addr = pool.resolve_path(&path).expect("a 200-cell pool must map this");
    assert_ne!(
        addr.cell(),
        lattice::tessellation::CellId::ORIGIN,
        "this path must not collapse to the origin"
    );

    pool.write(addr, b"resonant").unwrap();
    assert_eq!(pool.read(addr, 8).unwrap(), b"resonant");
}

// --------------------------------------------------- Group 8: fault trapping
//
// `Hypervisor::allocate_trapped` — a real fault path, scoped honestly: not
// guest isolation or privilege levels (nothing here executes untrusted guest
// code for those to protect), but a genuine trap dispatch where control
// really transfers to a handler on every fault, and the handler can act on
// the same pool the fault came from.

/// The handler is invoked on every single fault, unconditionally — a real
/// transfer of control, not a conditional notification.
#[test]
fn handler_is_invoked_on_every_fault() {
    let mut hv = Hypervisor::boot(1, 64);
    let _filler = hv.pool_mut().allocate(64).unwrap(); // the only cell is now full

    let mut calls = 0;
    let result = hv.allocate_trapped(32, 0, |fault, _pool| {
        calls += 1;
        assert!(matches!(fault, SubstrateError::Exhausted { .. }));
        TrapAction::Propagate
    });

    assert!(result.is_err());
    assert_eq!(calls, 1, "the handler must see the fault exactly once");
}

/// [D] **Real recovery, not a simulation of it.** The handler frees an
/// allocation it knows about, directly on the same pool the fault came
/// from, and the retried allocation actually succeeds — the same shape as
/// a page fault a real OS resolves by evicting a page, not a closure
/// pretending to help while the underlying resource stays exhausted.
#[test]
fn handler_can_recover_by_freeing_and_retrying() {
    let mut hv = Hypervisor::boot(1, 128);
    let filler = hv.pool_mut().allocate(128).unwrap(); // fills the only cell

    let mut freed = false;
    let result = hv.allocate_trapped(64, 1, move |fault, pool| {
        assert!(matches!(fault, SubstrateError::Exhausted { .. }));
        if !freed {
            pool.free(&filler);
            freed = true;
            TrapAction::Retry
        } else {
            TrapAction::Propagate
        }
    });

    assert!(
        result.is_ok(),
        "the handler freed the filler; the retried allocation should have succeeded, got {result:?}"
    );
}

/// A handler that declines outright propagates the original fault
/// immediately, without retrying even once.
#[test]
fn declining_the_fault_propagates_without_retrying() {
    let mut hv = Hypervisor::boot(1, 64);
    let _filler = hv.pool_mut().allocate(64).unwrap();

    let mut calls = 0;
    let result = hv.allocate_trapped(32, 5, |_fault, _pool| {
        calls += 1;
        TrapAction::Propagate
    });

    assert!(matches!(result, Err(SubstrateError::Exhausted { .. })));
    assert_eq!(calls, 1, "declining on the first fault must not retry");
}

/// **The safety guard.** A handler that always asks to retry, but never
/// actually resolves the fault, cannot hang the caller: `max_retries`
/// bounds the loop, and the handler is called exactly `max_retries + 1`
/// times — once for the original fault, once per retry.
#[test]
fn retries_are_bounded_even_when_the_handler_never_gives_up() {
    let mut hv = Hypervisor::boot(1, 64);
    let _filler = hv.pool_mut().allocate(64).unwrap();

    let mut calls = 0;
    let result = hv.allocate_trapped(32, 3, |_fault, _pool| {
        calls += 1;
        TrapAction::Retry // never actually frees anything
    });

    assert!(matches!(result, Err(SubstrateError::Exhausted { .. })));
    assert_eq!(calls, 4, "expected 1 initial fault + 3 retries = 4 calls, got {calls}");
}

/// A handler is never invoked at all when the allocation simply succeeds —
/// trapping is for faults, not a hook on every call.
#[test]
fn a_successful_allocation_never_touches_the_handler() {
    let mut hv = Hypervisor::boot(4, 64);
    let mut calls = 0;
    let result = hv.allocate_trapped(32, 0, |_fault, _pool| {
        calls += 1;
        TrapAction::Propagate
    });
    assert!(result.is_ok());
    assert_eq!(calls, 0);
}
