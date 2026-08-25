---
type: reconciliation-ledger
layer: law
status: awaiting-review
date: 2026-08-13
---

# Reconciliation Ledger

The source corpus contains **mutually incompatible definitions** for several core concepts, and several formulas do not evaluate to the results their own papers claim. This ledger picks exactly one definition per concept. Those choices become law; `_mkb/` and all code follow this file, not the papers.

Every row is a decision. Read them before any code is built on them.

**Method:** where papers conflict, preference goes to (a) the definition that is dimensionally coherent, (b) the paper written as a formal axiomatic foundation over one written as an application, (c) the reading that makes a stated axiom exactly true rather than approximately true.

---

## R1 — The ⊗ operator · RESOLVED

| Source | Definition | 1⊗1 gives |
|---|---|---|
| `_mkb/equations.md` (old) | `a×b + d(a,b)` | **1** — any metric has `d(x,x)=0` |
| `Mathematical_Fra.pdf` | `a×b + sinh(a·b·l_P/r)` | **2.1752** at `r = l_P` |
| `vACUUM_FLUX.pdf` | `2R·sinh(a/R)·sinh(b/R)` | ≈2 only near `R≈1.232`; overflows at the stated `R=3.6e-10 m` |

**Resolved:** adopt the `Mathematical_Fra.pdf` form. It is the only paper written as a rigorous axiomatic foundation, and it is the only form that is dimensionally coherent.

Per the decision that **`1⊗1 = 2` binds as an exact identity**, the scale parameter is pinned:

$$r = \frac{l_P}{\operatorname{arcsinh}(1)} \qquad \Rightarrow \qquad \frac{l_P}{r} = \operatorname{arcsinh}(1) = \ln(1+\sqrt2)$$

Substituting, the operator becomes scale-free:

$$a \otimes b = a\,b + \sinh(a\,b\,\lambda), \qquad \lambda = \operatorname{arcsinh}(1) = 0.881373587019543$$

**Why this is the right pin:** `sinh(arcsinh(1)) = 1` identically, so `1⊗1 = 1 + 1 = 2` holds **bit-exactly** in IEEE-754 — verified, `otimes(1,1) == 2.0` is `True`, not merely within tolerance. The axiom needs no epsilon. As a bonus the absolute value of `l_P` drops out of the operator entirely; only the ratio survives, which removes the dimensional inconsistency that broke the `vACUUM_FLUX` form.

### R1a — Consequence: ⊗ is strongly non-associative

`Mathematical_Fra.pdf` states associativity holds to `O(l_P²/r²)` and calls the correction "negligible except at Planck scales." **The pinned scale is the Planck scale** — `l_P/r = 0.881`, which is order 1, not small. The correction is therefore not negligible; it dominates. Measured:

```
(2⊗3)⊗4 = 2.864e160
2⊗(3⊗4) = OVERFLOW (+inf)
```

Not a small deviation — total divergence. **No code may assume associativity or distributivity of ⊗.** Evaluation order is semantically significant and must be explicit at every call site.

### R1b — Consequence: ⊗ has a hard domain limit

`sinh` overflows `f64` near argument 710, so ⊗ is defined only for:

$$a\,b < \frac{710}{\lambda} \approx 805.56$$

Beyond that it returns `+inf`. This is a **domain constraint, not a rounding concern** — it must be enforced by the type system or a checked constructor, never left to the caller to remember.

---

## R2 — Curvature `K` · RESOLVED

| Source | Value |
|---|---|
| `_mkb/constants.json` | `-1.0` |
| `vACUUM_FLUX.pdf` | `K = -1/R²`, with `R = 3.6e-10 m` |

**Resolved:** keep `K = -1.0`, and record `K = -1/R²` as the *physical embedding* form.

These reconcile under an explicit unit convention: the lattice works in **lattice-native units where `R = 1`**, giving `K = -1`. The `vACUUM_FLUX` value applies when embedding the lattice into physical superfluid-helium space. The Poincaré disk distance function in `equations.md` is valid as written only at `K = -1`, so the native convention is the one the code uses.

**Written down so it cannot drift back:** all `lattice` code is in native units. Any physical embedding is a conversion at the boundary, never a change to `K`.

---

## R3 — Vertex degree of the tessellation · RESOLVED

`vACUUM_FLUX.pdf` writes: a `{5,4}` tessellation, "where **five** pentagons meet at each vertex."

Schläfli `{p,q}` means *q* p-gons meet at each vertex. `{5,4}` therefore means **four** pentagons per vertex, not five. The prose contradicts the notation in the same sentence.

