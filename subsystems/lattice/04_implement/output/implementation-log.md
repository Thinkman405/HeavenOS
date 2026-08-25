---
type: implementation-log
subsystem: lattice
stage: 04_implement
status: complete
toolchain: rustc 1.97.1 / cargo 1.97.1
result: 73 passed, 0 failed — see pathfinding addendum
slices: ["metric + algebra + invariants", "tiling generation + neighbour naming", "numerical (x) inverse"]
---

# Lattice — Implementation Log

## Result

```
cargo build --workspace   → Finished, no warnings
cargo test  --workspace   → 38 passed; 0 failed
                            (24 metric/algebra + 14 tiling)
```

Every assertion in [test-plan.md](../../03_tests/output/test-plan.md) is implemented and passing. Tests were written first and confirmed failing before the implementation existed.

## Files written

| Path | Role |
|---|---|
| `neos/Cargo.toml` | workspace manifest, `lattice` only |
| `neos/lattice/Cargo.toml` | crate manifest |
| `neos/lattice/build.rs` | generates constants from `_mkb/constants.json` |
| `neos/lattice/src/lib.rs` | crate root |
| `neos/lattice/src/metric.rs` | ⊗, ⊘, `d_H`, the two invariant-carrying types |
| `neos/lattice/src/tessellation.rs` | `{5,4}` constants and closed forms |
| `neos/tests/lattice_metric.rs` | 23 physics assertions |

## Verification beyond the suite

**Doctrine check performed.** ⊗ was temporarily replaced with ordinary multiplication (`Ok(Self(product))`) and the suite re-run. Result: **4 tests failed**, including `otimes_unit_bifurcation_is_exactly_two`. The sabotage was reverted and the full suite re-confirmed green; `grep` confirms no marker remains.

This is the check that matters. A suite that stayed green under that substitution would be testing nothing about NEOS. The 19 tests that stayed green are the distance and tessellation groups, which do not route through ⊗ — correct behaviour, not a gap.

**One home per fact confirmed.** `grep` for every MKB constant value across all `.rs` files returns nothing. Each flows from `constants.json` through `build.rs`.

## Deviations from the design

**None affecting behaviour.** Three mechanism notes:

1. **Test target declared explicitly.** `_spec/target-tree.md` places tests at `neos/tests/`, but the workspace root is a virtual manifest so Cargo will not auto-discover them there. Wired with an explicit `[[test]]` path in `lattice/Cargo.toml`. The spec's layout is honoured rather than the file being moved — keeping the spec and the filesystem in agreement was the point.

2. **`opt-level = 1` on the test profile.** Set with a comment explaining why no fast-math-style flag may ever be added: reassociation would silently change ⊗ results, because ⊗ is not associative. rustc does not enable such flags by default; the comment exists so nobody adds one later.

3. **Test expectations are hand-computed closed forms**, not MKB constants — the radii `0.5306…`/`0.6269…` and `2⊗3 = 104.9949…` appear literally in the test file. That is the correct place for them: a test whose expected value is imported from the code under test proves nothing. All were computed independently and verified before the implementation was written.

## Design decisions worth recording

**`build.rs` asserts the JSON is self-consistent.** It checks that `tessellation.schlafli`'s `q` equals `tessellation.vertex_degree` and fails the build if not. This is the R3 resolution defended mechanically — the exact confusion that produced the original contradiction cannot silently reappear in the JSON.

**`LatticeScalar` has no `impl Mul`.** Per contract §3.3. An operator symbol would hide non-associativity at precisely the call sites where it matters. `otimes` returning `Result` also forces the domain question into view.

**`PoincarePoint::new` rejects NaN explicitly.** The natural check `norm < 1.0` *accepts* NaN, since every comparison against NaN is false. Written as `!(norm < 1.0)` plus a finiteness pass, with a comment. This is a bug the type system would not have caught.

---

# Slice 2 — tiling generation and neighbour naming

Added `neos/lattice/src/isometry.rs`, rewrote `tessellation.rs`, added `neos/tests/lattice_tiling.rs` (14 assertions).

## A correctness fix in slice 1

**`circumradius()` was returning the half-edge length.** It computed `acosh(cos(π/p)/sin(π/q)) ≈ 0.5306`, which is the vertex-to-edge-midpoint leg, not the centre-to-vertex hypotenuse (`acosh(cot(π/p)·cot(π/q)) ≈ 0.8425`).

