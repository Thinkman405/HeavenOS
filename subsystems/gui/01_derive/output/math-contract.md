---
type: math-contract
subsystem: gui
stage: 01_derive
derived_from: ["_mkb/axioms.md", "_mkb/tetryen.md", "_mkb/equations.md", "_spec/prd.md"]
consumes: [lattice]
---

# GUI — Math Contract

The complete law binding `neos/gui`. PRD §9: render the wave mechanics and spatial geometry of the kernel.

## 1. Binding axiom

**A3 — Spatial Addressing Override.** The scene lives in hyperbolic space. Projection to a screen is a change of chart, not a change of geometry.

A1 and A2 do not bind rendering.

## 2. The Tetryen — the rendering primitive

From [`_mkb/tetryen.md`](../../../../_mkb/tetryen.md): four nodes at the vertices of a tetrahedron, with edges and faces curved by the standing waves the nodes produce. Characterised as the minimiser of

$$E[\Gamma] = \int_\Gamma \left( K(s) + H(s)^2 \right)\,ds$$

with edges satisfying the geodesic equation.

### 2.1 What is implemented, and what is not

**Implemented:** the *characterisation* — four nodes, six geodesic edges, regular structure, node standing waves.

**Not implemented:** minimising `E[Γ]` numerically. Computing a variational minimum over a curved surface is a research problem, not a rendering slice. The Tetryen is **constructed** to satisfy the characterisation rather than **found** by minimisation.

Stated so nobody reads the code as solving the functional. If a future slice does minimise it, this record's construction is the thing to check against.

### 2.2 Regularity

Four nodes at equal hyperbolic distance from the centre, in directions forming a regular tetrahedron. Verified: all six edge lengths equal with spread **exactly 0** at circumradius 0.3, 0.842, and 1.5.

### 2.3 Node dynamics

$$\psi(r) = A\,\sinh(r/R)\,e^{-r/R}$$

In lattice-native units `R = 1`, so `ψ(r) = A·sinh(r)·e^(−r)`.

## 3. Edges are geodesics — the defining constraint

**A straight-edge tetrahedron is a defect, not a simplification.**

A point `p` lies on the geodesic between `u` and `v` exactly when

$$d(u,p) + d(p,v) = d(u,v)$$

Off the geodesic the triangle inequality is **strict**, so this is a sharp membership test.

**Execution rule:** every rendered edge is sampled along the hyperbolic geodesic, computed in the hyperboloid model as

$$\gamma(t) = u\cosh(t) + w\sinh(t), \qquad w = \frac{v - u\cosh(d)}{\sinh(d)}$$

where `w` is the unit tangent at `u` toward `v` and `t ∈ [0, d]`.

### 3.1 The test discriminates by five orders of magnitude — measured

| Sampled point | `d(u,p) + d(p,v) − d(u,v)` |
|---|---|
| on the geodesic | `≤ 2.1e-08` |
| Euclidean chord, ¼ | `+3.20e-03` |
| Euclidean chord, ½ | `+4.45e-03` |
| Euclidean chord, ¾ | `+3.51e-03` |

The chord is **strictly longer**, by ~10⁵× the numerical floor. A renderer that drew straight edges could not pass.

### 3.2 Why the floor is `1e-8` and not `1e-15`

`acosh(x)` has unbounded derivative as `x → 1`, so `acosh(1 + ε) ≈ √(2ε)`. A representation error of `~1e-16` in the hyperboloid constraint therefore surfaces as `~1e-8` in a distance between near-coincident points.

**Execution rule:** tolerances on hyperbolic distances between *close* points must be `~1e-7`, not machine epsilon. Choosing `1e-15` here produces a test that fails for reasons unrelated to the property under test. This is a property of the metric, not of the renderer.

## 4. Fractal navigation — zoom is an isometry

"Infinite resolution scaling: zoom into localized data nodes without pixelation."

**Execution rule:** navigation is a **hyperbolic translation**, which is an isometry. Distances between scene nodes are preserved exactly; the view moves without distortion, and detail does not degrade because nothing is being magnified — the observer is moving.

Verified: translating a Tetryen by 0.5, 1.0, 3.0, and 6.0 changes no pairwise distance by more than `2.2e-16`, `1.7e-15`, `5.4e-14`, `3.2e-11` respectively.

**A Euclidean zoom by factor `k` multiplies every distance by `k`.** That is the discriminating difference, and it is what "without pixelation" actually means here.

### 4.1 Isometry error grows with translation distance

The residual above is not constant — it grows as the view moves outward, because ball coordinates crowd toward `‖u‖ = 1` (measured max norm `0.9965` at translation 6.0).

**Execution rule:** the isometry tolerance must scale with translation distance, in the manner of `ftg::cancellation_floor`. A fixed bound taken from a short translation will fail on a long one.

## 5. Interference visualisation

$$f(t) = 2A\sin(kx)\cos(\omega_{sync} t)$$

System load, memory, and network traffic render as standing waves.

**Execution rule:** amplitude is proportional to the quantity visualised, so zero load renders zero amplitude — verified `0.0`, `0.932`, `1.864` at loads `0.0`, `0.5`, `1.0`.

Constructive and destructive states must be visually distinct: aligned phases superpose to `2.0`, opposed to **exactly `0.0`**.

## 6. Consumed, never reimplemented

| From | What |
|---|---|
| `lattice` | `PoincarePoint`, the hyperbolic metric, `Isometry`, the hyperboloid model |

The metric and the isometry group already exist. Neither may be rebuilt here.

## 7. Forbidden constructs

| Forbidden | Because |
|---|---|
| straight-line edges between scene nodes | §3 — a defect, not an approximation |
| Euclidean scaling for zoom | §4 — destroys the isometry |
| reimplementing the metric or isometries | §6 |
| `1e-15` tolerances on close-point distances | §3.2 — the metric cannot deliver it |
| claiming `E[Γ]` is minimised | §2.1 — it is constructed, not solved |
| hardcoding any constant from `constants.json` | one home per fact |

## 8. Constants consumed

| JSON path | Use |
|---|---|
| `constants.hyperbolic_curvature.value` | `K = −1`, validity of the metric |
| `scales.lattice_scale_R.value` | `R = 1` in node dynamics |

## 9. Open questions

**None blocking.** The `E[Γ]` minimisation is recorded as out of scope (§2.1) rather than open — it is a decision, not a gap.
