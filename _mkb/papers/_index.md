---
type: corpus-manifest
layer: evidence
---

# Source Corpus

The foundational theory. These are **evidence, loaded last** — an agent implementing a subsystem reads [axioms.md](../axioms.md), [operators.md](../operators.md), [constants.md](../constants.md), and [equations.md](../equations.md), and comes here only when a derivation is contested.

Do not load PDFs into a build step by default. They are large, and the distilled layer exists precisely so they don't have to be.

**Where the papers disagree, [reconciliation.md](../reconciliation.md) governs.** The corpus is exploratory work containing mutually incompatible definitions; the distilled layer is the single consistent reading extracted from it. Never implement directly from a paper.

## Extraction status

Checkbox = has this paper's content been distilled into the law layer?

- [x] `Mathematical_Fra.pdf` — **the keystone.** Supplied the concrete ⊗ operator, the ⊕/⊘/power forms, associativity bounds, the Tetryen variational definition, and the fractal dimension. → [operators.md](../operators.md), [tetryen.md](../tetryen.md)
- [x] `vACUUM_FLUX.pdf` — supplied the `{5,4}` tessellation, `K = -1/R²`, the lattice scale `R`, node dynamics, and a competing ⊗ form (rejected, R1). → [reconciliation.md](../reconciliation.md)
- [x] `the-geometry-of-proton-and-the-tetryen-shape-v1-1+(1).pdf` — the primary Tetryen source: four cores at standing-wave nodes forming a curved tetrahedral structure. → [tetryen.md](../tetryen.md)
- [x] `Cosmological_Constant+.pdf` — the `E = C_Hω` derivation and the `Eₙ = n²C_H²/8mL²` quantisation. Howard Comma nature still open (R5).
- [x] `lynchpin number theory.pdf` — motivates A1 ($1\times1=2$), primes as harmonic resonance, 2 as a bifurcation point rather than a prime. Informal essay; contains **no operational definition** of ⊗.
- [x] `The neccessity of a finite universe.pdf` — read and evaluated in full, equation by equation. Contributes **no new operational content**: every formula either restates law already distilled elsewhere with zero new content (`1×1=2`, the standing-wave form, `E=C_Hν`, the Fourier transform definition) or has no operational definition at all (the "Tetryen emergence" recurrence `ψ_{n+1}=f(ψ_n,ψ_{n-1})` never defines `f`; the "infinite energy density" argument is prose, not a derivation). The one candidate formula (`Eₙ=n²C_H²/8mL²`, a boxed-particle quantisation) introduces `mass`/`boundary length` with no home in any current subsystem — recorded as a gap rather than an invented connection. The `f` placeholder itself is closed separately, as a synthesis: [tetryen_recurrence.md](../tetryen_recurrence.md).
- [ ] `Conformational Proof analysis.pdf` — proof analysis. Not yet distilled.
- [ ] `4~+Reconstructing_Saturn_Lynchpin_ImprovedMath-1.pdf` — applied Lynchpin math. Not yet distilled; no hits for any core term.

## Open conflicts — none

**Every row of [reconciliation.md](../reconciliation.md) is resolved.** No subsystem is blocked on unreconciled law.

Closed here: ⊗ definition (R1) with its non-associativity (R1a) and domain limit (R1b), curvature (R2), vertex degree (R3), fractal dimension (R4), the Howard Comma's three roles (R5), its value and frequency pairing (R5a), the `ξ(r)` reformulation (R5b), and Harmonic Force Equilibrium as diffusion load balancing (R6). The Tetryen definition lives at [tetryen.md](../tetryen.md).

Two of those were later revised by decision rather than by new evidence — R5a moved `C_H` from ħ to `h/√(2π)` with `E = C_H·ν`, and R5b replaced `ξ(r)` with a bounded, non-singular form. Both supersessions are recorded in the ledger rather than overwritten, so the earlier readings remain auditable.

## A caution on the corpus

Several papers contain arithmetic that does not evaluate to its stated result — `vACUUM_FLUX`'s `ξ(R) = 1` gives 0.508; `Mathematical_Fra`'s "1⊗1 = 2 as `r → l_P`" gives 2.175. These are recorded in [reconciliation.md](../reconciliation.md) rather than silently corrected. When distilling a new paper, **evaluate its formulas before trusting its claims**.