The consequence was visible and I mis-read it: the code produced `inradius > circumradius`, and rather than investigate an impossible result — a centre-to-edge distance cannot exceed a centre-to-vertex distance in *any* geometry — I wrote a test pinning the inversion and a comment calling it "counter-intuitive but correct... do not swap the formulas". That comment actively defended the bug.

Fixed: `circumradius()` corrected, `half_edge_length()` added, the test rewritten to assert `inradius < circumradius`, and **hyperbolic Pythagoras (`cosh c = cosh a · cosh b`) added as a permanent guard** — that identity fails immediately under the swap and would have caught it at once.

Nothing downstream consumed the wrong value, so no other code changed.

## The generator must be a reflection

The first working attempt used `rotate(2πk/5) ∘ translate(2·inradius)`. Every neighbour landed at exactly the right distance, and the construction looked correct — but it is **not an involution**, so crossing an edge and crossing back did not return. The enumeration unfolded into a free tree: `1, 5, 25, 125, 625…`, growing at exactly 5× per ring with no coincidences.

The correct generator conjugates an edge reflection into place:

```
gen_k = R(2πk/p) · [T(inradius) ∘ flip_x ∘ T(−inradius)] · R(−2πk/p)
```

This is an involution by construction, and the tiling closes.

## Verified structure

| Property | Result |
|---|---|
| ring sizes | `1, 5, 15, 40, 105, 275, 720, 1885` = **exactly `5·Fib(2n)`** |
| recurrence | `a(n) = 3a(n−1) − a(n−2)`, exact |
| growth constant | → `φ² = 2.618033988749895` |
| minimum cell separation | `2 × inradius ≈ 1.2537` |
| adjacency symmetry | 0 asymmetric pairs over ~1100 interior adjacencies |
| **cells per vertex** | **4 — derived from the group action, independently confirming reconciliation R3** |

The ring-size identity is exact integer arithmetic with no tolerance, which is unusual for geometry code and worth keeping.

## The word problem

A cell is named by a word in the five edge reflections; two words name the same cell iff their isometries agree on the origin. Decided by geometric realisation — project the centre to the Poincaré disk, quantise at `1e-9`.

**Soundness:** distinct cell centres are separated by at least `2 × inradius ≈ 1.2537` in the hyperbolic metric, nine orders of magnitude above the quantisation grid. Dedup cannot merge distinct cells or split a single one at any depth this enumerator is used for. The separation bound is asserted directly (test 6.7) rather than assumed.

## Doctrine check on slice 2

The exact bug above was reintroduced (generators reverted to rotate-then-translate) and the suite re-run: **9 of 14 tiling tests failed.**

Notably `generators_step_exactly_one_cell` and `neighbours_are_one_separation_away` **still passed** — the broken generator does move the correct distance. That is precisely why the bug was subtle, and why distance checks alone were never going to catch it. The tests that caught it were the closure and structure ones: involution, round trip, ring sizes, vertex degree.

Reverted, full suite re-confirmed at 38/38, `grep` confirms no marker remains.

## Numerical choices

`Isometry` works in the hyperboloid model rather than with Möbius transformations on the disk. Cells crowd exponentially toward `‖u‖ = 1`, so disk coordinates lose absolute precision exactly where the tiling is interesting; hyperboloid coordinates grow instead, preserving relative precision. Inversion is `J Mᵀ J` with `J = diag(1,1,−1)` — exact for `O(2,1)`, no general matrix inversion.

## What is still not built

- **Geodesic path-finding** between arbitrary cells. The tiling supports it; it is not implemented.
- **⊗-based address arithmetic** over cell coordinates. This is the actual storage-addressing job from PRD §5 — the tiling is the substrate for it, not the thing itself.
- **A genuine 4D honeycomb.** `{5,4}` tessellates H², while the lattice is H⁴; the tiling occupies a 2-plane of the 4-ball. A rank-4 Schläfli symbol would be needed and the corpus does not supply one. Recorded as a real gap.

## Human check

Run `cargo test --workspace`. Read `ring_sizes_are_five_times_even_fibonacci` and `four_cells_meet_at_each_vertex` — the first is an exact integer identity that no approximate implementation would satisfy, and the second re-derives the R3 decision from geometry alone rather than reading the constant it is supposed to check.

