---
type: subsystem-law
layer: law
status: canonical
resolves: ["R5", "R6"]
---

# Resonance Law — Timing and Equilibrium

Closes [R5](reconciliation.md#r5--howard-comma) and [R6](reconciliation.md#r6--harmonic-force-equilibrium). Both were blocked not because the mathematics was missing, but because **one symbol was carrying three different jobs**. Separating the roles resolves the contradiction without discarding any source.

## Part 1 — The Howard Comma is three objects, not one

The papers use "Howard Comma" for three mathematically distinct things. They are not competing definitions of one quantity; they are different objects at different layers, and all three are true simultaneously.

| Role | Object | Type | Subsystem duty |
|---|---|---|---|
| **energy quantum** | $C_H = h/\sqrt{2\pi}$ | constant, J·s | discrete energy-bounded state transitions |
| **resonance correction** | $\xi(r)$ | dimensionless function of scale | clock jitter damping |
| **convergence invariant** | $H(\kappa) \to 0$ | residual of an integral | proof the correction is working |

The apparent contradiction — "`C_H = ħ`" versus "`H(κ) ≈ 0`" — dissolves once you see that `H(κ) ≈ 0` was never a claim about the *value* of `C_H`. It is the **residual after correction**: the accumulated phase error tends to zero *because* the damping works. A convergence criterion, not a constant.

This also retires the concern that `C_H ≈ 0` would collapse the scheduler. Nothing is zero except the error term, which is exactly what should be zero.

### 1.1 — $C_H$, the energy quantum

$$E = C_H\,\nu, \qquad C_H = \frac{h}{\sqrt{2\pi}} = 2.6434195357408632 \times 10^{-34}\ \text{J·s}$$

**This is neither $h$ nor $\hbar$.** The $1/\sqrt{2\pi}$ normalisation is a deliberate departure from Planck, arising from the hyperbolic-fractal boundary conditions rather than the full $2\pi$ radian loop of angular frequency. For the same process it yields $0.3989\times$ the Planck energy $h\nu$.

Governs energy-bounded state transitions: a thread's cost is set by its frequency, and idle states draw proportionally less because $\nu \to 0$ implies $E \to 0$.

**Execution rule:** replaces the fixed tick interval. The scheduler does not allocate time slices; it allocates energy states. Power consumption at idle falls out of the equation rather than needing a separate governor.

> **⚠ $\nu$, never $\omega$.** $C_H$ pairs with **ordinary frequency**. A process described as "1 GHz" has $\nu = 10^9$ but $\omega = 2\pi\times10^9$ — substituting one for the other changes every energy computation by $2\pi$, and **nothing in the units catches it** (both are J·s × s⁻¹ = J). The two must be distinct types in code. Note `baseline_carrier_frequency` ($\omega_c$) *is* angular and belongs to wave synthesis in `substrate`/`ftg`; it is a different quantity and must never be fed to this equation.

Supersedes the earlier `C_H = ħ` with `E = C_Hω`. See [reconciliation R5a](reconciliation.md#r5a--c_h-redefined-supersedes-the-earlier-value).

### 1.2 — $\xi(r)$, the resonance correction factor

$$\xi(r) = \frac{\sinh(r/R)}{(r/R)\,\sinh(1)}\; e^{\,1 - r/R}$$

The $\operatorname{sinc}$-like $\sinh(x)/x$ factor is what removes the singularity: as $r \to 0$ the ratio tends to 1 rather than diverging, so the correction stays finite at the smallest operating scales.

**Properties, all verified:**

| Property | Value | Why it matters |
|---|---|---|
| $\xi(R)$ | **exactly 1** | no correction at the reference scale |
| monotonicity | strictly decreasing on $(0,\infty)$ | ordering by scale is preserved |
| **bounded** | $\xi < e/\sinh(1) = 2.3130352854993315$ | **the correction can never diverge** |
| $r \to 0$ | $\to e/\sinh(1) \approx 2.3130$ | finite baseline for zero-point oscillations |
| $r \to \infty$ | $\to 0$ | correction fades at large scale |

Boundedness is what makes this safe in a clock path. An unbounded factor would be worse than none — a single bad sample could stall or race the scheduler. Worst case here is a $+131.3\%$ modulation, survivable by construction.

> **Correction to the source rationale.** The specification introducing this form states $\xi(0) = e/\sinh(1) \approx 1$. That quotient evaluates to **2.3130**, not 1. The formula satisfies both properties it was chosen for — unity at $r=R$ and no divergence — so it stands; but the ceiling is $2.3130$, and code must size its headroom for that, not for 1.

**Execution rule:** multiplies the nominal frequency of a timing source at observation scale $r$. Never gates correctness — failing to apply $\xi$ degrades timing precision, it does not produce a wrong result.

> **Execution rule — evaluate piecewise, or boundedness fails.** The literal transcription $\sinh(r)/(r\sinh 1)\cdot e^{1-r}$ **breaks the boundedness law above**: $\sinh(r)$ overflows `f64` at $r \approx 710.5$ before $e^{1-r}$ can rescue the product, yielding $+\infty$ on $[710.5, 745]$ and `NaN` beyond. Returned as a success value, that propagates into every downstream load computation — the exact failure boundedness exists to prevent.
>
> Since $e^{r}e^{1-r} = e$ identically, the same function is $\xi(r) = \dfrac{e - e^{1-2r}}{2r\sinh 1}$, which cannot overflow but loses precision as $r \to 0$, where it differences two nearly-equal numbers.
>
> Use each branch where it is exact, split at the reference scale $R$: the $\sinh$ form for $r \le R$ (where $\sinh(r) \le \sinh 1$, so overflow is impossible) and the exponential form for $r > R$ (where $e^{1-2r} \le e^{-1}$, so cancellation is impossible). The split point is $R$ itself, not a tuned threshold, and both branches give exactly $1$ there.
>
> Verified: the two agree to 2.5 ulp across $[10^{-8}, 700]$; the result is finite, positive and below the supremum for every input up to and including `f64::INFINITY`; and monotonicity is exact across $[10^{-6}, 10^{6}]$.

Supersedes the earlier increasing form. See [reconciliation R5b](reconciliation.md#r5b--xir-reformulated-supersedes-the-earlier-form).

**Composed into a logic gate.** `ξ` multiplying nominal frequency is what makes PRD §3's "scale modulation" a gate rather than an annotation — see [gates.md §3](gates.md#gate-3--scale-modulation), where it combines with the standing-wave `±π/4` criterion to give a derived resonance band of `1/8`.

### 1.3 — $H(\kappa) \to 0$, the convergence invariant

$$H(\kappa) = \int_0^t \gamma(\kappa, t')\,dt' \approx 0$$

The accumulated phase difference between hyperbolic and Euclidean geodesics, which approximates zero through cancellation across fractal scales.

**Execution rule:** this is the scheduler's **health invariant**, not an input. The kernel integrates observed phase error across cores; if `|H(κ)|` grows beyond threshold rather than tending to zero, the correction has failed and the clock domain has diverged. It is a monitorable quantity — the thing you assert in a test and alarm on in production.

---

## Part 2 — Harmonic Force Equilibrium as load balancing

$$\nabla \cdot \mathbf{E} = \frac{\rho}{\epsilon_0}$$

Previously unresolved because no paper gave it an execution rule. It now has one.

| Symbol | Physical | OS meaning |
|---|---|---|
| $\rho$ | charge density | task density — per-core load relative to mean |
| $\mathbf{E}$ | electric field | processing capacity field over the core topology |
| $\epsilon_0$ | permittivity | coupling constant — how strongly load gradients drive migration |

### 2.1 — The discrete form

With `E = −∇φ`, Gauss's law becomes Poisson's equation, and on a core-topology graph the Laplacian is the graph Laplacian `L`:

$$\nabla^2\varphi = -\frac{\rho}{\epsilon_0} \qquad\Longrightarrow\qquad L\varphi = -\frac{\rho}{\epsilon_0}$$

Load flows down the gradient of `φ`. This is diffusion-based load balancing — a real algorithm with known convergence behaviour, not a metaphor.

### 2.2 — Solvability constraint (falls out of the mathematics)

`L` has the constant vector in its nullspace on a connected graph, so `Lφ = b` has a solution **only if** `Σbᵢ = 0`. Therefore:

$$\sum_i \rho_i = 0$$

**Task density must be mean-centred**: `ρᵢ = loadᵢ − mean(load)`. This is not a modelling choice — it is the condition for the field equation to have a solution at all. Physically it is the statement that load balancing redistributes work rather than creating or destroying it.

**Execution rule:** compute `ρ` as deviation from mean load, never as absolute load. Absolute load makes the system unsolvable.

### 2.3 — Stability constraint

The diffusion update `x ← x − αLx` is stable iff `0 < αλ < 2` for every eigenvalue `λ` of `L`:

$$\alpha < \frac{2}{\lambda_{\max}(L)}, \qquad \lambda_{\max}(L) \le 2\,d_{\max}$$

where `d_max` is the maximum core degree in the topology. For a 4-core ring (`d = 2`, `λ_max = 4`) this requires `α < 0.5`.

**Execution rule:** the coupling `α = 1/ε₀` must be derived from the topology, never hardcoded. Exceeding the bound makes the balancer oscillate instead of converge — the failure mode is thrashing, which is precisely what the field model exists to eliminate.

### 2.4 — Verified behaviour

4-core ring, initial load `[10, 2, 2, 2]`:

```
step 1    [7.6000, 3.2000, 2.0000, 3.2000]   spread 5.600e+00
step 10   [4.1132, 3.9998, 3.8872, 3.9998]   spread 2.260e-01
step 40   [4.0000, 4.0000, 4.0000, 4.0000]   spread 5.093e-06
step 80   [4.0000, 4.0000, 4.0000, 4.0000]   spread 3.243e-12
```

Total load conserved exactly (16.0 → 16.0) — the Laplacian is conservative, each row summing to zero. At equilibrium `ρ → 0`, so `∇·E → 0`: the field is source-free and the system is self-stabilised. Cores resonate with incoming execution profiles without aggressive migration.

### 2.5 — Scope boundary: this does not eliminate deadlock

Stated plainly because building on the stronger claim would be dangerous.

The field model provably eliminates **thread thrashing** and **load bottlenecks** — that is what diffusion to a uniform equilibrium means, and §2.4 demonstrates it. It does **not** address deadlock. Deadlock is a circular wait in resource *acquisition*, orthogonal to how work is *distributed*: four perfectly balanced cores can still deadlock on two locks taken in opposite orders.

**Execution rule:** `symphony` must still implement deadlock detection or prevention. Do not omit it on the strength of the equilibrium property. A kernel built believing deadlock impossible will hang with no diagnostic.

---

## Constants introduced

Stored in [constants.json](constants.json) under `resonance`; values live only there.

| Key | Meaning |
|---|---|
| `xi_form` | the operative expression for `ξ(r)` |
| `xi_at_reference` | `ξ(R) = 1` exactly |
| `xi_upper_bound` | `e/sinh(1) = 2.3130…` — the supremum, approached as `r → 0` |
| `diffusion_stability_factor` | the `2` in `α < 2/λ_max` |

`howard_comma` under `constants` now carries `h/√(2π)` and a `frequency_variable: "nu"` field.

## Binds

- [[symphony-kernel]] — `neos/symphony/kernel/src/equilibrium.rs` and `scheduler.rs` (Part 2), `quantization.rs` and `resonance.rs` (Part 1)
- [[substrate]] — clock domain, `ξ` correction applied at the oscillator
