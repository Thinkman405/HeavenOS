//! Assertions for {5,4} tiling generation and neighbour naming.
//!
//! Doctrine: `_mkb/test-doctrine.md`. Tests marked **[D]** would fail against a
//! Euclidean or otherwise non-hyperbolic implementation.
//!
//! The strongest results here are *exact integer* identities — the ring sizes
//! are `5·Fib(2n)` with no tolerance at all — which is unusual for geometry
//! code and worth preserving.

use lattice::isometry::{HyperboloidPoint, Isometry};
use lattice::tessellation as tess;
use lattice::{shortest_distance, shortest_path, Cell, CellId, Tiling};

// ------------------------------------------------------- generators

/// Each edge generator must be an involution.
///
/// This is the property my first construction lacked: a rotate-then-translate
/// step is not an involution, so stepping across an edge and back never
/// returned, and the enumeration produced a free tree of `5^n` cells.
#[test]
fn edge_generators_are_involutions() {
    for (k, g) in tess::generators().iter().enumerate() {
        let sq = g.compose(g);
        let id = Isometry::IDENTITY;
        let err = (0..3)
            .flat_map(|i| (0..3).map(move |j| (i, j)))
            .map(|(i, j)| (sq.as_matrix()[i][j] - id.as_matrix()[i][j]).abs())
            .fold(0.0_f64, f64::max);
        assert!(err < 1e-12, "generator {k} is not an involution: |g^2 - I| = {err}");
    }
}

/// Every generator moves the origin cell exactly one edge-crossing away.
#[test]
fn generators_step_exactly_one_cell() {
    let o = HyperboloidPoint::ORIGIN;
    let expected = tess::centre_separation();
    for (k, g) in tess::generators().iter().enumerate() {
        let d = o.distance_to(&g.origin_image());
        assert!(
            (d - expected).abs() < 1e-12,
            "generator {k} moved {d}, expected {expected}"
        );
    }
}

// ------------------------------------------------------- neighbour naming

/// Every cell has exactly five distinctly-named neighbours.
#[test]
fn each_cell_has_five_distinct_neighbours() {
    let tiling = Tiling::grow(3);
    for cell in tiling.iter() {
        let ids: Vec<CellId> = cell.neighbors().iter().map(Cell::id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            5,
            "cell {:?} produced duplicate neighbour names: {ids:?}",
            cell.id()
        );
        assert!(!ids.contains(&cell.id()), "a cell must not be its own neighbour");
    }
}

/// Crossing an edge and crossing back returns to the same cell — the concrete
/// consequence of the generators being involutions.
#[test]
fn crossing_an_edge_twice_returns_home() {
    let tiling = Tiling::grow(3);
    for cell in tiling.iter() {
        for edge in 0..5 {
            let there = cell.neighbor(edge).expect("edge index in range");
            let back = there.neighbor(edge).expect("edge index in range");
            assert_eq!(
                back.id(),
                cell.id(),
                "edge {edge} round trip from {:?} landed on {:?}",
                cell.id(),
                back.id()
            );
        }
    }
}

/// Adjacency is symmetric: if B is a neighbour of A, then A is a neighbour of B.
#[test]
fn adjacency_is_symmetric() {
    // Depth 5. Rings 0-4 (166 cells) have every neighbour inside the grown
    // region, giving 830 interior adjacencies before ring 5 contributes any.
    let tiling = Tiling::grow(5);
    let mut checked = 0usize;
    for cell in tiling.iter() {
        for nbr in cell.neighbors() {
            if !tiling.contains(&nbr.id()) {
                continue; // beyond the grown region
            }
            let back: Vec<CellId> = nbr.neighbors().iter().map(Cell::id).collect();
            assert!(
                back.contains(&cell.id()),
                "{:?} lists {:?} as a neighbour, but not conversely",
                cell.id(),
                nbr.id()
            );
            checked += 1;
        }
    }
    assert!(checked > 1000, "expected a meaningful sample, checked {checked}");
}

/// All neighbours sit exactly one edge-crossing away.
#[test]
fn neighbours_are_one_separation_away() {
    let tiling = Tiling::grow(3);
    let expected = tess::centre_separation();
    for cell in tiling.iter() {
        for nbr in cell.neighbors() {
            let d = cell.centre().distance_to(&nbr.centre());
            assert!(
                (d - expected).abs() < 1e-9,
                "neighbour distance {d}, expected {expected}"
            );
        }
    }
}

// ------------------------------------------------------- the word problem