---

# Slice 3 — curved addressing (PRD §5)

Added `neos/lattice/src/addressing.rs` and `neos/tests/lattice_addressing.rs` (15 assertions). Workspace total: **221 passing**.

Builds the two claims in PRD §5: that `⊗` traverses the directory tree, and that hyperbolic storage eliminates fragmentation. Both hold — and both carry a constraint the PRD does not mention.

## The constraint the spec omits: paths are shallow

`⊗` grows super-exponentially and its domain stops at `a·b < 805.56`, so a path cannot be long. **Step magnitude decides reachable depth**, measured:

| step | reachable depth |
|---|---|
| 0.1 | 40+ |
| 0.5 | 40+ |
| **1.0** | **4** |
| **2.0** | **2** |
| 3.0 | 2 |

Sub-unit steps *contract* the running product and traverse indefinitely; unit-or-larger steps explode within a few levels. **A directory tree addressed by `⊗` with unit segments is about four levels deep.**

That is a real limit on the storage model, not a limit of this implementation. It is asserted in `step_magnitude_decides_depth` with the measured numbers, and `AddressPath::max_depth_for_step` computes it for any step so a caller can find out before walking.

An over-long path is **refused**, never overflowed — an infinite address is not a location.

## Traversal order is part of the address

`⊗` is strongly non-associative, so the fold must be fixed. Measured on the path `1 → 2.0 → 1.5`:

```
left  ((a⊗b)⊗c) =  303.23
right (a⊗(b⊗c)) = 3373.00     ratio 11.1
```

`resolve()` folds **left** and says so. `resolve_right` exists only so the suite can prove the difference; its docs forbid addressing with it.

## A thin sabotage result, and what it revealed

Flipping the fold caught **exactly one test**. That is thin for a load-bearing decision, and the reason turned out to be a property worth recording:

**`⊗` is commutative but not associative.** It depends on `a·b`, which is symmetric — so a path of *identical* steps folds the same either way and cannot distinguish the orders. Most paths in the suite used uniform steps.

Added `otimes_is_commutative_but_not_associative` (asserting both halves explicitly) and `left_fold_addresses_are_pinned` (three distinct-step paths, each cross-checked against an explicitly computed left fold). Re-running the sabotage: **1 failure became 3**.

## Fragmentation is zero structurally, not by good housekeeping

Gauss–Bonnet fixes a `{5,4}` cell's area at exactly `π/2` from its angles alone, with no free scale parameter. Storage is therefore quantised into **identical** units.

There are no partial cells, so no gap smaller than a cell can exist, and a gap of one or more cells is simply free space. `fragmentation()` returns `0.0` and always will — that is geometry, not an allocator invariant being maintained.

`area_is_history_independent` makes it concrete: an allocation churned through 30 grow/shrink cycles has byte-identical area to a fresh one of the same cell count. A block allocator reports non-zero as soon as anything is freed.

## Where this lives, and where it doesn't

`lattice` owns PRD §5 per `_spec/architecture-map.md`, so addressing lives here. `substrate::LatticeAddress` wraps `lattice`'s `CellId`; the wiring between the two is now built — see the addendum below.

## Still not built

- **`⊘` as a path inverse.** Already recorded as not a true inverse, so unwinding a path needs a numerical solve.

---

# Addendum — scalar → cell resolution and the `MemoryPool` join

Closes the two items this section used to list as open: `AddressPath` resolved only to a `LatticeScalar`, with nothing mapping that scalar back to a `CellId`, and `substrate::MemoryPool` tracked cells directly rather than resolving `AddressPath`s.

`Tiling::nearest_cell` finds the grown cell closest to an arbitrary point, the same distance-scan shape `cells_at_vertex` already used. `AddressPath::resolved_point` reuses `Isometry::translation` — already the tiling's own convention for a signed point on the canonical x-axis geodesic through the origin — to turn a resolved scalar's sign and magnitude into a point; `resolve_to_cell` composes the two. No new geometry: a resolved scalar is one real number, so it can only ever address one geodesic through the 2D tiling, stated as the honest limit of what "the tiling names cells with points, addressing computes a scalar" can mean here, not a shortcoming worked around.

