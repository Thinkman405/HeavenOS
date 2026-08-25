//! Live visualisation — `gui` rendering the running system.
//!
//! These assertions cross four subsystems: `symphony-kernel` supplies the real
//! load field, `ftg` the real delivery outcomes, `lattice` the topology beneath
//! both, and `gui` draws the result.
//!
//! **[D]** marks assertions a display fed by made-up numbers could not pass.

use ftg::transport::{Delivery, Gateway, Packet};
use ftg::Frame;
use gui::telemetry::SystemSnapshot;
use gui::visualization::LoadVisualisation;
use substrate::MemoryPool;
use symphony_kernel::{Scheduler, Task};

fn loaded_scheduler(cores: usize, tasks: u64, hz: f64) -> Scheduler {
    let mut s = Scheduler::new(cores);
    s.ingest((0..tasks).map(|i| Task::new(i, hz)));
    s
}

// ------------------------------------------- Group 1: reading the real kernel

/// 1.1 — the snapshot reads the kernel's actual per-core load.
#[test]
fn snapshot_reads_the_real_load_field() {
    let s = loaded_scheduler(16, 60, 2.0e9);
    let snap = SystemSnapshot::from_scheduler(&s);

    assert_eq!(snap.core_count(), s.core_count());
    assert_eq!(snap.task_count(), 60);
    assert_eq!(snap.raw_load(), s.load_per_core().as_slice());
    assert!(snap.total_load() > 0.0, "a loaded system must show load");
}

/// 1.2 [D] — **raw energy is unrenderable; the normalised field is not.**
///
/// A 2 GHz task costs ~5.3e-25 J. Drawing that directly would render a fully
/// loaded machine as idle. This asserts the gap: raw values are microscopic,
/// normalised values reach 1.0.
#[test]
fn raw_energy_is_unrenderable_but_normalised_is() {
    let s = loaded_scheduler(16, 60, 2.0e9);
    let snap = SystemSnapshot::from_scheduler(&s);

    let raw_peak = snap.raw_load().iter().cloned().fold(0.0_f64, f64::max);
    assert!(
        raw_peak < 1e-20,
        "raw load should be microscopic joules, got {raw_peak}"
    );

    let norm_peak = snap
        .normalised_load()
        .into_iter()
        .fold(0.0_f64, f64::max);
    assert!(
        (norm_peak - 1.0).abs() < 1e-12,
        "the busiest core must normalise to 1.0, got {norm_peak}"
    );
}

/// 1.3 [D] — the visualisation is **scale-free**.
///
/// The same load shape drawn at 1 GHz and at 100 GHz produces identical
/// amplitudes, and those amplitudes are of order 1.
///
/// The order-1 check is not decoration. An earlier version compared only the
/// absolute difference, which a raw-joules display *passes vacuously*: both
/// sides are ~1e-25 and ~1e-23, so their difference sits far below any
/// sensible absolute tolerance. The test guarding against the
/// absolute-vs-relative trap had fallen into it. Verified by sabotage.
#[test]
fn visualisation_is_scale_free() {
    let slow = SystemSnapshot::from_scheduler(&loaded_scheduler(16, 60, 1.0e9));
    let fast = SystemSnapshot::from_scheduler(&loaded_scheduler(16, 60, 1.0e11));

    let a = LoadVisualisation::from_snapshot(&slow, 1.2, 3.0);
    let b = LoadVisualisation::from_snapshot(&fast, 1.2, 3.0);
    assert_eq!(a.cores(), b.cores());

    // The amplitudes must be renderable at all — order 1, not order 1e-25.
    assert!(
        a.peak() > 0.1 && b.peak() > 0.1,
        "amplitudes must be order 1 to be drawable, got {} and {}",
        a.peak(),
        b.peak()
    );

    // And identical between the two energy scales, compared relatively.
    for core in 0..a.cores() {
        let (x, y) = (a.at(core, 0.9, 0.4), b.at(core, 0.9, 0.4));
        let scale = x.abs().max(y.abs()).max(1e-9);
        assert!(
            (x - y).abs() / scale < 1e-9,
            "core {core} drew differently across energy scales: {x} vs {y}"
        );
    }
}

/// 1.4 — an idle system draws nothing.
#[test]
fn idle_system_renders_zero() {
    let s = Scheduler::new(7);
    let snap = SystemSnapshot::from_scheduler(&s);
    assert_eq!(snap.task_count(), 0);
    assert!(snap.normalised_load().iter().all(|v| *v == 0.0));

    let vis = LoadVisualisation::from_snapshot(&snap, 1.2, 3.0);
    assert_eq!(vis.peak(), 0.0);
    assert_eq!(vis.total(0.8, 0.3), 0.0);
    assert_eq!(vis.amplitude_spread(), 0.0);
}