/// Distinct cells are separated by at least `2 x inradius`.
///
/// This is the bound that makes identity-by-centre a sound decision procedure
/// for the word problem: the quantisation grid is nine decimal places, while
/// genuine cells are ~1.2537 apart.
#[test]
fn distinct_cells_are_well_separated() {
    let tiling = Tiling::grow(4);
    let cells: Vec<&Cell> = tiling.iter().collect();
    let expected = tess::centre_separation();
    let mut min = f64::INFINITY;
    for i in 0..cells.len() {
        for j in (i + 1)..cells.len() {
            let d = cells[i].centre().distance_to(&cells[j].centre());
            if d < min {
                min = d;
            }
        }
    }
    assert!(
        (min - expected).abs() < 1e-6,
        "minimum separation {min} should equal 2*inradius {expected}"
    );
    assert!(min > 1.25, "separation {min} is far above any rounding scale");
}

/// Different words reaching the same cell must produce the same name.
///
/// Going out across edge 0 and back is the identity word; it must name the
/// origin cell, not a distinct one.
#[test]
fn equal_words_produce_equal_names() {
    let origin = Cell::at_origin();
    let round_trip = origin
        .neighbor(0)
        .and_then(|c| c.neighbor(0))
        .expect("edge index in range");
    assert_eq!(round_trip.id(), origin.id());
    assert_eq!(origin.id(), CellId::ORIGIN);
}

// ------------------------------------------------------- growth structure

/// [D] Ring sizes are exactly `5 * Fib(2n)`.
///
/// An **exact integer identity** — no tolerance. A Euclidean {5,4} does not
/// exist at all, and Euclidean tilings grow linearly per ring rather than
/// exponentially.
#[test]
fn ring_sizes_are_five_times_even_fibonacci() {
    let tiling = Tiling::grow(7);
    let sizes = tiling.layer_sizes();

    let mut fib = vec![0usize, 1];
    while fib.len() < 32 {
        fib.push(fib[fib.len() - 1] + fib[fib.len() - 2]);
    }

    assert_eq!(sizes[0], 1, "the origin ring holds one cell");
    for (n, &size) in sizes.iter().enumerate().skip(1) {
        assert_eq!(
            size,
            5 * fib[2 * n],
            "ring {n} has {size} cells, expected 5*Fib({}) = {}",
            2 * n,
            5 * fib[2 * n]
        );
    }
    assert_eq!(&sizes[..8], &[1, 5, 15, 40, 105, 275, 720, 1885]);
}

/// The linear recurrence behind those counts: `a(n) = 3a(n-1) - a(n-2)`.
#[test]
fn ring_sizes_obey_the_linear_recurrence() {
    let sizes = Tiling::grow(7).layer_sizes();
    for n in 3..sizes.len() {
        assert_eq!(
            sizes[n],
            3 * sizes[n - 1] - sizes[n - 2],
            "recurrence broken at ring {n}"
        );
    }
}

/// [D] Growth constant is `phi^2 = (3+sqrt5)/2`.
///
/// Exponential ring growth is the signature of negative curvature. A Euclidean
/// tiling grows linearly, so this ratio would tend to 1.
#[test]
fn growth_constant_approaches_phi_squared() {
    let sizes = Tiling::grow(7).layer_sizes();
    let n = sizes.len();
    let ratio = sizes[n - 1] as f64 / sizes[n - 2] as f64;
    let phi_sq = (3.0 + 5.0_f64.sqrt()) / 2.0;
    assert!(
        (ratio - phi_sq).abs() < 1e-3,
        "growth ratio {ratio} should approach phi^2 = {phi_sq}"
    );
    assert!(ratio > 2.5, "growth must be exponential, not linear");
}

// ------------------------------------------------------- vertex degree

/// [D] Four cells meet at each vertex — derived from the geometry, not read
/// from the constant.
///
/// This independently confirms reconciliation R3, which resolved
/// `vACUUM_FLUX.pdf` writing "{5,4}" and "five pentagons meet at each vertex"
/// in the same sentence. The tiling itself says four.
#[test]
fn four_cells_meet_at_each_vertex() {
    let tiling = Tiling::grow(4);
    let vertex = Tiling::origin_cell_vertex();
    let at_vertex = tiling.cells_at_vertex(&vertex, 1e-6);
    assert_eq!(
        at_vertex.len(),
        4,
        "expected 4 cells at the vertex, found {} — reconciliation R3 says four \
         pentagons per vertex, and the group action must agree",
        at_vertex.len()
    );
    assert!(
        at_vertex.contains(&CellId::ORIGIN),
        "the origin cell owns this vertex and must be among them"
    );
}

// ------------------------------------------------------- embedding

