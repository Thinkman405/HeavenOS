---
type: math-contract
subsystem: symphony-kernel
stage: 01_derive
derived_from: ["_mkb/axioms.md", "_mkb/resonance.md", "_mkb/reconciliation.md", "_mkb/equations.md", "_mkb/constants.json"]
consumes: [lattice]
---

# Symphony-kernel — Math Contract

The complete law binding `neos/symphony/kernel`. Formulas are copied, not paraphrased.

## 1. Binding axioms

**A1 — Multiplicative Identity Override.** `1 × 1 = 2`. Process forking is a structural geometric split, not scalar duplication. A fork of one unit task yields **two** child units and **two** units of address space.

**A2 — Logic Gate Override.** Boolean truth is deprecated; logic is phase orientation `φ ∈ {−π/2, +π/2}`. Branch evaluation is phase alignment, not `bool` comparison.

A3 binds only through the `lattice` crate, which this subsystem consumes rather than reimplements.

## 2. Energy quantization

$$E = C_H\,\nu, \qquad C_H = \frac{h}{\sqrt{2\pi}} = 2.6434195357408632 \times 10^{-34}\ \text{J·s}$$

**Execution rule:** garbage collection monitors `ν`. When `ν → 0`, `E → 0`, and the memory vector is unmapped. Priority is an energy state, not a queue position.

### 2.1 — The `ν` / `ω` hazard

`C_H` pairs with **ordinary frequency**. A "1 GHz" process has `ν = 1e9`, `ω = 2π×1e9`.

Substituting one for the other scales every energy computation by `2π`, and **the units do not catch it** — both are J·s × s⁻¹ = J. Therefore:

- `Frequency` (ordinary, Hz) and `AngularFrequency` (rad/s) are **distinct types**
- `E = C_H·ν` accepts only the former
- `baseline_carrier_frequency` (`ω_c`) is angular, belongs to wave synthesis, and must be unrepresentable as an input here

This is the single most likely silent defect in the subsystem. It gets a type, not a comment.

### 2.2 — Deliberate departure from Planck

`E = C_H·ν` yields `0.3989×` the Planck energy `hν`. That is intended, not an approximation error. Do not "correct" `C_H` toward `h` or `ħ`.

## 3. Resonance correction

$$\xi(r) = \frac{\sinh(r/R)}{(r/R)\,\sinh(1)}\;e^{\,1-r/R}$$

**Execution rule:** multiplies the nominal frequency of a timing source at observation scale `r`. Never gates correctness — a failure to apply `ξ` degrades precision only.

| Property | Value |
|---|---|
| `ξ(R)` | exactly 1 |
| shape | strictly decreasing on `(0,∞)` |
| **supremum** | `e/sinh(1) = 2.3130352854993315`, approached as `r → 0` |
| `r → ∞` | `→ 0` |

**Boundedness is a safety requirement, not an observation.** Headroom must be sized for `2.3130`. Note the source rationale claims `ξ(0) ≈ 1`; it is 2.3130.

`r = 0` must be handled explicitly — the expression is `0/0` there and evaluates by limit to the supremum.

## 4. Timing convergence invariant

$$H(\kappa) = \int_0^t \gamma(\kappa,t')\,dt' \to 0$$

**Execution rule:** a **health invariant, not an input.** Integrate observed phase error across cores. If `|H(κ)|` grows rather than tending to zero, the correction has failed and the clock domain has diverged. Monitorable and alarmable.

## 5. Harmonic Force Equilibrium

$$\nabla \cdot \mathbf{E} = \frac{\rho}{\epsilon_0} \;\Longrightarrow\; L\varphi = -\frac{\rho}{\epsilon_0}$$

`ρ` is task density, `E` the processing capacity field, `L` the graph Laplacian of the core topology. Load flows down the gradient of `φ`.

### 5.1 — Solvability (mandatory)

$$\sum_i \rho_i = 0$$

`L`'s nullspace is the constants, so this is the condition for a solution to **exist**. Task density is `loadᵢ − mean(load)` — never absolute load.

### 5.2 — Stability (mandatory)

$$\alpha < \frac{2}{\lambda_{\max}(L)}, \qquad \lambda_{\max}(L) \le 2\,d_{\max}$$

The coupling `α = 1/ε₀` is derived from the topology. A hardcoded `α` oscillates at some core count — producing the thrashing the model exists to prevent.

### 5.3 — Conservation

The Laplacian is conservative (rows sum to zero), so total load is invariant under balancing. Assert it.

## 6. Topology comes from `lattice`

Cores map onto `{5,4}` cells. `Tiling`, `Cell::neighbors()`, and `CellId` are **consumed from the `lattice` crate**, which has them built and tested.

**Do not reimplement tiling, neighbour resolution, or the metric here.** One home per fact applies to code. Adjacency is a closed-form group operation — which is what makes naming neighbours free of runtime discovery.

Vertex degree 4 and 5 face-neighbours per cell are `lattice`'s guarantees; this subsystem reads them.

## 7. Forbidden constructs

| Forbidden | Because |
|---|---|
| `bool` in branch evaluation | A2 |
| scalar duplication on fork | A1 |
| passing `ω` where `ν` is required | §2.1 |
| absolute (un-centred) task density | §5.1 |
| hardcoded `α` | §5.2 |
| reimplementing tiling or metric | §6 |
| assuming no deadlock | §8 |
| hardcoding any constant from `constants.json` | one home per fact |

## 8. Scope boundary — deadlock

Load equilibrium eliminates **thrashing** and **bottlenecks**. It does **not** eliminate deadlock, which is a circular wait in resource *acquisition* — orthogonal to load distribution. Four perfectly balanced cores still deadlock on two locks taken in opposite orders.

**Deadlock detection is required.** A kernel built believing deadlock impossible hangs with no diagnostic.

## 9. Constants consumed

Read via `build.rs` from `_mkb/constants.json`. Never retyped.

| JSON path | Use |
|---|---|
| `constants.howard_comma.value` | `C_H` |
| `resonance.xi_at_reference.value` | `ξ(R) = 1` assertion |
| `resonance.xi_upper_bound.value` | headroom sizing |
| `resonance.diffusion_stability_factor.value` | the `2` in `α < 2/λ_max` |
| `logic_phases.*` | A2 phase pair |

## 10. Open questions

**None blocking.** R1–R6 are closed, with R5a/R5b recording the revised `C_H` and `ξ`.
