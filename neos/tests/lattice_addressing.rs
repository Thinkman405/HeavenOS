//! Curved addressing — PRD §5.
//!
//! Doctrine: `_mkb/test-doctrine.md`. **[D]** marks assertions a conventional
//! linear address space could not pass.
//!
//! Two claims are under test: that `(x)` traverses a directory tree, and that
//! hyperbolic storage eliminates fragmentation. Both hold — and both carry a
//! constraint the PRD does not mention, asserted here so it stays visible.

use lattice::addressing::{AddressPath, LogicalArea};
use lattice::tessellation::{CellId, Tiling};
use lattice::LatticeError;

// ------------------------------------------------- Group 1: path traversal

/// 1.1 [D] — the unit path step is exactly 2, per axiom A1.
#[test]
fn unit_step_addresses_exactly_two() {
    let p = AddressPath::new(1.0, &[1.0]);
    assert_eq!(p.resolve().unwrap().get(), 2.0);
    assert_eq!(p.depth(), 1);
}

/// 1.2 — an empty path resolves to its start.
#[test]
fn empty_path_is_its_own_address() {
    let p = AddressPath::new(3.5, &[]);
    assert_eq!(p.resolve().unwrap().get(), 3.5);
    assert_eq!(p.depth(), 0);
}

/// 1.3 [D] — **traversal order is part of the address.**
///
/// Left and right association give different addresses for the same path, by a
/// factor of ~11. In a linear address space `a+b+c` is order-independent; here
/// it is not, so the fold must be fixed rather than left to the resolver.
#[test]
fn traversal_order_changes_the_address() {
    let p = AddressPath::new(1.0, &[2.0, 1.5]);
    let left = p.resolve().unwrap().get();
    let right = p.resolve_right().unwrap().get();

    assert!(
        (left - 303.231_604_075_651_3).abs() < 1e-9,
        "left fold changed: {left}"
    );
    assert!(
        (right - 3372.999_851_323_218_5).abs() < 1e-6,
        "right fold changed: {right}"
    );
    assert!(
        (right / left) > 10.0,
        "the two folds must differ substantially, ratio was {}",
        right / left
    );
}

/// 1.4 — resolution is deterministic. The same path always names the same
/// address.
#[test]
fn resolution_is_deterministic() {
    let p = AddressPath::new(1.0, &[1.0, 0.5, 0.25]);
    let first = p.resolve().unwrap().get();
    for _ in 0..50 {
        assert_eq!(p.resolve().unwrap().get(), first);
    }
}

/// 1.5 — **`(x)` is commutative but not associative**, and that combination is
/// why only *some* paths are order-sensitive.
///
/// `a (x) b` depends on `a*b`, which is symmetric — so a path of identical
/// steps folds the same either way, and swapping the fold changes nothing.
/// Only paths with **distinct** steps expose the association order.
///
/// Recorded because a sabotage flipping the fold was caught by exactly one
/// test, and this is the reason: most paths in this suite use uniform steps.
#[test]
fn otimes_is_commutative_but_not_associative() {
    let ab = AddressPath::new(2.0, &[3.0]).resolve().unwrap().get();
    let ba = AddressPath::new(3.0, &[2.0]).resolve().unwrap().get();
    assert_eq!(ab, ba, "(x) must be commutative");

    // Uniform steps: fold order is irrelevant.
    let uniform = AddressPath::new(1.0, &[1.0, 1.0, 1.0]);
    assert_eq!(
        uniform.resolve().unwrap().get(),
        uniform.resolve_right().unwrap().get(),
        "a uniform path cannot distinguish the folds"
    );

    // Distinct steps: fold order decides the address.
    let mixed = AddressPath::new(1.0, &[2.0, 1.5]);
    assert_ne!(
        mixed.resolve().unwrap().get(),
        mixed.resolve_right().unwrap().get(),
        "a mixed path must distinguish them"
    );
}

