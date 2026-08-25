---
type: implementation-log
subsystem: gui
stage: 04_implement
status: complete
toolchain: rustc 1.97.1 / cargo 1.97.1
result: 64 passed, 0 failed (418 workspace-wide) — see Tetryen recurrence addendum
consumes: [lattice]
---

# GUI — Implementation Log

## Result

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 169 passed; 0 failed
                           38 lattice · 24 substrate · 52 symphony-kernel
                           29 ftg · 26 gui
```

Tests were written before the implementation. 26/26 passed on the first run.

## Files written

| Path | Role |
|---|---|
| `neos/gui/Cargo.toml`, `build.rs` | manifest; constants from `_mkb/constants.json` |
| `neos/gui/src/lib.rs` | crate root, `GuiError` |
| `neos/gui/src/ball.rs` | H⁴ isometries in the hyperboloid model |
| `neos/gui/src/renderer.rs` | Tetryen construction, geodesic edges |
| `neos/gui/src/fractal.rs` | navigation as hyperbolic translation |
| `neos/gui/src/visualization.rs` | standing waves for load and traffic |
| `neos/tests/gui.rs` | 26 assertions |

## A layering decision worth recording

**H⁴ isometries did not exist anywhere.** `lattice` provides the 4-ball metric (`PoincarePoint::distance_to`) but its `Isometry` type is **3×3 — isometries of the hyperbolic plane**, built for the `{5,4}` tiling, which tessellates H². The Tetryen lives in the 4-ball.

They are implemented in `gui/src/ball.rs` rather than added to `lattice`, and the module says so explicitly: this is new geometry, not a duplicate — the metric itself is still consumed — and **if a second subsystem ever needs H⁴ isometries, the module should move to `lattice`**, which is its natural home.

The alternative was reopening a completed record for a reuse that has not happened. Recorded as a seam rather than resolved speculatively.

## Two things the design refuses to offer

**No straight-line interpolator.** `GeodesicEdge::point_at` computes in the hyperboloid model, `γ(t) = u cosh t + w sinh t`. There is no `chord_at` and no `straight: bool` flag, because a straight edge is not a lower-quality option — it is a different, wrong geometry. Lerping ball coordinates is the natural thing to write by accident, which is exactly why the API makes it unavailable.

**No `zoom`.** Navigation is `Viewport::translate`, a hyperbolic isometry. Offering a scale factor would invite the Euclidean operation that destroys the property "infinite resolution scaling" names. Moving closer is translating.

## Tolerances that could not be machine epsilon

`acosh(x)` has unbounded derivative as `x → 1`, so `acosh(1 + ε) ≈ √(2ε)`. A `1e-16` representation error in the hyperboloid constraint surfaces as **`~1e-8`** in a distance between near-coincident points. Measured floor: `2.11e-08`.

The geodesic tolerance is therefore `1e-7`. That is not slack — demanding `1e-15` would produce a test failing for reasons unrelated to the renderer. It remains sharp because the thing it must reject, a Euclidean chord, misses by `3.2e-03` to `4.5e-03`: **five orders of magnitude above the floor.**

The isometry tolerance is a **function**, `isometry_floor(distance)`, for the same reason `ftg::cancellation_floor` is. Measured distance drift under translation: `2.2e-16` at 0.5, `1.7e-15` at 1.0, `5.4e-14` at 3.0, `3.2e-11` at 6.0 — roughly exponential, which is what `cosh` growth implies. Ball coordinates crowd toward `‖u‖ = 1` as the view moves out (measured max norm `0.9965` at translation 6.0).

This is the third subsystem where a constant tolerance was the wrong shape. The pattern is now familiar enough to look for first.

## `E[Γ]` is constructed, not minimised

`_mkb/tetryen.md` characterises the Tetryen as the minimiser of `E[Γ] = ∫(K(s) + H(s)²)ds`. This crate **constructs** a shape satisfying the characterisation — regular, geodesic-edged — rather than solving the variational problem, which is a research task and not a rendering slice.

Said plainly in the type's doc comment so the code is not mistaken for a solver. If a future slice does minimise the functional, this construction is the thing to check it against.

## Doctrine checks — two performed

| Sabotage | Tests failed | Which |
|---|---|---|
| geodesic interpolation → straight-line lerp | **3** | on-geodesic sampling, all six Tetryen edges, midpoint bisection |
| navigation isometry → Euclidean scaling ×1.5 | **4** | distance preservation, rotation, composed moves, off-origin regularity |

Both reverted; suite re-confirmed at 169/169 with no markers and no hardcoded MKB constants.

Note `projected_points_stay_in_the_ball` survived the scaling sabotage — scaling by 1.5 kept points inside for that Tetryen. It is a weaker guard than the distance tests and should not be read as covering navigation correctness.

## A vacuous assertion I wrote and removed

`opposed_phases_cancel_exactly` originally contained `assert_eq!(x.abs().min(0.0), 0.0)` — which is true for every `x`, since `abs()` is non-negative. It asserted nothing.

Replaced with a real bound across several instants plus a bit-exact check at `t = 0`. Worth recording because it passed, looked like coverage, and provided none.

## What is not built

- **Pixels.** No framebuffer, window, or GPU binding. This is the geometry layer a rasteriser consumes; the PRD specifies no rasteriser.
- **Face tessellation.** The six edges are built; filling the four curved faces needs a surface-subdivision scheme the corpus does not supply.
- **Live kernel binding.** `StandingWave::for_load` takes a number. Nothing yet reads `symphony-kernel`'s actual `LoadField`, or `ftg`'s traffic. That wiring is the natural next slice and would be the first genuinely end-to-end visualisation.
- **`E[Γ]` minimisation**, per above.

## Human check

Run `cargo test -p gui`. Read `euclidean_chord_is_not_a_geodesic` and `navigation_preserves_all_distances` — the first proves the renderer is hyperbolic by showing what a flat one fails, the second is what "infinite resolution scaling without pixelation" actually means: the observer moves, nothing is magnified.

---

# Slice 2 — live visualisation

Added `telemetry.rs`, `visualization::LoadVisualisation`, and `neos/tests/gui_telemetry.rs` (13 assertions). Workspace total: **206 passing**.

Closes the gap recorded at the end of slice 1: *"`StandingWave::for_load` takes a number. Nothing yet reads `symphony-kernel`'s actual `LoadField` or `ftg`'s traffic."*

`gui` now depends on `symphony-kernel` and `ftg`. Direction is safe — `gui` is the presentation layer and nothing depends on it, so no cycle. `lattice ← substrate ← {symphony-kernel, ftg} ← gui`.

## Normalisation is the design decision, not a detail

A 2 GHz task costs `E = C_H·ν ≈ 5.3e-25` J. **Mapping load to amplitude directly renders every core as visually zero** — the display would show an idle machine under full load.

`normalised_load()` scales against the busiest core, giving `[0,1]`. Raw joules stay available for numeric readouts; nothing draws from them.

This is the **third** appearance of the absolute-vs-relative trap — after the false convergence in `symphony-kernel` and the flaky tolerance in `ftg`. First time it was designed for in advance rather than discovered by a failure.

## A vacuous test of mine, caught by measurement

`visualisation_flattens_as_the_field_equilibrates` originally asserted `spread_after <= spread_before`. It passed. Measuring showed why it proved nothing: **imbalance was 0.250000 before and after** — identical.

The cause is real and worth knowing. 60 identical tasks across 16 cores is 4/4/…/3/3/3/3, which is already optimal: migration moves whole tasks, and a task cannot be split. Nothing *could* improve. The `<=` hid that.

Rewritten with **varied task frequencies**, where diffusion genuinely helps — measured imbalance 0.3478 → 0.2857 — and asserted with a strict `<` plus a 0.01 margin. Two companion tests pin the floors so the strict inequality stays honest:

- `quantisation_limited_field_does_not_improve` — an already-optimal field must stay put
- `migration_cannot_fill_an_idle_core` — 5 tasks on 16 cores is pinned at imbalance 1.0; diffusion redistributes work, it does not create it

## The guard test had fallen into the trap it guards

`visualisation_is_scale_free` **passed under the normalisation sabotage**, which it should not have.

It compared absolute amplitudes with an absolute tolerance. Under raw joules both sides are ~1e-25 and ~1e-23, so their difference sits far below any sensible absolute threshold — the test written to catch the absolute-vs-relative trap was itself absolute.

Fixed with a relative comparison plus an order-1 magnitude check (`peak() > 0.1`), which is what "renderable at all" means. Re-running the sabotage then caught it: **3 failures became 4**.

That is the second time in this session a passing test turned out to assert nothing, after `x.abs().min(0.0) == 0.0` in slice 1. Both were found by sabotage rather than by review.

## Doctrine check

| Sabotage | Tests failed |
|---|---|
| draw raw joules, no normalisation | **4 of 13** (3 before the scale-free fix) |

## Traffic mapping is a stated assumption

Delivered packets contribute constructively, failures destructively, so `network_balance()` spans `+1` to `−1` with exact cancellation at zero. The PRD asks for "constructive and destructive energy states" but does not define the mapping — this one is **chosen, not derived**, and the module says so.

`SystemSnapshot::observe` consumes `ftg`'s real `Delivery` values, so the three outcomes cannot drift out of step with transport. `link_loss_is_counted_apart_from_dissipation` confirms slice 3's distinction survives into the display: a stranded packet is not miscounted as corruption.

## Still not built

- **Rasterisation.** Still no framebuffer; this remains the geometry and telemetry layer.
- **Memory-usage visualisation.** PRD §9 names load, memory, *and* traffic. `substrate::MemoryPool` exposes `available()` and `total_capacity()`, but the snapshot does not read them yet.
- **Continuous refresh.** A snapshot is point-in-time; nothing polls.

## Human check

Read `visualisation_flattens_as_the_field_equilibrates` alongside `quantisation_limited_field_does_not_improve`. The first shows the display tracking real equilibration; the second explains why the first needed varied task frequencies to say anything at all.

---

# Addendum — the Tetryen recurrence (`evolution::TetryenState`)

Closes a placeholder the undistilled corpus (`_mkb/papers/The neccessity of a finite universe.pdf`) names but never defines: `ψ_{n+1} = f(ψ_n, ψ_{n-1})`, "the governing equation for Tetryen emergence." The full paper was read and evaluated equation by equation before any code was written — every other formula in it restates law already distilled elsewhere with zero new content, and `f` itself was never given a body. Full account in `_mkb/papers/_index.md`'s own entry for that paper.

## The proposal that was rejected first

A detailed proposal arrived in conversation offering to "define `f` explicitly," citing `ξ(r) = 2r/(1+r²)` as "the settled MKB attenuation factor," a "hyperbolic graph Laplacian," and a coherence order parameter `𝒞 = 1 - |Σsin(φ_k)|/4`, plus working Rust. Checked against the actual law before writing anything: the real `ξ(r)` (`resonance.md`, `equations.md`) is `sinh(r/R)/((r/R)sinh(1))·e^{1-r/R}` — bounded *above* by `e/sinh(1)≈2.313` as `r→0`, where the proposed form gives `0` — a different function, not a rounding difference. The only graph Laplacian in `_mkb/` is the core-topology load balancer (`resonance.md §2`, a different graph entirely). The coherence formula appears nowhere in the corpus. The supplied Rust called a `PoincarePoint::r()` method that doesn't exist and iterated `Tetryen::edges()` without any way to recover which node indices each edge connects. None of it was implemented.

## What was actually built, and how each piece was checked before use

- **The uncoupled step is an exact discrete identity**, not a physical claim: `ψ_{n+1}+ψ_{n-1} = 2cos(ωΔt)ψ_n`, the sum-to-product identity `cos(A+B)+cos(A-B)=2cos(A)cos(B)`. Verified in a disposable scratch example (`neos/gui/examples/scratch_tetryen_recur.rs`, deleted after use) before writing the real module: worst error `5.7e-14` over 10,000 samples.
- **The coupling weight is real law, reused exactly where it fits**: `tetryen.md`'s own node standing-wave envelope, `Tetryen::node_amplitude`, evaluated at the real geodesic distance `PoincarePoint::distance_to` between two nodes — not the misattributed `ξ(r)`, not an invented Laplacian.
- **A structural fact, checked rather than assumed**: every Tetryen this crate constructs is regular (translations are isometries), so every pairwise coupling weight is identical. Checked directly at circumradius `0.5`: `d_H=0.827162`, `node_amplitude=0.404389` for all 12 ordered pairs.
- **Stability was measured, not derived in closed form** — unlike `resonance.md`'s `α<2/λ_max(L)` bound, which *is* derived, this 4×4 coupling matrix's exact spectral bound was not attempted. Swept empirically instead: bounded for `γ` up to `1e4` at `Δt=0.01`; genuinely diverges to `inf` at `γ≥1e5` (same `Δt`), or at `Δt≥1.0` (fixed `γ=1`) — the instability is real and was found, confirming these parameters are load-bearing, not decorative.
- **A step leaving that region is refused, not propagated**: `TetryenState::step` returns `GuiError::Diverged` on a non-finite result rather than returning it, the same discipline `lattice::LatticeScalar::otimes` applies at its own domain edge.

## What is deliberately not built

An "emergence" gate. Searched `tetryen.md` and `timecrystal.md` directly — neither defines a coherence threshold, criterion, or even an informal description of when a Tetryen should be considered to have "emerged." The rejected proposal's coherence formula would have filled this gap with something unfounded; declining to invent a replacement is the point, not an oversight. Recorded as an open gap in `_mkb/tetryen_recurrence.md` itself.

## Doctrine checks — two performed

| Sabotage | Result |
|---|---|
| Coupling sign flipped (`ψ_i - ψ_j` instead of `ψ_j - ψ_i`, anti-diffusive) | **1 of 37 failed** — `coupling_pulls_differing_nodes_toward_each_other`: the outlier node moved *away* from its neighbours instead of toward them |
| The `is_finite()` divergence guard removed | **1 of 37 failed** — `a_step_leaving_the_stability_region_is_refused_not_propagated`: 5,000 steps at `γ=1e6` completed with `Ok` throughout instead of ever returning `Err`, even though the underlying values had gone non-finite |

Both reverted after confirming; full workspace re-run clean at 418/418.

## Human check

Read `identical_nodes_evolve_by_the_uncoupled_identity_regardless_of_coupling` first — it isolates the uncoupled formula from the coupling term by construction (equal-valued nodes make every coupling difference exactly zero, for any `γ`), so it tests the discrete-oscillator identity on its own before anything else is layered on top. Then read the module doc comment in `neos/gui/src/evolution.rs` next to `_mkb/tetryen_recurrence.md` side by side — every claim in the code doc traces to a specific section of the law file, which is the property that separates this from the proposal that was rejected first.

---

# Addendum — `node_amplitude` relocated to `lattice`

`renderer::Tetryen::node_amplitude`, the coupling weight `TetryenState::step` calls, is now `lattice::tetryen_node_envelope(r)` — a one-line delegation, not a rewrite. `crystallisation::timecrystal::TetryenRecurrence` needed the same law-sourced weight for its own implementation of `_mkb/tetryen_recurrence.md`, and `crystallisation` cannot depend on `gui` (dependency direction runs the other way), so the fact moved down to `lattice`, the one crate both already depend on — one home per fact rather than a second copy. Full account is in `lattice`'s own implementation log addendum.

No behaviour changed: the existing suite (37/37, including `node_amplitude_follows_the_standing_wave` and every `TetryenState` test that exercises coupling) re-ran unmodified and stayed green after the move.
