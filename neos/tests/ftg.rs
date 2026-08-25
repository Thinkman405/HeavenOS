//! Physics assertions for the Fourier Transform Gateway.
//!
//! Doctrine: `_mkb/test-doctrine.md`. Plan:
//! `subsystems/ftg/03_tests/output/test-plan.md`.
//!
//! **[D]** marks assertions a conventional networking stack could not pass -
//! one that is CRC-checked, table-routed, and message-terminated.

use ftg::constants::*;
use ftg::layers_3_4::{channel_overlap, NetAddress, Port, Router};
use ftg::session::{cancellation_floor, superpose, Oscillator};
use ftg::{Frame, FtgError, Link, LinkState};
use lattice::tessellation::CellId;

// --------------------------------------------------- Group 1: Layer 1/2

/// 1.1 - frames round trip over varied payloads.
#[test]
fn frame_round_trip_is_lossless() {
    for payload in [
        vec![0x00u8],
        vec![0xFF],
        vec![0xAA, 0x55],
        b"NEOS gateway".to_vec(),
        (0..=255u8).collect::<Vec<_>>(),
    ] {
        let f = Frame::encode(&payload);
        assert_eq!(f.decode().unwrap(), payload);
    }
}

/// 1.2 [D] - a clean frame cancels to **exactly** zero.
///
/// Exact by construction: `sin(+-pi/2)` is exactly `+-1` in IEEE-754, and each
/// payload symbol is cancelled by its complement.
#[test]
fn clean_frame_dissonance_is_exactly_zero() {
    for payload in [vec![0x00u8], vec![0xFF], b"resonance".to_vec()] {
        let f = Frame::encode(&payload);
        assert_eq!(
            f.dissonance(),
            0.0,
            "a clean frame must cancel exactly, got {}",
            f.dissonance()
        );
        assert!(f.is_clean());
    }
}

/// 1.3 [D] - **every** single-symbol flip gives dissonance exactly 2.0.
///
/// Not a sample: every position is checked. A CRC yields a hash mismatch; this
/// yields an amplitude, which is the whole difference.
#[test]
fn any_single_flip_gives_dissonance_two() {
    let payload = b"interference";
    let base = Frame::encode(payload);
    for i in 0..base.phases().len() {
        let mut f = base.clone();
        f.corrupt(i);
        assert_eq!(
            f.dissonance(),
            2.0,
            "flip at symbol {i} should give dissonance 2.0, got {}",
            f.dissonance()
        );
    }
}

/// 1.4 [D] - a dissonant frame cannot be decoded. There is no lossy path.
#[test]
fn dissonant_frame_is_refused_not_repaired() {
    let mut f = Frame::encode(b"corrupt me");
    f.corrupt(3);
    assert!(!f.is_clean());
    assert!(matches!(f.decode(), Err(FtgError::Dissonant { .. })));
}

/// 1.5 - **the blind spot is real, and asserted so it stays visible.**
///
/// Flipping a symbol *and its complement partner* cancels: dissonance returns
/// to zero and the frame decodes. Interference checking measures net amplitude,
/// so it cannot separate "no error" from "errors that cancel".
///
/// This test exists to keep the limitation documented. If it ever fails, the
/// scheme changed and the claim in the contract must change with it.
#[test]
fn correlated_flip_of_a_symbol_and_its_partner_is_undetected() {
    let payload = b"blind";
    let mut f = Frame::encode(payload);
    let half = f.payload_bits();
    f.corrupt(0);
    f.corrupt(half); // the complement of symbol 0

    assert_eq!(f.dissonance(), 0.0, "correlated flips cancel");
    assert!(f.is_clean(), "and therefore report clean");
    assert_ne!(
        f.decode().unwrap(),
        payload.to_vec(),
        "yet the payload is genuinely corrupted - this is the documented limit"
    );
}

/// 1.6 - the complement structure doubles the symbol count.
#[test]
fn frame_length_is_twice_the_payload() {
    let f = Frame::encode(b"ab");
    assert_eq!(f.payload_bits(), 16);
    assert_eq!(f.phases().len(), 32);
}