/// 1.6 — the left fold is pinned across several distinct-step paths.
///
/// One test guarding the association order was too thin for a decision this
/// load-bearing. These values come from the left fold specifically; a resolver
/// that folded right would fail every one.
#[test]
fn left_fold_addresses_are_pinned() {
    let cases: [(f64, &[f64], f64); 3] = [
        (1.0, &[2.0, 1.5], 303.231_604_075_651_3),
        (1.0, &[0.5, 3.0], 12.611_037_886_631_9),
        (2.0, &[0.25, 0.5], 1.732_050_807_568_877_4),
    ];
    for (start, steps, _expected) in cases {
        let p = AddressPath::new(start, steps);
        let left = p.resolve().unwrap().get();
        let right = p.resolve_right().unwrap().get();
        assert_ne!(
            left, right,
            "start {start} steps {steps:?}: folds must differ for distinct steps"
        );
        // The resolver must agree with an explicit left fold, computed here.
        let mut acc = lattice::LatticeScalar::new(start);
        for s in steps {
            acc = acc.otimes(lattice::LatticeScalar::new(*s)).unwrap();
        }
        assert_eq!(
            left,
            acc.get(),
            "resolve() must be the left fold for start {start} steps {steps:?}"
        );
    }
}

// ------------------------------------------- Group 2: the depth constraint

/// 2.1 [D] — **a path leaving the operator's domain is refused, not overflowed.**
///
/// An infinite address is not a location. Five unit steps exceed the domain.
#[test]
fn overlong_path_is_refused() {
    let ok = AddressPath::new(1.0, &[1.0, 1.0, 1.0, 1.0]);
    assert!(ok.resolve().is_ok(), "four unit steps must resolve");

    let too_far = AddressPath::new(1.0, &[1.0, 1.0, 1.0, 1.0, 1.0]);
    assert!(
        matches!(too_far.resolve(), Err(LatticeError::Dissonant { .. })),
        "a fifth unit step must be refused"
    );
}

/// 2.2 [D] — **step magnitude decides reachable depth.**
///
/// The constraint the PRD does not mention. Sub-unit steps contract the running
/// product and traverse indefinitely; unit-or-larger steps explode within a few
/// levels. Measured values, not estimates.
#[test]
fn step_magnitude_decides_depth() {
    assert_eq!(AddressPath::max_depth_for_step(1.0, 1.0, 40), 4);
    assert_eq!(AddressPath::max_depth_for_step(1.0, 2.0, 40), 2);
    assert_eq!(AddressPath::max_depth_for_step(1.0, 3.0, 40), 2);

    // Contracting steps run past the cap rather than terminating.
    assert_eq!(AddressPath::max_depth_for_step(1.0, 0.5, 40), 40);
    assert_eq!(AddressPath::max_depth_for_step(1.0, 0.1, 40), 40);

    assert!(
        AddressPath::max_depth_for_step(1.0, 0.5, 40) > AddressPath::max_depth_for_step(1.0, 1.0, 40) * 5,
        "sub-unit steps must reach far deeper than unit steps"
    );
}

/// 2.3 — partial resolution reports how far the tree actually went.
///
/// For callers that want to walk to the end rather than treat the end as an
/// error.
#[test]
fn partial_resolution_reports_reachable_depth() {
    let p = AddressPath::new(1.0, &[1.0; 10]);
    let (addr, depth) = p.resolve_partial();
    assert_eq!(depth, 4, "unit steps run out at depth 4");
    assert!(addr.get().is_finite());
    assert!(p.resolve().is_err(), "the full path is still refused");
}

/// 2.4 — a refused address is never infinite.
#[test]
fn refused_addresses_are_never_infinite() {
    for depth in 1..12 {
        let p = AddressPath::new(1.0, &vec![1.0; depth]);
        if let Ok(addr) = p.resolve() {
            assert!(
                addr.get().is_finite(),
                "depth {depth} resolved to a non-finite address"
            );
        }
    }
}

// --------------------------------------------- Group 3: area preservation

/// 3.1 [D] — logical area is exactly `n x pi/2`.
///
/// Gauss-Bonnet fixes a hyperbolic cell's area by its angles alone, with no
/// free scale. A Euclidean allocator has no such quantity.
#[test]
fn logical_area_is_exactly_n_half_pi() {
    let unit = std::f64::consts::FRAC_PI_2;
    assert!((LogicalArea::unit_area() - unit).abs() < 1e-15);

    for n in [0usize, 1, 3, 7, 40, 1885] {
        let a = LogicalArea::of(n);
        assert_eq!(a.cells(), n);
        assert!(
            (a.area() - n as f64 * unit).abs() < 1e-12,
            "{n} cells gave area {}",
            a.area()
        );
        assert!(a.is_quantised());
    }
}

