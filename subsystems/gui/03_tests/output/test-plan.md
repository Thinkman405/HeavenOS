---
type: test-plan
subsystem: gui
stage: 03_tests
derived_from: ["../02_design/output/design.md", "../01_derive/output/math-contract.md"]
doctrine: _mkb/test-doctrine.md
---

# GUI — Test Plan

Target: `neos/tests/gui.rs`. Values measured before writing.

**[D]** marks assertions a conventional Euclidean renderer could not pass — one drawing straight edges and scaling to zoom.

## Group 1 — geodesic edges

The defining constraint. A straight-edge tetrahedron is a defect.

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 1.1 **[D]** | sampled points lie **on** the geodesic | `d(u,p)+d(p,v) = d(u,v)` | `1e-7` | The sharp membership test — off-geodesic the triangle inequality is strict. Measured floor `2.1e-08`; see the tolerance note below. |
| 1.2 **[D]** | a **Euclidean chord fails** the same test | excess `> 1e-4` | none | Measured `+3.2e-03` to `+4.5e-03` at ¼, ½, ¾ — five orders above the floor. This is the assertion that separates a hyperbolic renderer from a flat one. |
| 1.3 | endpoints are exact | `point_at(0) = u`, `point_at(1) = v` | `1e-7` | — |
| 1.4 | midpoint bisects | `d(u,mid) = d(mid,v) = d/2` | `1e-9` | — |
| 1.5 | `length` matches the metric | equals `lattice` distance | `1e-12` | Cross-checked against `lattice`, not recomputed. |
| 1.6 | coincident endpoints refused | `Err(DegenerateEdge)` | none | `sinh(d) = 0` divides; without the guard this is a silent `NaN` in the scene. |
| 1.7 | sampling is monotone | distance from `u` increases | none | Catches an interpolator that doubles back. |

## Group 2 — the Tetryen

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 2.1 | exactly four nodes, six edges | structural | none | Enforced by array types; asserted so the intent is recorded. |
| 2.2 | all six edges equal | spread `0` | `1e-12` | Measured spread **exactly 0** at three circumradii. |
| 2.3 | nodes equidistant from centre | spread `0` | `1e-12` | Measured exactly 0. |
| 2.4 **[D]** | every edge is geodesic | membership holds on all six | `1e-7` | Group 1 applied to the real primitive. |
| 2.5 | node amplitude follows `ψ(r)` | `A sinh(r)e^(−r)` | `1e-12` | From `_mkb/tetryen.md`. |
| 2.6 | `ψ(0) = 0` | exact | none | `sinh(0) = 0`. A node at zero radius has no amplitude. |
| 2.7 | invalid radius refused | `Err(InvalidRadius)` | none | Zero, negative, NaN, infinite. |
| 2.8 | a Tetryen can be placed off-origin | regular at a translated centre | `1e-9` | Regularity is a property of the shape, not of its position. |

## Group 3 — fractal navigation

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 3.1 **[D]** | translation **preserves all distances** | unchanged | `isometry_floor(d)` | The heart of "infinite resolution". Measured `2.2e-16` at distance 0.5 rising to `3.2e-11` at 6.0. |
| 3.2 **[D]** | a Euclidean scaling would **not** | scaled distances differ | none | The contrast made explicit: scaling by `k` multiplies distances by `k`, so it is a different operation, not a cheaper one. |
| 3.3 | the floor **scales** with distance | far ≫ near | none | Pins why a constant cannot be used, as in `ftg`. |
| 3.4 | rotation preserves distances | unchanged | `1e-12` | Rotations stay near the origin, so no scaling needed. |
| 3.5 | composition stays isometric | after 5 moves | `isometry_floor(total)` | Errors must not compound into distortion. |
| 3.6 | projected points remain in the ball | `‖u‖ < 1` | none | An isometry cannot push a point out of the space. |

## Group 4 — interference visualisation

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 4.1 | zero load renders zero amplitude | `0.0` | exact | The display cannot show activity where there is none. |
| 4.2 | amplitude is linear in load | `2×` load → `2×` peak | `1e-12` | Measured `0.0 / 0.932 / 1.864`. |
| 4.3 | wave matches `2A sin(kx)cos(ωt)` | closed form | `1e-12` | — |
| 4.4 **[D]** | opposed phases cancel **exactly** | `0.0` | **none** | Destructive interference is total. A renderer that merely dimmed overlapping waves would not reach zero. |
| 4.5 | aligned phases reinforce | `2×` single | `1e-12` | — |
| 4.6 | classification matches phase delta | Constructive / Destructive | none | — |

## Group 5 — consumed, not reimplemented

| # | Assertion | Justification |
|---|---|---|
| 5.1 | edge length equals `lattice`'s metric | Cross-checked directly, as `substrate` and `ftg` do. |
| 5.2 | curvature is `lattice`-native `K = −1` | The metric is only valid there. |
| 5.3 | no MKB constant literal in `gui` source | `grep` in verification. |

## Tolerance notes

**The geodesic tolerance is `1e-7`, and that is not slack.** `acosh(x)` has unbounded derivative as `x → 1`, so `acosh(1+ε) ≈ √(2ε)`: a `1e-16` representation error surfaces as `1e-8` in a distance between close points. Demanding `1e-15` would produce a test that fails for reasons unrelated to the renderer.

It is still a strong test, because the thing it must reject — a straight edge — misses by `3e-3`, **five orders of magnitude** above the floor.

The isometry tolerance is a **function**, not a constant, for the same reason `ftg::cancellation_floor` is.

## Deliberate omissions

Pixels, `E[Γ]` minimisation, face tessellation, and live kernel binding — all recorded in the design with reasons.

## Human check

Read 1.2 and 3.1. The first proves the renderer is hyperbolic rather than flat, by showing what a flat one would fail. The second is what "infinite resolution scaling without pixelation" actually means: the observer moves, nothing is magnified.