// --------------------------------------------------- Group 2: routing

fn router() -> Router {
    Router::new(5)
}

/// 2.1 / 2.2 - the address map is deterministic and total.
#[test]
fn address_mapping_is_deterministic_and_total() {
    let r = router();
    for i in 0..1000u128 {
        let a = NetAddress(i * 2_654_435_761);
        let first = r.cell_for(a);
        assert_eq!(r.cell_for(a), first, "mapping must be stable");
        assert!(r.contains(&first), "every address must map in-patch");
    }
}

/// 2.3 [D] - greedy descent always arrives.
///
/// Greedy geometric routing gets stuck at local minima on general graphs. On
/// this tiling it never does.
#[test]
fn greedy_descent_always_arrives() {
    let r = router();
    let cells = r.cells();
    let mut routed = 0;
    for i in (0..cells.len()).step_by(7) {
        for j in (0..cells.len()).step_by(11) {
            let path = r
                .route(cells[i], cells[j], 200)
                .unwrap_or_else(|e| panic!("routing {i}->{j} failed: {e}"));
            assert_eq!(*path.first().unwrap(), cells[i]);
            assert_eq!(*path.last().unwrap(), cells[j]);
            routed += 1;
        }
    }
    assert!(routed > 2000, "expected a meaningful sample, routed {routed}");
}

/// 2.4 [D] - **greedy descent is BFS-optimal.**
///
/// The sharper claim, asserted separately from 2.3 on purpose: if routing ever
/// degrades to merely "arrives", this fails while 2.3 still passes.
#[test]
fn greedy_descent_is_shortest_path() {
    let r = router();
    let cells = r.cells();
    let mut checked = 0;
    for i in (0..cells.len()).step_by(23) {
        for j in (0..cells.len()).step_by(29) {
            let path = r.route(cells[i], cells[j], 200).expect("routes");
            let optimal = r.bfs_hops(cells[i], cells[j]).expect("connected");
            assert_eq!(
                path.len() - 1,
                optimal,
                "greedy took {} hops, BFS-optimal is {optimal}",
                path.len() - 1
            );
            checked += 1;
        }
    }
    assert!(checked > 200, "expected a meaningful sample, checked {checked}");
}

/// 2.4a - `ftg`'s own `bfs_hops` (test-only, never called by routing itself)
/// agrees exactly with `lattice::shortest_distance` — a general-purpose,
/// independently-built path-finder that lives in `lattice` rather than
/// being duplicated per consumer. Two separate BFS implementations, over
/// the same tiling, cross-checked against each other rather than either
/// being trusted alone.
#[test]
fn lattice_shortest_distance_agrees_with_ftgs_own_bfs() {
    let r = router();
    let tiling = lattice::Tiling::grow(5); // same depth `router()` builds internally
    let cells = r.cells();
    let mut checked = 0;
    for i in (0..cells.len()).step_by(23) {
        for j in (0..cells.len()).step_by(29) {
            let expected = r.bfs_hops(cells[i], cells[j]).expect("connected");
            let actual =
                lattice::shortest_distance(&tiling, cells[i], cells[j]).expect("connected");
            assert_eq!(
                actual, expected,
                "{i}->{j}: lattice said {actual}, ftg's own bfs_hops said {expected}"
            );
            checked += 1;
        }
    }
    assert!(checked > 200, "expected a meaningful sample, checked {checked}");
}

