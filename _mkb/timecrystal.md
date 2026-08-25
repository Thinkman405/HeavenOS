---
type: subsystem-law
layer: law
status: canonical
closes: "PRD §8 volumetric time-crystal — previously undefined"
---

# Volumetric Time Crystal

A discrete spatiotemporal boundary structure exhibiting **periodic, non-thermal motion in time** while bound within a non-Euclidean 4D lattice.

Closes the gap `crystallisation`'s `01_derive` recorded: PRD §8 offered "localized oscillators **or** volumetric time-crystals", and only the first had law. This is the second.

Unlike [tetryen.md](tetryen.md), which was distilled from `Mathematical_Fra.pdf`, **no paper in the corpus defines this**. It is a synthesis of two things that *are* law — the [Howard Comma](resonance.md) and [Tetryen geometry](tetryen.md) — and is recorded as such rather than as a distillation.

## 1. Spatial foundation — Tetryen partitioning

A continuous time series `s(t)` embeds into 4D phase space by **Takens delay embedding**:

$$\mathbf{X}(t) = \left[\, s(t),\; s(t-\tau),\; s(t-2\tau),\; s(t-3\tau) \,\right] \in \mathbb{R}^4$$

The four components map to the four vertices of a fundamental Tetryen cell — the same four-ness `tetryen.md` fixes structurally.

**Execution rule:** `τ = 0` embeds nothing (all four components collapse onto one sample) and must be refused.

## 2. Temporal quantisation — the Howard Comma

### 2.1 Floquet quasi-energies

A VTC is driven by a periodic Hamiltonian `H(t) = H(t+T)` at the sample period `T = 1/ν₀`. Temporal states quantise into discrete quasi-energies:

$$\psi_k(t) = e^{-i\,\epsilon_k t / C_H}\, u_k(t), \qquad u_k(t+T) = u_k(t)$$

with **`C_H` standing where `ħ` normally would**, per [reconciliation R5a](reconciliation.md).

### 2.2 Quantised volumetric action

$$\oint_{\text{Tetryen}} \mathbf{p}\cdot d\mathbf{q} - E\,dt = n\,C_H, \qquad n \in \mathbb{Z}^+$$

### 2.3 The conservation invariant

$$\left|\, E_{\text{crystal}} - \sum_k n_k (C_H\,\nu_k) \,\right| \;\le\; \tfrac{1}{2} C_H \nu_0$$

A transformation that moves total energy beyond the half-quantum floor is **non-unitary** and fails the doctrine check.

> **Execution rule — the `n_k` must be chosen jointly.** The invariant does not say how, and the obvious choice — rounding each mode independently — **violates it**. Independent errors accumulate while the bound is a *single* half-quantum. Measured on an eight-harmonic signal: independent rounding gives a residual of `1.04e-31` against a floor of `1.32e-32`, exceeding it eightfold, worst case 36×. Quantise the harmonics freely and let **the fundamental absorb the residual**, which brings it inside by construction — verified at `2.11e-33`.

### 2.4 The quantisable ceiling

`C_H ≈ 2.64e-34` J·s, so a unit-amplitude tone needs **`2.5e35` quanta** — far past `f64`'s exact-integer ceiling of `2⁵³ ≈ 9.0e15`. Beyond that, adding a quantum does not change the total and the quantisation is pretend.

**Execution rule:** refuse a signal whose occupation would exceed `2⁵³`. Maximum quantisable energy is `2⁵³ · C_H · ν₀` — about **`1.9e-17` J** at `ν₀ = 7.8` Hz.

This is the physics meeting IEEE-754, in the same way `⊗`'s domain limit is, and it is surfaced rather than papered over.

## 3. Modulation — `SO(3,1)` pseudo-rotation

Pitch shifting, time dilation, and filtering are 4D pseudo-rotations `R₄ ∈ SO(3,1)` across the Tetryen vertices.

**Two invariants, both verified exactly:**

- **Liouville:** `det(J) = 1`. Phase-space volume is information content; a modulation that changed it would create or destroy information.
- **Minkowski form preserved:** the `(3,1)` interval is invariant, so modulation is a change of view rather than of content.