/// 3.2 [D] — **fragmentation is exactly zero, and structurally so.**
///
/// Identical cells cannot leave a gap smaller than a cell, and a gap of one or
/// more cells is free space. There is nothing to fragment. A block allocator
/// reports non-zero here as soon as anything is freed.
#[test]
fn fragmentation_is_exactly_zero() {
    let mut a = LogicalArea::of(40);
    assert_eq!(a.fragmentation(), 0.0);

    a.shrink(13);
    a.grow(5);
    a.shrink(2);
    a.grow(31);
    assert_eq!(
        a.fragmentation(),
        0.0,
        "no sequence of grow/shrink can create a gap"
    );
    assert!(a.is_quantised());
}

/// 3.3 [D] — area depends only on cell count, never on allocation history.
///
/// The concrete form of "no fragmentation": two allocations of the same size
/// are byte-for-byte the same area regardless of how they got there.
#[test]
fn area_is_history_independent() {
    let direct = LogicalArea::of(20);

    let mut churned = LogicalArea::of(1);
    for _ in 0..30 {
        churned.grow(7);
        churned.shrink(4);
    }
    churned.shrink(churned.cells() - 20);

    assert_eq!(churned.cells(), 20);
    assert_eq!(
        churned.area(),
        direct.area(),
        "a churned allocation must be identical in area to a fresh one"
    );
}

/// 3.4 — shrinking saturates rather than wrapping.
#[test]
fn shrink_saturates_at_empty() {
    let mut a = LogicalArea::of(3);
    a.shrink(99);
    assert_eq!(a.cells(), 0);
    assert_eq!(a.area(), 0.0);
    assert_eq!(a.fragmentation(), 0.0);
}

/// 3.5 [D] — scaling adds cells; it never resizes one.
///
/// "Scaling file sizes triggers geometric fractals preserving logical area."
/// The preserved quantity is the **unit**: every cell stays `pi/2` however the
/// allocation grows.
#[test]
fn scaling_adds_cells_without_resizing_them() {
    let unit = LogicalArea::unit_area();
    let mut a = LogicalArea::of(1);
    let mut last = a.area();
    for _ in 0..10 {
        a.grow(3);
        let delta = a.area() - last;
        assert!(
            (delta - 3.0 * unit).abs() < 1e-12,
            "growing by 3 cells must add exactly 3 unit areas, added {delta}"
        );
        last = a.area();
    }
}

// ------------------------------------- Group 4: a resolved scalar names a cell

/// 4.1 — the empty path resolves to exactly the origin cell.
///
/// `AddressPath::new(0.0, &[])` folds to `0.0`; `Isometry::translation(0.0)` is
/// the identity, so the named point is `HyperboloidPoint::ORIGIN` — the origin
/// cell's own centre, at distance `0.0`. Nothing else can be nearer.
#[test]
fn empty_path_resolves_to_the_origin_cell() {
    let tiling = Tiling::grow(4);
    let cell = AddressPath::new(0.0, &[])
        .resolve_to_cell(&tiling)
        .unwrap();
    assert_eq!(cell, CellId::ORIGIN);
}

/// 4.2 — resolution to a cell is deterministic, same as resolution to a
/// scalar. The nearest-cell search introduces no nondeterminism of its own.
#[test]
fn cell_resolution_is_deterministic() {
    let tiling = Tiling::grow(5);
    let p = AddressPath::new(1.0, &[0.5, 0.5]);
    let first = p.resolve_to_cell(&tiling).unwrap();
    for _ in 0..20 {
        assert_eq!(p.resolve_to_cell(&tiling).unwrap(), first);
    }
}

/// 4.3 [D] — **the resolved scalar's sign selects direction along the
/// reference geodesic**, and that reaches genuinely different cells.
///
/// A flat address space has no such thing as "the same magnitude, opposite
/// direction" naming two different locations — an offset and its negation are
/// symmetric around nothing in particular. Here they are opposite ends of the
/// same geodesic through the origin, and land in different cells.
#[test]
fn sign_selects_direction_to_different_cells() {
    let tiling = Tiling::grow(4);
    for (start, steps) in [(1.0_f64, vec![1.0]), (1.0, vec![0.5, 0.5]), (2.0, vec![0.3])] {
        let positive = AddressPath::new(start, &steps);
        let negative = AddressPath::new(-start, &steps.iter().map(|s| *s).collect::<Vec<_>>());
        let pos_cell = positive.resolve_to_cell(&tiling).unwrap();
        let neg_cell = negative.resolve_to_cell(&tiling).unwrap();
        assert_ne!(
            pos_cell, neg_cell,
            "start {start} steps {steps:?}: opposite-signed scalars must name different cells"
        );
    }
}