**Resolved:** the notation is correct, the prose is a transcription error. **`{5,4}` — four pentagons per vertex, vertex degree 4.**

Reasons: `{5,4}` is the notation used consistently in the PRD and everywhere else in the corpus; both `{5,4}` and `{5,5}` are validly hyperbolic (`(p−2)(q−2) > 4` holds for each), so feasibility does not disambiguate — but only the notation appears more than once.

This determines the entire adjacency structure of `tessellation.rs`. Vertex degree is asserted directly in the test suite.

---

## R4 — Fractal dimension · RESOLVED

| Source | Value |
|---|---|
| `Mathematical_Fra.pdf` | `D(r) = 3 + sin²(πr/l_P)` → ranges over [3, 4] |
| `vACUUM_FLUX.pdf` | `D ≈ 2.32` |

These cannot both be right: the first can never produce 2.32.

**Resolved:** adopt `D(r) = 3 + sin²(πr/l_P)` as the lattice's scale-dependent dimension. `D ≈ 2.32` describes the *superfluid embedding* — a measured property of the physical helium/graphene realisation, not of the abstract lattice. Recorded as context, not as law.

Not consumed by `lattice` in this build; it is recorded now so the conflict is not rediscovered later.

---

## R5 — Howard Comma · RESOLVED

Four definitions, apparently three incompatible:

| Source | Definition |
|---|---|
| `_mkb/constants.json` | `1.0545718e-34` — numerically ħ |
| `Cosmological_Constant+.pdf` | replaces *unreduced* `h`; used squared in `Eₙ = n²C_H²/8mL²` (particle-in-a-box form) |
| `Mathematical_Fra.pdf` | accumulated geodesic phase difference, `H(κ) ≈ 0` |
| `vACUUM_FLUX.pdf` | `ξ(r) = sinh(1)·sinh(r/R)·e^(−r/R)`, claimed `= 1` at `r = R` — **evaluates to 0.508** |

**Resolved — the definitions were never in competition.** One symbol was carrying three distinct jobs:

| Role | Object | Duty |
|---|---|---|
| energy quantum | `C_H` — value since revised, see **R5a** | discrete energy-bounded state transitions |
| resonance correction | `ξ(r)`, dimensionless | clock jitter damping |
| convergence invariant | `H(κ) → 0` | proof the correction is working |

The apparent contradiction dissolves once `H(κ) ≈ 0` is read as what it is: the **residual after correction**, not a value of `C_H`. Accumulated phase error tends to zero *because* the damping works. Nothing is zero except the error term — which is exactly what should be zero — so the concern that a near-zero `C_H` would collapse the scheduler does not arise.

The value stays at ħ: `E = C_Hω` uses angular frequency, which pairs with ħ rather than h.