/// 2.4b [D] - **any** strict descent is optimal, not just the greedy pick.
///
/// The stronger invariant, and the one that survives sabotage. Choosing the
/// *closest* neighbour turned out not to be load-bearing: 42% of steps have
/// more than one descending option, yet deliberately taking the worst still
/// yields a shortest path. What matters is that the step strictly descends.
///
/// This also records why `next_hop`'s `min_by` exists - determinism, not
/// correctness - so nobody "optimises" it away thinking it is decorative, nor
/// defends it as load-bearing when it is not.
#[test]
fn any_strict_descent_is_also_optimal() {
    let r = router();
    let cells = r.cells();
    let mut with_choice = 0;
    let mut steps = 0;
    let mut checked = 0;

    for i in (0..cells.len()).step_by(37) {
        for j in (0..cells.len()).step_by(41) {
            let (src, dst) = (cells[i], cells[j]);
            if src == dst {
                continue;
            }
            let mut cur = src;
            let mut hops = 0;
            while cur != dst && hops < 200 {
                let (next, options) = r.any_descent_hop(cur, dst).expect("descent exists");
                if options > 1 {
                    with_choice += 1;
                }
                steps += 1;
                cur = next;
                hops += 1;
            }
            let optimal = r.bfs_hops(src, dst).expect("connected");
            assert_eq!(
                hops, optimal,
                "worst-choice descent took {hops} hops, BFS-optimal is {optimal}"
            );
            checked += 1;
        }
    }
    assert!(checked > 100, "expected a meaningful sample, checked {checked}");

    // Guard against the test proving nothing: if descent were always forced,
    // "worst choice" and "greedy choice" would be the same walk.
    //
    // Measured ~16% along the worst-choice path. Note this is lower than the
    // ~42% seen along the greedy path - the two take different trajectories, so
    // they meet different branching. The threshold is set from the walk this
    // test actually performs, not the other one.
    assert!(
        with_choice * 10 > steps,
        "expected genuine branching, saw {with_choice} of {steps} steps with a \
         choice - below 10% this test would prove little"
    );
}

/// 2.5 - a route only ever crosses real edges.
#[test]
fn every_hop_crosses_an_edge() {
    let r = router();
    let cells = r.cells();
    let path = r.route(cells[0], cells[cells.len() - 1], 200).unwrap();
    for w in path.windows(2) {
        assert!(
            r.adjacent(&w[0], &w[1]),
            "route jumped between non-adjacent cells"
        );
    }
}

/// 2.6 - each hop strictly decreases distance. That is what descent means.
#[test]
fn each_hop_strictly_descends() {
    let r = router();
    let cells = r.cells();
    let dst = cells[cells.len() - 1];
    let path = r.route(cells[3], dst, 200).unwrap();
    for w in path.windows(2) {
        assert!(
            r.distance(&w[1], &dst) < r.distance(&w[0], &dst),
            "hop did not descend toward the destination"
        );
    }
}

/// 2.6b [D] — **the true diameter of the documented 441-cell patch is small**
/// — measured, not assumed, before building multi-hop transport tests on top
/// of it.
///
/// Every existing wave-level transport test (`ftg_transport.rs`,
/// `ftg_session_transport.rs`) uses routes of only 3-4 hops, even though this
/// same 441-cell patch — already exercised by `greedy_descent_is_shortest_path`
/// above — reaches up to **10 hops** between its farthest cells. That gap
/// between what routing is tested at and what transport is tested at is
/// closed in `ftg_transport.rs`/`ftg_session_transport.rs`; this test is the
/// measurement that justifies the hop count used there, kept next to the
/// routing tests it depends on rather than duplicated.
///
/// Hyperbolic growth is why the diameter stays this small: ring `n` holds
/// `5*Fib(2n)` cells, so 441 cells are reached by ring 5 while the *distance*
/// across them grows only logarithmically. This is the payoff of the
/// embedding the module docs describe, now measured directly rather than
/// asserted in prose.
#[test]
fn diameter_of_the_documented_patch_is_measured() {
    let r = router();
    let (_, _, diameter) = farthest_pair(&r);
    assert!(
        (1..=15).contains(&diameter),
        "diameter {diameter} is outside the range every hard-coded multi-hop \
         test in this workspace assumes — update MULTI_HOP_COUNT everywhere \
         it is used if this ever changes"
    );
}