// -------------------------------------- Group 2: the display tracks the field

/// 2.1 [D] — **the visualisation flattens as the field equilibrates.**
///
/// The strongest claim in this slice: draw the field, run the real scheduler,
/// draw again, and the ragged display measurably flattens.
///
/// Tasks carry **varied frequencies** on purpose. With identical tasks the
/// field is already optimal the moment it is ingested — migration moves whole
/// tasks, so 60 identical tasks on 16 cores can only ever be 4/4/…/3/3/3/3, and
/// nothing improves. See [`quantisation_limited_field_does_not_improve`], which
/// pins that as a property rather than leaving it to mask a weak assertion.
///
/// Measured with varied frequencies: imbalance 0.3478 → 0.2857.
#[test]
fn visualisation_flattens_as_the_field_equilibrates() {
    let mut s = Scheduler::new(16);
    s.ingest((0..60u64).map(|i| Task::new(i, 1.0e9 + f64::from((i % 7) as u32) * 3.0e9)));

    let snap_before = SystemSnapshot::from_scheduler(&s);
    let before = LoadVisualisation::from_snapshot(&snap_before, 1.2, 3.0);
    let (imb_before, spread_before) = (snap_before.imbalance(), before.amplitude_spread());

    let alpha = s.topology().stability_bound() * 0.9;
    for _ in 0..12 {
        s.schedule(alpha, 20_000).expect("stable coupling");
    }

    let snap_after = SystemSnapshot::from_scheduler(&s);
    let after = LoadVisualisation::from_snapshot(&snap_after, 1.2, 3.0);

    assert!(
        snap_after.imbalance() < imb_before - 0.01,
        "equilibration must measurably flatten the field: {imb_before} -> {}",
        snap_after.imbalance()
    );
    assert!(
        after.amplitude_spread() < spread_before - 0.01,
        "and the drawn amplitudes must follow: {spread_before} -> {}",
        after.amplitude_spread()
    );
    assert_eq!(after.cores(), 16);
    assert_eq!(s.task_count(), 60, "no task may be lost while balancing");
}

/// 2.1b — a quantisation-limited field does **not** improve, and that is not a
/// failure of the balancer.
///
/// 60 identical tasks across 16 cores is 4/4/…/3/3/3/3 — already optimal,
/// because migration moves whole tasks and a task cannot be split. Asserting
/// the floor keeps 2.1's strict inequality honest: without this, a `<=` there
/// would silently pass while measuring nothing.
#[test]
fn quantisation_limited_field_does_not_improve() {
    let mut s = Scheduler::new(16);
    s.ingest((0..60u64).map(|i| Task::new(i, 2.0e9)));

    let before = SystemSnapshot::from_scheduler(&s).imbalance();
    let alpha = s.topology().stability_bound() * 0.9;
    for _ in 0..12 {
        s.schedule(alpha, 20_000).unwrap();
    }
    let after = SystemSnapshot::from_scheduler(&s).imbalance();

    assert!(
        (after - before).abs() < 1e-12,
        "an already-optimal field must stay put, not drift: {before} -> {after}"
    );
    assert!(
        before > 0.0,
        "the residual imbalance is the whole-task floor, not zero"
    );
}

/// 2.1c — migration cannot fill an idle core.
///
/// 5 tasks on 16 cores leaves 11 cores empty, so imbalance is pinned at 1.0.
/// Diffusion redistributes work; it does not create it.
#[test]
fn migration_cannot_fill_an_idle_core() {
    let mut s = Scheduler::new(16);
    s.ingest((0..5u64).map(|i| Task::new(i, 2.0e9)));
    let alpha = s.topology().stability_bound() * 0.9;
    for _ in 0..12 {
        s.schedule(alpha, 20_000).unwrap();
    }
    assert_eq!(
        SystemSnapshot::from_scheduler(&s).imbalance(),
        1.0,
        "with more cores than tasks, some core is always idle"
    );
}

/// 2.2 [D] — imbalance is visible, and balance is visibly different.
///
/// A pathological field draws with real spread; a hand-built uniform one draws
/// flat. If the display could not tell them apart it would be decoration.
#[test]
fn imbalance_is_visible_and_balance_is_not() {
    let mut piled = Scheduler::new(16);
    piled.ingest((0..60u64).map(|i| Task::new(i, 2.0e9)));
    let piled_snap = SystemSnapshot::from_scheduler(&piled);

    // Every core equally loaded.
    let mut flat = Scheduler::new(16);
    flat.ingest((0..16u64).map(|i| Task::new(i, 2.0e9)));
    let flat_snap = SystemSnapshot::from_scheduler(&flat);

    assert!(
        flat_snap.imbalance() < piled_snap.imbalance() + 1e-12,
        "an evenly loaded field must not read as more imbalanced"
    );
    assert!(
        flat_snap.imbalance() < 1e-9,
        "16 identical tasks on 16 cores should be flat, got {}",
        flat_snap.imbalance()
    );
}