`substrate::MemoryPool::resolve_path` is the other half: it calls `resolve_to_cell` against the pool's own tiling, then checks the resulting cell is actually one the pool holds a slab for — a resolvable point in the address space is not automatically part of any one pool's backing store, so this is a genuine second check, not a formality. Verified against real magnitude and sign progression (positive/negative scalars land on different cells; increasing magnitude steps outward through successive rings) before any test was written. Full detail and the sabotage gate for both are in `neos/tests/lattice_addressing.rs` and `neos/tests/substrate.rs`, and in root `CONTEXT.md`'s cross-cutting-slices list.

## Human check

Read `step_magnitude_decides_depth` and `area_is_history_independent`. The first is the constraint the PRD does not state — four levels with unit steps. The second is its claim of "eliminating disk fragmentation entirely", made concrete and true for a stated geometric reason.

---

# Addendum — `solve_otimes`, a numerical inverse for (x)

Closes the item `oslash`'s own doc comment had been flagging since it was written: `oslash` is explicitly not a true inverse of `otimes`, and nothing filled the gap.

## Derivation history

Four scratch-harness iterations (`solve.rs` -> `solve2.rs` -> `solve3.rs` -> `solve4.rs`), run outside the crate before any real code was written, per the project's standing rule for numerical claims:

1. **Naive Newton's method** — diverges approaching the `(x)` domain edge, where `sinh`'s derivative grows fast enough that Newton steps overshoot past the bracket entirely, even though the true root is well-defined there.
2. **Bisection with an output-space acceptance test** (`|f(x) - target| <= eps * target`) — produced spurious "did not converge" refusals near the domain edge, where even the mathematically exact `x` cannot reproduce an astronomically large `target` to tight relative precision in `f64`, because `sinh` is locally so steep there that a tiny change in `x` swings the output by many orders of magnitude. Replaced with a bracket-width test in **x-space** (`(hi - lo) <= eps * scale`), which bisection shrinks reliably regardless of how steep `f` is.
3. **A real orientation bug**, found and fixed at this stage: for negative `a`, the draft set `(lo, hi) = (edge, -edge)` — i.e. `lo > hi` numerically — which produced catastrophically wrong, target-independent recovered values for every negative-`a` case tried. Fixed by always keeping `lo = -edge < hi = +edge` fixed, and reading the increasing/decreasing direction directly from `f_hi > f_lo` rather than from the sign of `a`. This one case-split removal is what makes the final algorithm handle both signs of `a` identically.
4. **Final verified form** — swept `a` from `0.01` to `50` (both signs) and `x` from `0.01` to `100` (both signs), including targets reachable only within 1 ulp of the domain edge: **zero hard failures across 117 cases, worst relative error 4.4e-14**. Only then was `LatticeScalar::solve_otimes` written into `metric.rs`, with the verified sweep numbers cited directly in its doc comment.