/// Find a pair near the patch's diameter by sampling, and how many hops BFS
/// gives between them.
///
/// Not the exact graph diameter (that would need all-pairs BFS) — a wide,
/// deterministic sample that reliably lands within one hop of it. Shared with
/// `ftg_transport.rs`/`ftg_session_transport.rs` conceptually, though each
/// test file keeps its own copy rather than pulling in a shared test module
/// for three lines of code.
pub fn farthest_pair(r: &Router) -> (CellId, CellId, usize) {
    let cells = r.cells();
    let (mut best_a, mut best_b, mut best_hops) = (cells[0], cells[0], 0);
    for (i, &a) in cells.iter().enumerate().step_by(7) {
        for &b in cells.iter().skip(i + 1).step_by(11) {
            if let Some(h) = r.bfs_hops(a, b) {
                if h > best_hops {
                    (best_a, best_b, best_hops) = (a, b, h);
                }
            }
        }
    }
    (best_a, best_b, best_hops)
}

/// 2.7 - routing to self is trivial.
#[test]
fn route_to_self_is_a_single_cell() {
    let r = router();
    let c = r.cells()[9];
    assert_eq!(r.route(c, c, 200).unwrap(), vec![c]);
}

/// 2.8 - no descent is reported, never looped on.
#[test]
fn stranded_packet_reports_no_descent() {
    let small = Router::new(1);
    let large = Router::new(4);
    let outside = large
        .cells()
        .iter()
        .find(|c| !small.contains(c))
        .copied()
        .expect("a deeper tiling has cells the shallow one lacks");
    assert!(matches!(
        small.route(small.cells()[0], outside, 50),
        Err(FtgError::NoDescent { .. })
    ));
}

/// 2.9 - the hop limit is a real guard.
#[test]
fn hop_limit_is_enforced() {
    let r = router();
    let cells = r.cells();
    assert!(matches!(
        r.route(cells[0], cells[cells.len() - 1], 1),
        Err(FtgError::HopLimit { limit: 1 })
    ));
}

// --------------------------------------------------- Group 3: multiplexing

/// 3.1 - port `n` rides the `(n+1)`-th harmonic.
#[test]
fn port_overtones_are_harmonics_of_the_carrier() {
    for n in [0u16, 1, 2, 7, 443] {
        let expected = CARRIER_RAD_PER_SEC * f64::from(u32::from(n) + 1);
        let got = Port(n).overtone().get();
        assert!(
            ((got - expected) / expected).abs() < 1e-9,
            "port {n}: got {got}, expected {expected}"
        );
    }
}

/// 3.2 [D] - distinct ports are orthogonal, so they do not interfere.
#[test]
fn distinct_ports_are_orthogonal() {
    for (a, b) in [(0u16, 1u16), (0, 2), (1, 3), (2, 7), (5, 11)] {
        let overlap = channel_overlap(Port(a), Port(b), 200_000);
        assert!(
            overlap.abs() < 1e-12,
            "ports {a} and {b} overlap by {overlap}; channels must not interfere"
        );
    }
}

/// 3.3 - self-overlap is 0.5, confirming the integrator works.
///
/// Without this, 3.2's zero could just mean a broken sum.
#[test]
fn self_overlap_is_one_half() {
    let s = channel_overlap(Port(1), Port(1), 200_000);
    assert!((s - 0.5).abs() < 1e-6, "expected 0.5, got {s}");
}

// --------------------------------------------------- Group 4: session

/// 4.1 - aligned oscillators lock.
#[test]
fn aligned_oscillators_resonate() {
    let link = Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.1, 1.0))
        .expect("variance 0.1 is well inside the bound");
    assert!(link.is_resonant());
    assert!(link.still_locked());
    assert!(matches!(link.state(), LinkState::Resonant { .. }));
}