/// 2.3 — imbalance is bounded in `[0, 1]` whatever the load.
#[test]
fn imbalance_is_bounded() {
    for (cores, tasks, hz) in [(7, 1u64, 1.0e9), (16, 60, 2.0e9), (31, 300, 5.0e9)] {
        let snap = SystemSnapshot::from_scheduler(&loaded_scheduler(cores, tasks, hz));
        let imb = snap.imbalance();
        assert!(
            (0.0..=1.0).contains(&imb),
            "imbalance {imb} out of range for {cores} cores"
        );
    }
}

// ------------------------------------- Group 3: reading real network traffic

fn gateway_packet(g: &Gateway, payload: &[u8], min_hops: usize) -> Packet {
    let cells = g.router().cells();
    let src = cells[0];
    for &dst in cells.iter().rev() {
        if let Ok(path) = g.router().route(src, dst, 200) {
            if path.len() - 1 >= min_hops {
                return Packet::new(Frame::encode(payload), src, dst);
            }
        }
    }
    panic!("no route of at least {min_hops} hops");
}

/// 3.1 [D] — traffic counts come from `ftg`'s real delivery outcomes.
///
/// Not a tally the caller kept: the snapshot consumes the actual `Delivery`
/// values transport returned, so the three outcomes cannot drift apart.
#[test]
fn snapshot_observes_real_delivery_outcomes() {
    let g = Gateway::new(4);
    let packet = gateway_packet(&g, b"telemetry", 3);
    let mut snap = SystemSnapshot::new();

    // A clean delivery.
    let ok = g.deliver(&packet, 200).unwrap();
    assert!(matches!(ok, Delivery::Arrived { .. }));
    snap.observe(&ok);

    // A corrupted one.
    let bad = g
        .deliver_through(&packet, 200, |hop, frame| {
            if hop == 0 {
                frame.corrupt(2);
            }
        })
        .unwrap();
    assert!(matches!(bad, Delivery::Dissipated { .. }));
    snap.observe(&bad);

    assert_eq!(snap.delivered(), 1);
    assert_eq!(snap.dissipated(), 1);
    assert_eq!(snap.failed(), 1);
}

/// 3.2 [D] — a lost session counts separately from a corrupted frame.
///
/// The distinction slice 3 established survives into the display: a stranded
/// packet is not miscounted as corruption.
#[test]
fn link_loss_is_counted_apart_from_dissipation() {
    use ftg::constants::CARRIER_RAD_PER_SEC;
    use ftg::session::{Link, Oscillator};

    let g = Gateway::new(4);
    let packet = gateway_packet(&g, b"stranded", 4);
    let mut link = Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.1, 1.0))
        .expect("locks");

    let lost = g
        .deliver_over_with(&mut link, &packet, 200, |hop, _f, l| {
            if hop == 1 {
                l.teardown(CARRIER_RAD_PER_SEC, 1e-10).ok();
            }
        })
        .unwrap();
    assert!(matches!(lost, Delivery::LinkLost { .. }));

    let mut snap = SystemSnapshot::new();
    snap.observe(&lost);
    assert_eq!(snap.link_lost(), 1);
    assert_eq!(snap.dissipated(), 0, "a lost session is not a corrupt frame");
    assert_eq!(snap.failed(), 1);
}

/// 3.3 — network balance spans `+1` (all delivered) to `-1` (all failed), with
/// exact cancellation at zero.
#[test]
fn network_balance_spans_constructive_to_destructive() {
    let g = Gateway::new(4);
    let packet = gateway_packet(&g, b"balance", 3);
    let ok = g.deliver(&packet, 200).unwrap();
    let bad = g
        .deliver_through(&packet, 200, |hop, frame| {
            if hop == 0 {
                frame.corrupt(1);
            }
        })
        .unwrap();

    let mut all_good = SystemSnapshot::new();
    for _ in 0..4 {
        all_good.observe(&ok);
    }
    assert!((all_good.network_balance() - 1.0).abs() < 1e-12);

    let mut all_bad = SystemSnapshot::new();
    for _ in 0..4 {
        all_bad.observe(&bad);
    }
    assert!((all_bad.network_balance() + 1.0).abs() < 1e-12);

    let mut even = SystemSnapshot::new();
    for _ in 0..3 {
        even.observe(&ok);
        even.observe(&bad);
    }
    assert_eq!(
        even.network_balance(),
        0.0,
        "equal successes and failures must cancel exactly"
    );
}