Full treatment: **[resonance.md § Part 1](resonance.md#part-1--the-howard-comma-is-three-objects-not-one)**.

### R5a — `C_H` redefined (supersedes the earlier value)

The role split above stands. The **value and its frequency pairing have since changed** by decision:

| | Superseded | Current |
|---|---|---|
| `C_H` | `1.0545718e-34` (ħ) | **`2.6434195357408632e-34`** = `h/√(2π)` |
| equation | `E = C_H·ω` (angular) | **`E = C_H·ν`** (ordinary) |

Rationale given: the `1/√(2π)` normalisation follows from the hyperbolic-fractal boundary conditions rather than the full `2π` radian loop, and ordinary frequency ties state transitions to the kernel's clock-resonance ticks. The `(ħ, ω)` pairing is formally deprecated.

**Consequence:** `E = C_H·ν` gives **0.3989×** the Planck energy `hν` for the same process. This is a deliberate departure from Planck, not an approximation of it.

**Implementation hazard — `ν` vs `ω`.** A process described as "1 GHz" has `ν = 1e9` but `ω = 2π×1e9`. Substituting one for the other changes every energy computation by `2π`, and **the units do not catch it** — both are J·s × s⁻¹. They must be distinct types in code. Note `baseline_carrier_frequency` (`ω_c`) remains angular and belongs to wave synthesis; it must never reach this equation.

### R5b — `ξ(r)` reformulated (supersedes the earlier form)

| | Superseded | Current |
|---|---|---|
| form | `sinh(r/R)e^(−r/R)/(sinh(1)e⁻¹)` | **`sinh(r/R)/((r/R)sinh(1)) · e^(1−r/R)`** |
| `ξ(R)` | 1 | 1 |
| shape | increasing | **decreasing** |
| bound | 1.1565 (asymptote) | **2.3130** = `e/sinh(1)` (at `r→0`) |

The `sinh(x)/x` factor removes the singularity at `r → 0`, so the correction stays finite at the smallest operating scales while still decaying with scale.

Both required properties are preserved: **unity at the reference scale** and **boundedness**. An intermediate proposal — `ξ(r) = sinh(1)/sinh(r/R)·e^(−r/R)` — was rejected because it gives `ξ(R) = 0.368` and **diverges** as `r → 0` (reaching 1.2e6 at `r = 1e-6·R`), which in a clock path turns a small sample error into a stalled scheduler.

**One correction to the source rationale:** it states `ξ(0) = e/sinh(1) ≈ 1`. That quotient is **2.3130**, not 1. The formula stands — it satisfies both properties it was chosen for — but the true ceiling is 2.3130 and code must size headroom for that.

---

## R6 — Harmonic Force Equilibrium · RESOLVED

`∇·E = ρ/ε₀` was assigned to process scheduling but had no execution rule in any paper — and implementing a scheduler against an equation with no execution rule was forbidden.

**Resolved:** the equation governs **dynamic load balancing**, with `ρ` as task density (per-core load relative to mean) and `E` as the processing capacity field over the core topology.

With `E = −∇φ` this is Poisson's equation; on a core-topology graph it becomes `Lφ = −ρ/ε₀` for the graph Laplacian `L`. Load flows down the gradient of `φ`. That is diffusion-based load balancing — a real algorithm with known convergence, not a metaphor.

Two constraints fall out of the mathematics rather than being chosen:

- **Solvability:** `L`'s nullspace is the constants, so `Σρᵢ = 0` is required for any solution to exist. Task density *must* be mean-centred; absolute load makes the system unsolvable.
- **Stability:** `α < 2/λ_max(L)`, so the coupling `1/ε₀` must be derived from the topology. Exceeding it produces oscillation — the thrashing the model exists to eliminate.

Verified on a 4-core ring: `[10,2,2,2]` converges to uniform with spread `3.2e-12` by step 80, total load conserved exactly.

**Scope boundary, recorded deliberately:** this eliminates thread thrashing and load bottlenecks. It does **not** eliminate deadlock — that is a circular wait in resource acquisition, orthogonal to load distribution. `symphony` must still implement deadlock detection. See [resonance.md § 2.5](resonance.md#25--scope-boundary-this-does-not-eliminate-deadlock).

Full treatment: **[resonance.md § Part 2](resonance.md#part-2--harmonic-force-equilibrium-as-load-balancing)**.

---

## Summary

| ID | Concept | Status | Blocks |
|---|---|---|---|
| R1 | ⊗ operator | ✅ resolved — pinned, `1⊗1=2` exact | — |
| R1a | non-associativity | ✅ resolved — strongly non-associative | constrains all code |
| R1b | overflow domain | ✅ resolved — `a·b < 805.56` | constrains all code |
| R2 | curvature `K` | ✅ resolved — `-1.0`, native units | — |
| R3 | vertex degree | ✅ resolved — 4 per vertex | — |
| R4 | fractal dimension | ✅ resolved — `3 + sin²(πr/l_P)` | — |
| R5 | Howard Comma | ✅ resolved — three roles, not three rival definitions | — |
| R5a | `C_H` value and frequency pairing | ✅ resolved — `h/√(2π)` with `E = C_H·ν` | **supersedes** ħ with ω |
| R5b | `ξ(r)` form | ✅ resolved — unity at reference, bounded by `e/sinh(1)` | **supersedes** the increasing form |
| R6 | Harmonic Force Equilibrium | ✅ resolved — diffusion load balancing | — |

**The ledger is closed. R1–R6 all resolved.**

No subsystem is blocked on unreconciled law. `lattice` is built. `gui` is unblocked by [tetryen.md](tetryen.md); `symphony` and `substrate` by [resonance.md](resonance.md); `ftg` was never blocked.

Two items are carried as **scope boundaries** rather than open questions — settled decisions about what the law does *not* claim:

- Load equilibrium does not eliminate deadlock ([resonance.md § 2.5](resonance.md#25--scope-boundary-this-does-not-eliminate-deadlock)). `symphony` must implement deadlock detection.
- Two papers remain undistilled (`Conformational Proof`, `Reconstructing Saturn`). Neither is referenced by any current subsystem contract. `finite universe` has since been read and evaluated in full — it contributed no new operational content, and its one named placeholder (`f(ψ_n,ψ_{n-1})`) was closed separately as a synthesis, [tetryen_recurrence.md](tetryen_recurrence.md). See [papers/_index.md](papers/_index.md).