**Execution rule:** reject any transform with `det ≠ 1`.

## 4. Summary

| Property | Standard analogy | NEOS implementation |
|---|---|---|
| Spatial envelope | atomic lattice | **Tetryen geometry**, 4D hyperbolic cell |
| Temporal periodicity | subharmonic oscillation | **Floquet modes** at `T = 1/ν₀` |
| Quantisation step | `ħ` | **Howard Comma** `C_H = h/√(2π)` |
| Phase-space state | continuous trajectory | **Takens 4D vector** on Tetryen vertices |
| Conservation | symplectic volume, energy | **Liouville `det = 1`** + `C_H ν` quanta |

## 5. Video — a frame sequence as the embedded signal

Closes the second remaining gap `crystallisation`'s `04_implement` recorded: PRD §8 groups video with audio under *"media files act as localized oscillators or volumetric time-crystals"*, but §§1–4 above only ever embed a **one-dimensional** time series. A frame sequence is not one.

Like §§1–4, this is a **synthesis** — no paper defines it — and like them, it is built by composing law that already exists rather than by inventing anything new.

### 5.1 The composition

Section 2 of [gates.md](gates.md) established the discipline of reusing a primitive exactly where it already fits rather than deriving a parallel one. The same move applies here:

$$s_k = E[\text{frame}_k], \qquad E[\cdot] \text{ the holographic pipeline's own energy}$$

Each frame is a 2D grid — exactly what the holographic pipeline already crystallises — so its energy is already defined, already Parseval-verified, and already tested: `PixelGrid::energy()`, the sum of squared pixel values. Reducing a frame to that one number turns a frame *sequence* into the one-dimensional signal `[s_0, s_1, ..., s_{N-1}]`, which is precisely the input type §§1–4's machinery already takes. Video crystallisation is this reduction followed by the **unmodified** procedure of §§1–4: Takens embedding, Floquet quantisation by `C_H`, `SO(3,1)` modulation. Nothing downstream of the reduction is new.

### 5.2 What is *not* claimed

This is a scalar-per-frame reduction, not a per-pixel embedding. A video's spatial structure within a frame is not represented in the resulting time-crystal — only how each frame's total energy varies from the next. A richer reading (e.g. embedding several holographic face-energies per frame, or a per-pixel Takens embedding) has no operational definition in the corpus and is not invented here.

### 5.3 The quantisation ceiling applies exactly as before, and that is the honest result

§2.4 already established `C_H`'s scale meets IEEE-754 with a hard ceiling — `2^53` quanta, about `1.9e-17` J at `nu_0 = 7.8` Hz — and that the existing audio-driven test suite works *only* because its signals are deliberately scaled into that regime (amplitude `~2e-15`).

A real video frame's energy is not in that regime. Verified: a modest 64×64 8-bit frame has `PixelGrid::energy()` on the order of `10^8`; `crystallise`'s own `input_energy` is the sum of squares of the **per-frame energy sequence**, so a handful of such frames already sits around `10^17`–`10^18` — **on the order of `10^50` quanta**, fifty orders past the ceiling.

**This is not a defect to fix.** It is the same ceiling `§2.4` already names, encountered by a second signal source rather than by a coincidence video introduces. The execution rule is the one already in force:

> **Execution rule.** `crystallise_video` performs the frame-energy reduction and nothing else — it does not rescale, normalise, or otherwise invent a mapping from pixel-domain energy into the quantisable regime, because no such mapping exists in the corpus. A caller wanting quantised video must supply frames already scaled the way the existing audio tests scale their signal. An unscaled, realistic frame sequence is **refused** via the existing `EnergyExceedsQuantisation`, not truncated or silently rescaled.

## Binds

- [[crystallisation]] — `neos/crystallisation/src/timecrystal.rs`, `neos/crystallisation/src/codec.rs`
- Consumes [resonance.md](resonance.md) for `C_H`, [tetryen.md](tetryen.md) for the spatial envelope, and (§5) this file's own §§1–4 for the unmodified VTC procedure