/// Cell centres embed into the 4-ball and respect its invariant.
#[test]
fn cell_centres_embed_in_the_four_ball() {
    let tiling = Tiling::grow(3);
    for cell in tiling.iter() {
        let p = cell
            .centre_in_ball()
            .expect("a cell centre is always inside the ball");
        assert!(p.norm() < 1.0);
    }
}

/// Growing deeper strictly adds cells, and the total matches the ring sums.
#[test]
fn tiling_totals_match_ring_sums() {
    for depth in 1..=6 {
        let t = Tiling::grow(depth);
        assert_eq!(t.len(), t.layer_sizes().iter().sum::<usize>());
        assert!(t.len() > Tiling::grow(depth - 1).len());
    }
}

// ------------------------------------------------------- path-finding
//
// `lattice::shortest_path`/`shortest_distance` — general-purpose, exact
// (every edge has the same length, so hop count and geodesic length agree)
// BFS over the tiling, distinct from `ftg::layers_3_4::Router`'s own
// memoryless greedy descent. See `pathfinding.rs`'s module docs for why the
// two are deliberately separate tools, and `tests/ftg.rs` for the
// cross-check against `ftg`'s own independent `bfs_hops`.

/// A path from a cell to itself is just that cell, zero hops.
#[test]
fn path_to_self_is_the_single_cell() {
    let tiling = Tiling::grow(3);
    let c = CellId::ORIGIN;
    assert_eq!(shortest_path(&tiling, c, c), Some(vec![c]));
    assert_eq!(shortest_distance(&tiling, c, c), Some(0));
}

/// Edge-adjacent cells are exactly one hop apart.
#[test]
fn adjacent_cells_are_one_hop_apart() {
    let tiling = Tiling::grow(3);
    let origin = tiling.get(&CellId::ORIGIN).unwrap();
    for n in origin.neighbors() {
        let dist = shortest_distance(&tiling, CellId::ORIGIN, n.id())
            .expect("a direct neighbour must be reachable");
        assert_eq!(dist, 1, "neighbour {:?} should be 1 hop away, got {dist}", n.id());
    }
}

/// The returned path is a real walk: every consecutive pair is actually
/// edge-adjacent in the tiling, not merely the right length.
#[test]
fn returned_path_is_a_real_walk() {
    let tiling = Tiling::grow(4);
    let cells: Vec<CellId> = tiling.iter().map(|c| c.id()).collect();
    let mut checked = 0;
    for i in (0..cells.len()).step_by(17) {
        for j in (0..cells.len()).step_by(19) {
            let Some(path) = shortest_path(&tiling, cells[i], cells[j]) else {
                continue;
            };
            for pair in path.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let cell_a = tiling.get(&a).expect("path cells must be in the tiling");
                assert!(
                    cell_a.neighbors().iter().any(|n| n.id() == b),
                    "{a:?} -> {b:?} in the returned path are not actually adjacent"
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 50, "expected a meaningful sample, checked {checked}");
}

/// [D] Hop distance from the origin to any cell in ring `n` is exactly `n`
/// — the tiling's own ring structure as ground truth, independent of any
/// BFS implementation detail.
#[test]
fn distance_from_origin_matches_ring_depth() {
    let tiling = Tiling::grow(5);
    for ring in 1..=5 {
        let layer = tiling.layer(ring).expect("ring exists at this depth");
        for &cell in layer.iter().step_by(3) {
            let dist = shortest_distance(&tiling, CellId::ORIGIN, cell)
                .expect("every cell in a grown tiling is reachable from the origin");
            assert_eq!(
                dist, ring,
                "cell {cell:?} is in ring {ring} but BFS distance from origin is {dist}"
            );
        }
    }
}

/// A cell absent from the tiling has no path to or from it — "unreachable"
/// is a real answer, not an error.
#[test]
fn absent_cells_have_no_path() {
    let shallow = Tiling::grow(2);
    let outside = *Tiling::grow(6)
        .layer(5)
        .expect("a depth-6 tiling has a ring 5")
        .first()
        .unwrap();
    assert!(
        !shallow.contains(&outside),
        "premise: a ring-5 cell must not be in a depth-2 tiling"
    );
    assert_eq!(shortest_path(&shallow, CellId::ORIGIN, outside), None);
    assert_eq!(shortest_distance(&shallow, outside, CellId::ORIGIN), None);
}

/// No cell repeats within a returned path.
#[test]
fn path_visits_each_cell_at_most_once() {
    let tiling = Tiling::grow(4);
    let far = *tiling.layer(4).unwrap().last().unwrap();
    let path = shortest_path(&tiling, CellId::ORIGIN, far).expect("connected");

    let mut sorted = path.clone();
    sorted.sort();
    let before = sorted.len();
    sorted.dedup();
    assert_eq!(sorted.len(), before, "path revisited a cell");
}