Two new `LatticeError` variants carry the two ways a solve can be legitimately refused rather than guessed: `DegenerateInverse` (`a` effectively zero — `0 (x) x = 0` for every `x`) and `UnreachableTarget` (no `x` inside `(x)`'s domain from this `a` reaches `target`), matching how `otimes` itself refuses on domain violation rather than saturating.

## Doctrine checks — three performed

| Sabotage | Failures |
|---|---|
| Bisection direction inverted (`increasing == (f_mid < 0.0)` flipped) | **3 of 30** |
| Bracket-width convergence tolerance halved before the loop starts | **2 of 30** |
| Degenerate-`a` guard (`a.abs() < 1e-12` check) removed entirely | **1 of 30** — and confirmed to fail *safely* even without the guard: `edge` computes as `+-inf` from division by a near-zero `a`, so the bracket immediately produces a non-finite `f(lo)`/`f(hi)` and the solve still returns `Err`, just as `UnreachableTarget` instead of the more honestly-named `DegenerateInverse`. Not a correctness gap, a naming one — the guard was kept for the more precise error variant, not because removing it would silently return a wrong answer. |

All three reverted after confirming failure for the expected reason; suite re-confirmed at 30/30 in `lattice_metric.rs`, 385/385 workspace-wide.

## Human check

Read `solve_otimes_stays_accurate_at_the_domain_edge` and the degenerate-`a` sabotage row above. The test is the sharpest case bisection has to handle; the sabotage row is the reminder that "zero failures" from removing a guard needs the same scrutiny as any other doctrine-check result, not an automatic pass.

---

# Addendum — a lattice-owned, general-purpose path-finder

Closes the item every stage since `02_design` carried forward verbatim: "the tiling now supports [geodesic path-finding]; it is not built." `pathfinding::shortest_path`/`shortest_distance` (`neos/lattice/src/pathfinding.rs`) are exact breadth-first search over a `Tiling`, returning the real cell sequence between any two cells.

## Deliberately not `ftg`'s router

`ftg::layers_3_4::Router` already does its own descent over lattice primitives, but it is not a general path-finder and was never meant to be one: forwarding there is memoryless metric descent, one hop at a time, no routing table, no stored path — a physics-motivated transport constraint (real packet forwarding has no global view of the network), not an implementation shortcut. Its own docs say plainly that a patch with holes could strand a packet, and it carries a private `bfs_hops` used only by its own test suite, explicitly never called by routing itself.

`lattice::pathfinding` is the general tool that private utility was standing in for: robust to incomplete or irregular patches, for any caller — not just `ftg` — that needs a real, guaranteed path rather than a physically-constrained forwarding decision. BFS is exact here, not a heuristic: `tessellation::centre_separation()` is one fixed edge length for the entire `{5,4}` tiling, so minimising hop count and minimising total geodesic length are the same problem.

## Verification: cross-checked against an independent implementation, not just self-tested

Rather than trust a single BFS implementation, `neos/tests/ftg.rs` gained `lattice_shortest_distance_agrees_with_ftgs_own_bfs`, comparing `lattice::shortest_distance` against `ftg::Router::bfs_hops` — two separately written breadth-first searches, over the identical tiling, agreeing exactly across 200+ sampled cell pairs. `lattice`'s own suite additionally uses the tiling's own ring structure as ground truth (`distance_from_origin_matches_ring_depth`: every cell in ring `n` must be exactly `n` hops from the origin, by definition of what a ring is), independent of any BFS implementation detail.

## Doctrine check

| Sabotage | Result |
|---|---|
| Queue order flipped from FIFO (`push_back`) to LIFO (`push_front`), turning BFS into DFS | **1 of 20** failed in `lattice_tiling.rs` (`distance_from_origin_matches_ring_depth` — reported 34 hops for a cell actually 2 rings out); **1 of 34** failed in `ftg.rs`'s independent cross-check, for the same underlying reason, caught a second, unrelated way |

Reverted after confirming; full workspace re-run clean at 407/407.

## Human check

Read `distance_from_origin_matches_ring_depth` in `neos/tests/lattice_tiling.rs` and `lattice_shortest_distance_agrees_with_ftgs_own_bfs` in `neos/tests/ftg.rs`. Together they're the same claim checked two structurally different ways — one against the tiling's own definition of a ring, one against a second, independently-written search — which is why the sabotage above shows up as two separate, unrelated-looking failures rather than one.

---

# Addendum — `tetryen_node_envelope`, relocated in rather than added new

Added `neos/lattice/src/tetryen.rs`: `tetryen_node_envelope(r) = sinh(r/R)·e^(−r/R)` (`R=1` in lattice-native units), `_mkb/tetryen.md`'s node standing-wave form.

This is a **move**, not new physics. The formula already existed and was already tested, inside `gui::renderer::Tetryen::node_amplitude`. It moved here because `crystallisation::timecrystal::TetryenRecurrence` needed the identical weight for its own implementation of [`_mkb/tetryen_recurrence.md`](../../../../_mkb/tetryen_recurrence.md), and `crystallisation` cannot depend on `gui` — dependency direction runs the other way (`crystallisation → {lattice, substrate} → gui`, not through it). `lattice` is the one crate both already depend on, so the fact moved down to it rather than being copied into `crystallisation` or a dependency cycle being introduced.

`gui::renderer::Tetryen::node_amplitude` is now `lattice::tetryen_node_envelope(r)`, a one-line delegation. No new test was written for the function in isolation — its existing coverage (`gui`'s `node_amplitude_follows_the standing_wave` and the whole downstream `TetryenState` suite) re-ran unchanged against the relocated implementation and stayed green (37/37 in `gui`), which is the same evidence a dedicated lattice-side test would have produced, without duplicating it.

## Human check

Diff `neos/gui/src/renderer.rs`'s `node_amplitude` before and after — it should be exactly the one-line delegation, nothing else changed in that file's behaviour.