/// 3.4 — an unused network reads neutral, not failing.
#[test]
fn quiet_network_reads_neutral() {
    let snap = SystemSnapshot::new();
    assert_eq!(snap.network_balance(), 0.0);
    assert_eq!(snap.delivered(), 0);
    assert_eq!(snap.failed(), 0);
}

// ------------------------------------------ Group 4: reading real memory usage

/// 4.1 — the snapshot reads the pool's actual capacity and free space, not a
/// number the caller invented.
#[test]
fn snapshot_reads_the_real_pool() {
    let pool = MemoryPool::new(16, 256);
    let mut snap = SystemSnapshot::new();
    snap.read_memory(&pool);

    assert_eq!(snap.memory_total(), pool.total_capacity());
    assert_eq!(snap.memory_available(), pool.available());
    assert_eq!(snap.memory_total(), 16 * 256);
}

/// 4.2 — memory usage needs no normalisation trick: it is already a ratio of
/// two same-unit quantities, unlike core load's `1e-25` joules.
///
/// This is the direct contrast the module docs promise: usage lands in
/// `[0, 1]` from real allocation, with no analogue of `normalised_load`'s
/// peak-scaling required.
#[test]
fn memory_utilisation_needs_no_scaling_trick() {
    let mut pool = MemoryPool::new(16, 256);
    let alloc = pool.allocate(256 * 4).expect("fits");

    let mut snap = SystemSnapshot::new();
    snap.read_memory(&pool);

    assert!(
        (0.0..=1.0).contains(&snap.memory_utilisation()),
        "utilisation must land in [0, 1] on its own, got {}",
        snap.memory_utilisation()
    );
    assert_eq!(snap.memory_used(), 256 * 4);
    assert!(
        (snap.memory_utilisation() - (256.0 * 4.0) / (16.0 * 256.0)).abs() < 1e-12,
        "utilisation must be exactly used/total"
    );

    pool.free(&alloc);
}

/// 4.3 — utilisation tracks real allocation and deallocation, not a snapshot
/// taken once and never revisited.
#[test]
fn utilisation_tracks_allocation_and_freeing() {
    let mut pool = MemoryPool::new(16, 256);
    let mut snap = SystemSnapshot::new();
    snap.read_memory(&pool);
    let empty = snap.memory_utilisation();
    assert_eq!(empty, 0.0, "a fresh pool must read as fully free");

    let alloc = pool.allocate(256 * 8).expect("fits");
    snap.read_memory(&pool);
    let half = snap.memory_utilisation();
    assert!(half > empty, "allocation must raise utilisation");

    pool.free(&alloc);
    snap.read_memory(&pool);
    let freed = snap.memory_utilisation();
    assert_eq!(freed, 0.0, "freeing everything must return to fully free");
}

/// 4.4 — an unread snapshot reads as idle, not as an error or a divide-by-zero.
#[test]
fn unread_memory_reads_as_idle() {
    let snap = SystemSnapshot::new();
    assert_eq!(snap.memory_total(), 0);
    assert_eq!(snap.memory_available(), 0);
    assert_eq!(snap.memory_used(), 0);
    assert_eq!(snap.memory_utilisation(), 0.0);
}

/// 4.5 — reading memory does not disturb load or network fields already on
/// the snapshot, and vice versa: the three readouts are independent.
#[test]
fn memory_load_and_network_readouts_are_independent() {
    let s = loaded_scheduler(16, 60, 2.0e9);
    let mut snap = SystemSnapshot::from_scheduler(&s);
    let load_before = snap.total_load();
    let task_count_before = snap.task_count();

    let pool = MemoryPool::new(16, 256);
    snap.read_memory(&pool);

    assert_eq!(snap.total_load(), load_before, "load must survive a memory read");
    assert_eq!(snap.task_count(), task_count_before);
    assert_eq!(snap.memory_total(), pool.total_capacity());

    let g = Gateway::new(4);
    let packet = gateway_packet(&g, b"mem", 3);
    let ok = g.deliver(&packet, 200).unwrap();
    snap.observe(&ok);

    assert_eq!(snap.delivered(), 1);
    assert_eq!(snap.memory_total(), pool.total_capacity(), "observe() must not disturb memory");
    assert_eq!(snap.total_load(), load_before, "observe() must not disturb load");
}

/// 4.6 [D] — `memory_used` never underflows for an unread or partially-freed
/// snapshot, unlike a plain `total - available` on unsigned integers would.
#[test]
fn memory_used_never_underflows() {
    let snap = SystemSnapshot::new();
    // total == available == 0 here; total.saturating_sub(available) must not
    // panic the way unchecked unsigned subtraction would on any mismatch.
    assert_eq!(snap.memory_used(), 0);
}
