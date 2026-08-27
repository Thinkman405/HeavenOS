//! A geometric routing and load balancing test bed.
//!
//! Doctrine: `_mkb/test-doctrine.md`. Law for routing is `ftg`'s Layer 3/4
//! binding (`_mkb/axioms.md`); law for load balancing is `_mkb/resonance.md`
//! Part 2 / `_mkb/reconciliation.md` R6.
//!
//! `ftg::layers_3_4::Router` ("geometric routing") and
//! `symphony_kernel::equilibrium::{CoreTopology, LoadField}` ("load
//! balancing") are both built from the identical real `{5,4}` hyperbolic
//! tessellation (`lattice::Tiling`) — but until this file, nothing ever
//! checked that the two subsystems actually agree on that geometry, and
//! neither subsystem's own suite was swept much past its smallest useful
//! example (`ftg` stops at depth 5 / 441 cells; `symphony_kernel` stops at 64
//! cores). Per this workspace's own recorded lesson — a correct assertion
//! over too narrow a domain hid a real `xi(r)` divergence past `r ~ 710` —
//! this is a dedicated, wider-sweep harness, not a duplicate of either
//! subsystem's own suite.
//!
//! **What this file deliberately does not do**: it does not couple the two
//! systems. No load-aware routing, no routing-aware load balancing — no
//! axiom or law names that coupling, so none is invented here. This is a
//! shared verification bed over one geometry two subsystems already build
//! independently, not a new feature joining them.

use ftg::layers_3_4::Router;
use lattice::tessellation::CellId;
use lattice::Tiling;
use std::collections::HashSet;
use symphony_kernel::{CoreTopology, KernelError, LoadField};

/// A router and a core topology built over the *same* full patch: `depth`'s
/// entire tiling, both ways. `CoreTopology::from_tiling` is asked to grow
/// exactly `Router::new(depth)`'s own cell count — since both consume
/// `lattice::Tiling::grow`/`Tiling::layer` in the same ring order, this lands
/// on the identical patch rather than a coincidentally same-sized different
/// one, confirmed directly below rather than assumed.
fn full_patch(depth: usize) -> (Router, CoreTopology) {
    let router = Router::new(depth);
    let topo = CoreTopology::from_tiling(router.cell_count());
    (router, topo)
}

// -------------------------------------------- cross-consistency: one geometry

/// The identity this file exists to check first: two subsystems, each with
/// their own struct, each independently calling into `lattice::Tiling`,
/// describe the exact same set of cells for the same full-patch size.
#[test]
fn router_and_core_topology_describe_the_same_geometry() {
    for depth in 1..=7 {
        let (router, topo) = full_patch(depth);
        assert_eq!(
            topo.cells(),
            router.cells(),
            "depth {depth}: CoreTopology and Router disagree on which cells make up the patch"
        );
    }
}

/// The sharper claim: not just the same cells, but the same edges — checked
/// against `lattice::Tiling` itself as the independent ground truth, not
/// against each other, so this cannot pass by both sides sharing one bug.
/// `Cell::neighbors()` always returns the full 5-neighbor group-theoretic set
/// regardless of patch boundary; a neighbor only counts here if it is also
/// one of this patch's own materialised cells — exactly the distinction that
/// makes boundary valence real (see the tests below).
#[test]
fn router_and_core_topology_agree_on_every_real_edge_in_the_full_patch() {
    for depth in 1..=7 {
        let (router, topo) = full_patch(depth);
        let raw = Tiling::grow(depth);

        for (i, &a) in router.cells().iter().enumerate() {
            let ground_truth: HashSet<CellId> = raw
                .get(&a)
                .expect("every patch cell exists in its own tiling")
                .neighbors()
                .iter()
                .map(|c| c.id())
                .filter(|id| router.contains(id))
                .collect();

            let core_neighbors: HashSet<CellId> =
                topo.neighbors(i).iter().map(|&j| topo.cells()[j]).collect();
            assert_eq!(
                ground_truth, core_neighbors,
                "depth {depth}, cell {i}: CoreTopology's edges don't match lattice::Tiling"
            );

            for &b in &ground_truth {
                assert!(
                    router.adjacent(&a, &b),
                    "depth {depth}, cell {i}: Router disagrees that {b:?} is a real neighbor"
                );
            }
            // The negative case matters just as much as the positive one: an
            // `adjacent()` that answered "yes" to almost everything (e.g. "at
            // least one of my 5 neighbors differs from b", true for nearly
            // every b) would still pass the loop above, since every true
            // neighbor is also included. A handful of deterministic
            // non-neighbor probes per cell — not an exhaustive O(n^2) scan,
            // which this geometry's own per-call cost makes too slow at
            // scale — catches that shape of bug without it.
            let n = router.cells().len();
            for offset in [n / 3, 2 * n / 3, n / 2 + 1] {
                let c = router.cells()[(i + offset.max(1)) % n];
                if c != a && !ground_truth.contains(&c) {
                    assert!(
                        !router.adjacent(&a, &c),
                        "depth {depth}, cell {i}: Router wrongly claims {c:?} is a neighbor"
                    );
                }
            }
            assert_eq!(
                topo.degree(i),
                ground_truth.len(),
                "depth {depth}, cell {i}: CoreTopology's own degree() disagrees with its own neighbor set"
            );
        }
    }
}

// ------------------------------------------------- boundary valence, for real