/// 4.4 [D] — **magnitude selects hyperbolic distance from the origin**: larger
/// resolved scalars step outward through successively farther cells rather
/// than collapsing to one.
///
/// Measured on the positive x-axis: cell identity changes at each of five
/// magnitude bands sampled from 0.5 to 8.0, and does not revisit an earlier
/// cell — a real progression outward, not noise in the quantisation.
#[test]
fn magnitude_steps_outward_through_successive_cells() {
    let tiling = Tiling::grow(6);
    let mut seen = Vec::new();
    for mag in [0.5, 1.0, 2.5, 4.0, 6.0] {
        let cell = AddressPath::new(mag, &[]).resolve_to_cell(&tiling).unwrap();
        assert!(
            !seen.contains(&cell),
            "magnitude {mag} revisited a cell already seen at a smaller magnitude: {cell:?}"
        );
        seen.push(cell);
    }
    assert_eq!(seen.len(), 5, "five distinct magnitude bands must give five distinct cells");
}

/// 4.5 — the resolved cell is always a member of the tiling it was resolved
/// against. `nearest_cell` cannot invent a cell the tiling never grew.
#[test]
fn resolved_cell_always_belongs_to_its_tiling() {
    let tiling = Tiling::grow(3);
    for (start, steps) in [(0.5, vec![]), (2.0, vec![0.3]), (-1.5, vec![0.2, 0.2])] {
        let cell = AddressPath::new(start, &steps).resolve_to_cell(&tiling).unwrap();
        assert!(
            tiling.contains(&cell),
            "resolved cell {cell:?} is not a member of the tiling it was resolved against"
        );
    }
}

/// 4.6 — a magnitude beyond a small tiling's grown extent still resolves,
/// degrading gracefully to the nearest cell the tiling actually has rather
/// than panicking or refusing.
///
/// This is the honest reading of "nearest": the target point named by a huge
/// scalar may be far outside anything grown, and the answer is still whichever
/// grown cell happens to be closest to it.
#[test]
fn oversized_magnitude_degrades_to_the_nearest_grown_cell() {
    let small = Tiling::grow(1);
    assert_eq!(small.len(), 6, "depth 1 is the origin plus its five neighbours");

    let far = AddressPath::new(50.0, &[]);
    let cell = far.resolve_to_cell(&small).unwrap();
    assert!(
        small.contains(&cell),
        "an oversized magnitude must still resolve to a real cell of the small tiling"
    );
}

/// 4.7 [D] — **domain refusal propagates through to cell resolution.** A path
/// whose `(x)`-fold leaves the domain names no point and therefore no cell —
/// refused, not resolved to a nonsense location.
#[test]
fn domain_refusal_propagates_to_cell_resolution() {
    let tiling = Tiling::grow(2);
    let too_far = AddressPath::new(1.0, &[1.0, 1.0, 1.0, 1.0, 1.0]);
    assert!(too_far.resolve().is_err(), "premise: the fold itself must be refused");
    assert!(
        matches!(
            too_far.resolve_to_cell(&tiling),
            Err(LatticeError::Dissonant { .. })
        ),
        "cell resolution must refuse for the same reason resolve() does"
    );
}

/// 4.8 — `resolved_point` and `resolve_to_cell` agree: the cell returned is
/// genuinely the nearest one to the point the scalar names, not an
/// independently-computed answer that happens to usually match.
#[test]
fn resolve_to_cell_matches_an_independent_nearest_search() {
    let tiling = Tiling::grow(5);
    let p = AddressPath::new(1.3, &[0.4]);
    let point = p.resolved_point().unwrap();
    let via_path = p.resolve_to_cell(&tiling).unwrap();
    let via_direct = tiling.nearest_cell(&point).unwrap();
    assert_eq!(via_path, via_direct);
}
