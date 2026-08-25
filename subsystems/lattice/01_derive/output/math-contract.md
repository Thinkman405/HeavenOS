---
type: math-contract
subsystem: lattice
stage: 01_derive
derived_from: ["_mkb/axioms.md", "_mkb/reconciliation.md", "_mkb/operators.md", "_mkb/equations.md", "_mkb/constants.json"]
---

# Lattice — Math Contract

The complete law binding `neos/lattice`. Everything in `02_design` must trace to a line here. Formulas are copied, not paraphrased.

## 1. Binding axiom

**A3 — Spatial Addressing Override.** Cartesian coordinate arrays are replaced by non-Euclidean hyperbolic metric spaces.

*Directive:* no flat indexed arrays for addressable space. Address resolution goes through the hyperbolic distance function against the `{5,4}` tessellation.

A1 and A2 do not bind lattice directly. A1's operator consequence reaches lattice through ⊗ (§3), but lattice does no process forking; A2's phase logic belongs to symphony and ftg.

## 2. Unit convention

All lattice code works in **lattice-native units**: `R = 1`, therefore `K = -1/R² = -1`.

Physical embedding (`R = 3.6e-10 m`) is a conversion at the boundary and never changes `K`. Per [reconciliation.md R2](../../../../_mkb/reconciliation.md).

Consequence: the Poincaré disk distance function in §4 is valid as written. It would not be at any other `K`.

## 3. The ⊗ operator

$$a \otimes b = a\,b + \sinh(a\,b\,\lambda), \qquad \lambda = \operatorname{arcsinh}(1) = 0.881373587019543$$

**Execution rule:** all lattice address arithmetic uses ⊗, never scalar `*`. Scaling a stored object triggers geometric fractal expansion preserving logical area.

### 3.1 Exactness

`1 ⊗ 1 = 1 + sinh(λ) = 1 + 1 = 2` **bit-exactly** in IEEE-754. Verified: `otimes(1.0, 1.0) == 2.0` is `True`.

Assert with exact equality. **Do not** wrap this in an epsilon comparison — doing so would hide a regression that a strict check catches.

### 3.2 Domain — hard limit

$$a\,b < \frac{710}{\lambda} \approx 805.5607865456228$$

Above this, `sinh` overflows `f64` and ⊗ returns `+inf`. **Enforced by a checked constructor.** A ⊗ that silently returns infinity is a defect.

### 3.3 Non-associativity — structural, not a rounding artefact

At the pinned scale `l_P/r = λ ≈ 0.881`, order 1. The `O(l_P²/r²)` bound from `Mathematical_Fra.pdf` assumes `r ≫ l_P` and does **not** apply here. Measured:

```
(2⊗3)⊗4 = 2.864e160
2⊗(3⊗4) = +inf   (overflow)
```

Forbidden:
- reordering or re-associating any ⊗ chain
- `fold`/`reduce` over a collection with ⊗ unless association order is fixed and documented
- implementing `std::ops::Mul` for lattice scalars — `Mul` carries an associativity expectation that ⊗ violates

### 3.4 Related operators

`⊘` (division) is **not** a true inverse: `(a ⊗ b) ⊘ b ≠ a` in general. `a^{⊗n} ≠ a ⊗ a ⊗ …` — powers are a separate definition. Neither is required by this build; both are recorded so no one assumes otherwise.

## 4. Hyperbolic distance

$$d_{\mathbb{H}}(\mathbf{u}, \mathbf{v}) = \operatorname{arcosh}\left(1 + \frac{2\Vert{}\mathbf{u} - \mathbf{v}\Vert{}^2}{(1 - \Vert{}\mathbf{u}\Vert{}^2)(1 - \Vert{}\mathbf{v}\Vert{}^2)}\right)$$

Geodesic distance in the Poincaré disk model of the `{5,4}` tessellation.

**Domain:** `‖u‖ < 1` and `‖v‖ < 1` strictly. The disk boundary is at infinite distance — a point with `‖u‖ ≥ 1` is not a point of the space. This is a **representable-state invariant**, not a validation nicety: it must be impossible to construct such a point.

**Required properties** (these are what the test suite proves):
- `d(u,u) = 0`
- symmetry: `d(u,v) = d(v,u)`
- triangle inequality: `d(u,w) ≤ d(u,v) + d(v,w)`
- `d → ∞` as either point approaches the boundary

## 5. Tessellation

**Schläfli `{5,4}`** — pentagons, **four** meeting at each vertex. Vertex degree is exactly 4.

Hyperbolicity check: `(p−2)(q−2) = 3 × 2 = 6 > 4` ✓.

Per [reconciliation.md R3](../../../../_mkb/reconciliation.md): `vACUUM_FLUX.pdf`'s prose says "five pentagons meet at each vertex", contradicting its own `{5,4}` notation in the same sentence. The notation won.

Vertex degree is asserted directly in the test suite — it determines the entire adjacency structure.

## 6. Forbidden constructs

Each traces to an axiom or reconciliation row:

| Forbidden | Because |
|---|---|
| flat indexed arrays for addressable space | A3 |
| scalar `*` on lattice addresses | A3 / §3 |
| `impl Mul` for lattice scalars | §3.3 |
| assuming `(a⊗b)⊗c == a⊗(b⊗c)` | §3.3 |
| constructing a point with `‖u‖ ≥ 1` | §4 |
| unchecked ⊗ where `a·b` may exceed 805.56 | §3.2 |
| hardcoding any numeric constant in `.rs` | one home per fact |
| `assert_eq!(x, true)` style assertions | test doctrine |

## 7. Constants consumed

Read via `build.rs` from `_mkb/constants.json`. **Never retyped.**

| JSON path | Use |
|---|---|
| `constants.hyperbolic_curvature.value` | `K = -1` |
| `operators.otimes_scale_lambda.value` | `λ` |
| `operators.otimes_domain_max_product.value` | domain guard |
| `tessellation.schlafli` / `.vertex_degree` | topology |
| `scales.lattice_scale_R.value` | `R = 1` native |

## 8. Open questions

**None blocking.** R5 (Howard Comma) and R6 (Harmonic Force Equilibrium) remain open but bind `symphony` only; lattice reads neither `C_H` nor any scheduling equation.

This subsystem is fully derivable from settled law. That is why it was chosen first.
