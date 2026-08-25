//! General-purpose path-finding over a [`Tiling`], owned by `lattice` rather
//! than duplicated by every consumer that needs an actual path between two
//! cells.
//!
//! Deferred at `02_design`, and again through `03_tests`/`04_implement`:
//! "the tiling now supports [geodesic path-finding]; it is not built." This
//! closes it.
//!
//! ## Distinct from `ftg::layers_3_4::Router`, deliberately
//!
//! `ftg`'s router is not a path-finder in this sense at all. Forwarding
//! there is metric descent — one hop at a time, no routing table, no stored
//! path — because "no routing table" is a physics-motivated transport-layer
//! constraint, not an implementation shortcut: real packet forwarding has
//! no global view of the network. Greedy descent is *measured* to be
//! BFS-optimal on a complete `{5,4}` patch, but `ftg`'s own docs are honest
//! that it isn't robust in general — "a patch with holes could strand a
//! packet." `ftg` even carries its own private `bfs_hops`, explicitly never
//! called by routing itself, existing only so its test suite can check
//! descent against the true answer.
//!
//! This *is* that true answer, generalised and given a real home: exact
//! shortest-path search that returns the actual cell sequence, robust to
//! incomplete or irregular patches, for any caller that needs a real path
//! rather than a physically-constrained forwarding decision.
//!
//! ## Why breadth-first search is exact here, not a heuristic
//!
//! Every edge in a `{5,4}` tessellation has the same geometric length —
//! `tessellation::centre_separation()` is one fixed constant for the whole
//! tiling, not a per-edge value. Minimising hop count and minimising total
//! geodesic length are therefore the same problem, and BFS solves the
//! unweighted version exactly. A Dijkstra-style weighted search would be
//! solving a generalisation this tiling's own uniformity doesn't need.

use crate::tessellation::{CellId, Tiling};
use std::collections::{HashSet, VecDeque};

/// The shortest path from `src` to `dst` within `tiling`, inclusive of both
/// endpoints.
///
/// `None` if `dst` is unreachable from `src` — including if either is
/// absent from `tiling` — which is not treated as an error: "no path
/// exists" is a legitimate fact about the graph's own connectivity, not a
/// search failure.
pub fn shortest_path(tiling: &Tiling, src: CellId, dst: CellId) -> Option<Vec<CellId>> {
    if !tiling.contains(&src) || !tiling.contains(&dst) {
        return None;
    }
    if src == dst {
        return Some(vec![src]);
    }

    let mut parent: std::collections::HashMap<CellId, CellId> = std::collections::HashMap::new();
    let mut seen: HashSet<CellId> = HashSet::from([src]);
    let mut queue: VecDeque<CellId> = VecDeque::from([src]);

    while let Some(cur) = queue.pop_front() {
        let Some(cell) = tiling.get(&cur) else { continue };
        for n in cell.neighbors() {
            let id = n.id();
            if !tiling.contains(&id) || !seen.insert(id) {
                continue;
            }
            parent.insert(id, cur);
            if id == dst {
                let mut path = vec![dst];
                let mut cursor = dst;
                while cursor != src {
                    cursor = parent[&cursor];
                    path.push(cursor);
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(id);
        }
    }
    None
}

/// Hop count alone, when the path itself isn't needed. `None` under the
/// same conditions as [`shortest_path`].
pub fn shortest_distance(tiling: &Tiling, src: CellId, dst: CellId) -> Option<usize> {
    shortest_path(tiling, src, dst).map(|path| path.len() - 1)
}