/// CLAUDE.md's own rigor rule, checked rather than assumed: "measure local
/// cell degrees dynamically... boundary valence dropping from 5 to 1." Every
/// non-trivial patch size swept here actually reaches degree exactly 1
/// somewhere on its boundary — confirmed with a disposable scratch harness
/// before this assertion was written, not guessed at.
#[test]
fn boundary_valence_really_drops_all_the_way_to_one() {
    for n in [6usize, 7, 21, 61, 100, 166, 441] {
        let topo = CoreTopology::from_tiling(n);
        assert_eq!(
            topo.min_degree(),
            1,
            "n={n}: expected at least one real boundary cell of degree exactly 1"
        );
        assert_eq!(
            topo.max_degree(),
            5,
            "n={n}: interior cells must still reach the full {{5,4}} degree"
        );
        assert!(
            topo.is_connected(),
            "n={n}: a patch this size must still be one connected graph"
        );
    }
}

/// The degenerate extreme: a single-core "topology" has no neighbours and no
/// edges at all. `stability_bound()` divides by `max_degree().max(1)`
/// specifically to survive this without a NaN/Inf/panic — confirmed directly
/// here rather than trusted from reading the guard.
#[test]
fn a_single_core_topology_is_the_zero_degree_limit_and_does_not_break_the_bound() {
    let topo = CoreTopology::from_tiling(1);
    assert_eq!(topo.len(), 1);
    assert_eq!(topo.min_degree(), 0);
    assert_eq!(topo.max_degree(), 0);
    assert!(
        topo.is_connected(),
        "a single node is trivially connected to itself"
    );
    let bound = topo.stability_bound();
    assert!(
        bound.is_finite() && bound > 0.0,
        "stability_bound() must not divide by zero degree: got {bound}"
    );
}

// ---------------------------------------------- load balancing across scale

/// A real, non-trivial imbalance: one hot core carrying far more than its
/// share, the rest at a common baseline — scaled with `n` so the imbalance
/// stays proportionally severe as the patch grows, not diluted away.
fn hotspot_load(n: usize) -> Vec<f64> {
    let mut load = vec![1.0; n];
    load[0] = 1.0 + n as f64 * 50.0;
    load
}

/// `_mkb/resonance.md` Part 2's own worked example is a 4-core ring; the
/// subsystem's existing tests stop at 64 cores. This sweeps to 441 — `ftg`'s
/// own documented "true patch" size — so load balancing is verified at
/// exactly the scale routing already claims as real, not just at small
/// hand-picked examples.
#[test]
fn load_balancing_converges_across_a_wide_sweep_of_real_patch_sizes() {
    for n in [2usize, 6, 21, 61, 166, 441] {
        let topo = CoreTopology::from_tiling(n);
        let bound = topo.stability_bound();
        let mut field = LoadField::new(hotspot_load(n));
        let alpha = 0.9 * bound;
        let steps = field
            .relax_to_equilibrium(&topo, alpha, 1e-6, 5_000)
            .unwrap_or_else(|e| panic!("n={n}: {e}"));
        assert!(
            field.relative_spread() <= 1e-6,
            "n={n}: did not actually converge within the step budget (spread {})",
            field.relative_spread()
        );
        assert!(
            steps < 5_000,
            "n={n}: consumed the entire step budget without early convergence"
        );
    }
}

/// The stability bound is a hard analytic edge, not a rule of thumb: this
/// checks both sides of it, at every swept size, not just at the one
/// hand-picked core count the subsystem's own existing tests use.
#[test]
fn the_stability_bound_holds_exactly_across_the_same_sweep() {
    for n in [2usize, 6, 21, 61, 166, 441] {
        let topo = CoreTopology::from_tiling(n);
        let bound = topo.stability_bound();

        let mut converges = LoadField::new(hotspot_load(n));
        converges
            .relax_to_equilibrium(&topo, 0.999 * bound, 1e-6, 5_000)
            .unwrap_or_else(|e| panic!("n={n}: just below the bound must still converge: {e}"));

        let mut at_bound = LoadField::new(hotspot_load(n));
        let err = match at_bound.relax_to_equilibrium(&topo, bound, 1e-6, 10) {
            Err(e) => e,
            Ok(steps) => panic!("n={n}: alpha == bound must be refused, converged in {steps} steps instead"),
        };
        assert!(
            matches!(err, KernelError::Unstable { alpha, bound: b } if alpha == bound && b == bound),
            "n={n}: wrong error shape: {err:?}"
        );
    }
}

// -------------------------------------------------- routing beyond depth 5

/// `ftg`'s own suite never samples past `Router::new(5)` (441 cells, its
/// documented "true patch"). This extends the identical BFS-optimality claim
/// to depth 6 (1161 cells) — real coverage past the previous ceiling, kept to
/// a modest sample since `bfs_hops`/`route` genuinely cost real per-call time
/// on this geometry: dense sampling at depth 7 measured in the minutes, not
/// seconds, while this file was being developed.
#[test]
fn greedy_routing_is_still_complete_and_bfs_optimal_past_ftgs_own_tested_depth() {
    let router = Router::new(6);
    let cells = router.cells();
    let mut checked = 0;
    for i in (0..cells.len()).step_by(150) {
        for j in (0..cells.len()).step_by(170) {
            if i == j {
                continue;
            }
            let path = router
                .route(cells[i], cells[j], 400)
                .unwrap_or_else(|e| panic!("routing {i}->{j} at depth 6 failed: {e}"));
            let optimal = router
                .bfs_hops(cells[i], cells[j])
                .expect("depth 6's patch is connected");
            assert_eq!(
                path.len() - 1,
                optimal,
                "depth 6, {i}->{j}: greedy took {} hops, BFS-optimal is {optimal}",
                path.len() - 1
            );
            checked += 1;
        }
    }
    assert!(
        checked > 20,
        "expected a meaningful sample, checked {checked}"
    );
}
