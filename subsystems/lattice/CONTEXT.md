---
type: subsystem
subsystem: lattice
tier: storage
language: Rust
stage: 04_implement
status: complete
prd_sections: ["5"]
binds_axioms: ["A3"]
result: "73 tests passing. Metric, algebra, tiling generation, neighbour naming, curved addressing with area preservation, scalar-to-cell resolution wired into substrate's MemoryPool, a numerical (x) inverse (solve_otimes), a general-purpose path-finding utility (shortest_path/shortest_distance), distinct from ftg's own Router, and the Tetryen node standing-wave envelope (tetryen_node_envelope), relocated here from gui so crystallisation can share it. No open items remain."
slices: ["metric + algebra + invariants", "tiling + neighbour naming", "curved addressing (PRD 5)", "scalar to cell resolution", "numerical (x) inverse", "general-purpose path-finding", "Tetryen node envelope (shared law)"]
---

# Lattice — hyperbolic 4D storage engine

One job: store and address data within a $\{5,4\}$ pentagonal hyperbolic tessellation, using curved addressing instead of linear sectors.

## The build loop

| Stage | Job | Output |
|---|---|---|
| `01_derive` | pull the exact law that binds this subsystem | `math-contract.md` |
| `02_design` | types and interfaces against that contract | `design.md` |
| `03_tests` | physics assertions, written before code | `test-plan.md` |
| `04_implement` | write the Rust into `neos/lattice/` | `implementation-log.md` |

## Scope

**Owns:** `neos/lattice/**` — `tessellation.rs`, `metric.rs`, `tetryen.rs`
**PRD sections:** §5 (File System and Data Storage)
**Axioms that bind it:** A3 (non-Euclidean addressing)
**Equations that bind it:** Hyperbolic Distance Function; the ⊗ operator per [`_mkb/operators.md`](../../_mkb/operators.md)
**Constants read:** `hyperbolic_curvature`, `tessellation.*`, `operators.*`, `scales.lattice_scale_R`

## Why this one is a good second build

`metric.rs` is pure mathematics with no I/O and no dependency on any other subsystem. Both of its equations already carry execution rules, and both are directly testable in wave terms. Of the five records, this has the cleanest path from contract to passing test — which makes it the best place to prove the doctrine works before committing to it everywhere.

## Resolved before build

The $\{5,4\}$ question is closed — see [reconciliation.md § R3](../../_mkb/reconciliation.md#r3--vertex-degree-of-the-tessellation--resolved). **Vertex degree is 4**, not the 5 that `vACUUM_FLUX.pdf` states in prose while writing `{5,4}` in the same sentence.

Two constraints from the reconciliation shape this build and are not negotiable:

- **⊗ is strongly non-associative** at the pinned scale. Never reorder a ⊗ chain; never `fold` over one without fixing the association order.
- **⊗ has a hard domain limit** at `a·b < 805.56`, above which `sinh` overflows `f64`. Enforce with a checked constructor, never with a comment.

## Solving (x), not just applying it

`oslash` was already documented as **not** a true inverse of `otimes` — the `sinh` correction doesn't cancel algebraically. `LatticeScalar::solve_otimes(self, target)` closes that gap the only way available: numerically, by bracketed bisection on `x` over `(x)`'s own domain for `self`.

Monotonicity is provable (differentiating `a*x + sinh(a*x*lambda)` gives `a * (1 + lambda*cosh(..))`, which never changes sign for fixed `a != 0`), so a bracket always contains at most one root. A plain Newton iteration was tried first and rejected — `sinh`'s derivative grows too fast approaching the domain edge, and Newton steps overshoot and diverge exactly where a solution still exists. Bisection cannot diverge: each step only ever halves the bracket.

Verified against a disposable Rust harness before touching the real crate, across a sweep of `a` from `0.01` to `50` (both signs) and `x` from `0.01` to `100` (both signs), including targets reachable only within 1 ulp of the domain edge: worst observed relative error `4.4e-14`. Refuses rather than guesses in two cases — `DegenerateInverse` when `a` is effectively zero (`0 (x) x = 0` for every `x`, nothing to invert), `UnreachableTarget` when no `x` inside the domain reaches `target` — matching how `otimes` itself refuses rather than saturating.

## A general-purpose path-finder, and why it isn't `ftg`'s router

Deferred at `02_design` and carried through every stage since: "the tiling now supports [geodesic path-finding]; it is not built." `pathfinding::shortest_path`/`shortest_distance` close it — exact breadth-first search over a `Tiling`, returning the real cell sequence.

Deliberately not the same thing as `ftg::layers_3_4::Router`. `ftg`'s router is memoryless by design — metric descent, one hop at a time, no stored path, no routing table — because "no routing table" is a physics-motivated transport constraint, not a shortcut; its own docs say a patch with holes could strand a packet. `lattice`'s version is the general tool: robust to incomplete or irregular patches, for any caller that needs an actual guaranteed path rather than a physically-constrained forwarding decision. BFS is *exact* here, not a heuristic, because every edge in `{5,4}` has the same geometric length (`centre_separation()` is one fixed constant for the whole tiling) — minimising hop count and minimising geodesic length are the same problem.

Cross-validated against `ftg`'s own independently-built `bfs_hops` (over the identical tiling, `neos/tests/ftg.rs`) — two separate BFS implementations agreeing exactly, rather than either trusted alone. Sabotage (queue order flipped from FIFO to LIFO, turning BFS into DFS) was caught by both: a lattice-side test using the tiling's own ring structure as ground truth, and the cross-check against `ftg`.

## A shared law fact, relocated rather than duplicated

`tetryen.rs::tetryen_node_envelope(r)` is `_mkb/tetryen.md`'s node standing-wave form, `A·sinh(r/R)·e^(−r/R)` (`A=1` in lattice-native units). It used to live in `gui::renderer::Tetryen::node_amplitude` directly. When `crystallisation::timecrystal::TetryenRecurrence` needed the same weight and `crystallisation` cannot depend on `gui` (dependency direction runs the other way), the fact moved down to the one crate both already depend on — one home per fact, not two copies. `gui::renderer::Tetryen::node_amplitude` is now a one-line delegation to it, re-verified unchanged by the existing `gui` suite (37/37) after the move.

## Do not

Load other subsystems' records. They don't share state; they share the factory (`_mkb/`, `_spec/`).