/// 4.2 - the lock bound is strict, and exact at the boundary.
#[test]
fn lock_bound_is_strictly_exclusive() {
    let below = Link::attempt_handshake(
        Oscillator::new(0.0, 1.0),
        Oscillator::new(LINK_LOCK_BOUND - 1e-9, 1.0),
    );
    assert!(below.is_ok(), "just below the bound must lock");

    for v in [LINK_LOCK_BOUND, LINK_LOCK_BOUND + 1e-9, 1.0] {
        assert!(
            matches!(
                Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(v, 1.0)),
                Err(FtgError::NoLock { .. })
            ),
            "variance {v} must refuse to lock"
        );
    }
}

/// 4.3 - the standing wave matches `2A sin(kx) cos(wt)`.
#[test]
fn standing_wave_matches_closed_form() {
    let link =
        Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.0, 1.0)).unwrap();
    let (k, x, w, t) = (2.0_f64, 0.7_f64, 3.0_f64, 0.4_f64);
    let expected = 2.0 * (k * x).sin() * (w * t).cos();
    assert!((link.standing_wave(k, x, w, t) - expected).abs() < 1e-12);
}

/// 4.4 - a collapsed link is terminal.
#[test]
fn collapsed_link_cannot_be_reused() {
    let mut link =
        Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.05, 1.0)).unwrap();
    link.teardown(CARRIER_RAD_PER_SEC, 1e-10).unwrap();
    assert_eq!(link.state(), LinkState::Collapsed);
    assert!(!link.is_resonant());
    assert!(matches!(
        link.teardown(CARRIER_RAD_PER_SEC, 1e-10),
        Err(FtgError::Collapsed)
    ));
}

/// 4.5 — **automatic teardown on drift, at the `Link` level.**
///
/// `equations.md`'s Standing Wave Superposition rule: *"Any phase variance
/// exceeding `+-pi/4` triggers automatic phase inversion and teardown."*
/// `still_locked`/`drift_to` deliberately never enforce this — detection
/// must stay pure measurement. `enforce_lock` is the policy that finally
/// does: a link discovered drifted past the bound is not merely *reported*
/// lost, it is actually collapsed by the call that discovers it.
#[test]
fn enforce_lock_collapses_a_drifted_link() {
    let mut link =
        Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.1, 1.0)).unwrap();
    link.drift_to(LINK_LOCK_BOUND + 0.1);
    // `drift_to` alone changes nothing about `state` — the gap this closes.
    assert_eq!(link.state(), LinkState::Resonant { sync_phase: 0.05 });

    let still_ok = link.enforce_lock(CARRIER_RAD_PER_SEC, 1e-10);
    assert!(!still_ok, "a drifted link must not report itself as still locked");
    assert_eq!(
        link.state(),
        LinkState::Collapsed,
        "enforce_lock must actually collapse the link, not just report the loss"
    );
}

/// 4.6 — a link that never drifted is left completely alone.
#[test]
fn enforce_lock_leaves_a_locked_link_untouched() {
    let mut link =
        Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.1, 1.0)).unwrap();
    let before = link.clone();

    let still_ok = link.enforce_lock(CARRIER_RAD_PER_SEC, 1e-10);
    assert!(still_ok);
    assert_eq!(link, before, "a still-locked link must be byte-for-byte unchanged");
}

/// 4.7 — enforcing on an already-collapsed link is a no-op, not a second
/// teardown attempt. A caller driving the link's lifecycle should be able to
/// call this unconditionally without first checking `state()` itself.
#[test]
fn enforce_lock_is_idempotent_on_an_already_collapsed_link() {
    let mut link =
        Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.05, 1.0)).unwrap();
    link.teardown(CARRIER_RAD_PER_SEC, 1e-10).unwrap();
    let after_teardown = link.clone();

    let still_ok = link.enforce_lock(CARRIER_RAD_PER_SEC, 2e-10);
    assert!(!still_ok);
    assert_eq!(
        link, after_teardown,
        "enforcing on an already-collapsed link must not mutate it further"
    );
}

// ------------------------------------- Group 5: TEST CASE 1 (canonical)

/// 5.1 [D] - **Test Case 1**: waveform A at `phi=0`, B at `phi=pi`, sum zero.
///
/// The doctrine says "absolute zero". True zero is unavailable in IEEE-754 for
/// a general `t` because `x + pi` rounds. The tolerance comes from
/// [`ftg::cancellation_floor`], which **scales with the phase argument** rather
/// than being a constant - see its docs for the derivation and the measured
/// values behind it.
#[test]
fn test_case_1_destructive_interference_teardown() {
    let t = 3.7e-10;
    let sum = superpose(0.0, TEARDOWN_PHASE_SHIFT, CARRIER_RAD_PER_SEC, t);
    assert!(
        sum.abs() <= cancellation_floor(CARRIER_RAD_PER_SEC, t),
        "Test Case 1: superposition must evaluate to absolute zero, got {sum}"
    );
}

/// 5.2 [D] - cancellation holds at **every** instant, not just sampled ones.
///
/// Continuity is what lets teardown work without an acknowledgement message.
/// Swept across 40 periods, not one, so the growth of the residual with the
/// phase argument is actually exercised.
#[test]
fn cancellation_is_continuous_not_sampled() {
    let period = std::f64::consts::TAU / CARRIER_RAD_PER_SEC;
    for k in 0..40 {
        for i in 0..50 {
            let t = period * (f64::from(k) + f64::from(i) / 50.0);
            let sum = superpose(0.0, TEARDOWN_PHASE_SHIFT, CARRIER_RAD_PER_SEC, t);
            assert!(
                sum.abs() <= cancellation_floor(CARRIER_RAD_PER_SEC, t),
                "residual {sum} at t = {t}; cancellation must be continuous"
            );
        }
    }
}

/// 5.2b - the floor genuinely grows with the phase argument.
///
/// Pins the reason 5.1/5.2 cannot use a constant. If someone replaces
/// `cancellation_floor` with a fixed number, this fails and says why.
#[test]
fn cancellation_floor_scales_with_phase_argument() {
    let near = cancellation_floor(CARRIER_RAD_PER_SEC, 1e-10);
    let far = cancellation_floor(CARRIER_RAD_PER_SEC, 1e-7);
    assert!(
        far > near * 100.0,
        "the floor must scale with |omega*t|: near {near}, far {far}"
    );
}

/// 5.3 - teardown drives a real link's amplitude to zero.
#[test]
fn teardown_drives_amplitude_to_zero() {
    let link =
        Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.2, 1.0)).unwrap();
    for i in 1..20 {
        let t = 1e-10 * f64::from(i);
        let mut l = link.clone();
        let residual = l.teardown(CARRIER_RAD_PER_SEC, t).unwrap();
        assert!(
            residual <= cancellation_floor(CARRIER_RAD_PER_SEC, t),
            "teardown left residual {residual} at t = {t}"
        );
    }
}

/// 5.4 - the shift is exactly pi, taken from the MKB.
#[test]
fn teardown_shift_is_exactly_pi() {
    assert!((TEARDOWN_PHASE_SHIFT - std::f64::consts::PI).abs() < 1e-15);
}

// --------------------------------------------- Group 6: consumed, not rebuilt

/// 6.1 - routing distance is `lattice`'s metric, cross-checked.
#[test]
fn routing_distance_matches_lattice_metric() {
    let r = router();
    let t = lattice::Tiling::grow(5);
    let (a, b) = (r.cells()[0], r.cells()[17]);
    let expected = t
        .get(&a)
        .unwrap()
        .centre()
        .distance_to(&t.get(&b).unwrap().centre());
    assert!((r.distance(&a, &b) - expected).abs() < 1e-12);
}

/// 6.2 - the bit/phase mapping is `substrate`'s, one home.
#[test]
fn bit_phase_mapping_matches_substrate() {
    assert_eq!(PHASE_TRUE, substrate::constants::PHASE_TRUE);
    assert_eq!(PHASE_FALSE, substrate::constants::PHASE_FALSE);
    assert_eq!(CARRIER_RAD_PER_SEC, substrate::constants::CARRIER_RAD_PER_SEC);
}
