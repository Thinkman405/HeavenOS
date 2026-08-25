---
type: math-contract
subsystem: crystallisation
stage: 01_derive
derived_from: ["_mkb/axioms.md", "_mkb/operators.md", "_mkb/tetryen.md", "_spec/prd.md"]
consumes: [lattice, substrate]
---

# Crystallisation — Math Contract

PRD §8: convert linear 1D/2D data into native 3D/4D resonant shapes. Three pipelines, and **they do not have equal standing in the law.**

## 1. What each pipeline can be built on

| Pipeline | Law available | Status |
|---|---|---|
| **Linguistic** | A1 bifurcation, ⊗ from `operators.md`, `substrate` translation | fully specified |
| **Holographic** | Fourier transform (standard maths), `tetryen.md` for the projection surface | fully specified |
| **Resonant chambers** | "localised oscillator" — yes, via `substrate` + a frequency | **partially** specified |

### 1.1 — "Volumetric time-crystal" — gap now closed

This section originally recorded the term as **undefined**: PRD §8 offers *"localized oscillators **or** volumetric time-crystals"*, and only the first had law. No paper in the corpus defines a time-crystal, so unlike the Tetryen there was nothing to distil, and inventing one was forbidden.

**The gap is now closed by [`_mkb/timecrystal.md`](../../../../_mkb/timecrystal.md)** — a synthesis of two things that *are* law, the Howard Comma and Tetryen geometry, recorded as a synthesis rather than a distillation:

- **Takens delay embedding** into 4D phase space, one component per Tetryen vertex
- **Floquet quasi-energies** quantised by `C_H` in place of `ħ`
- **`SO(3,1)` modulation** preserving phase-space volume (Liouville) and the Minkowski form

Two execution rules came out of building it, neither stated in the definition:

1. **Joint quantisation is mandatory.** Independent per-mode rounding violates the half-quantum bound by up to 36×; the fundamental must absorb the residual.
2. **There is a quantisable ceiling.** `C_H ≈ 2.6e-34` means a unit-amplitude tone needs `2.5e35` quanta, past `f64`'s exact-integer limit. Signals above `~1.9e-17` J are refused.

Both are recorded in `timecrystal.md`.

## 2. Binding axioms

**A1 — Multiplicative Identity Override.** "Line breaks or code structures trigger bifurcation events." A bifurcation scales extent by `u ⊗ u`, not by copying. Unit case is exactly 2.

**A3 — Spatial Addressing Override.** Frequency maps project into hyperbolic space, onto Tetryen faces.

A2 does not bind this subsystem — nothing here evaluates a branch.

## 3. Linguistic crystallisation

> "Linear character strings are converted into sequential harmonic nodes. Line breaks or code structures trigger bifurcation events, rendering text documents as navigable 3D polymer-like fractals."

**Execution rule:** each character becomes one harmonic node, in order. Each line break is a bifurcation event, scaling the structure's extent by ⊗.

### 3.1 — The systemic depth ceiling

A1 says a bifurcation is `u ⊗ u` — **self**-⊗, which squares the product each time:

```
break 1 -> 2.0
break 2 -> 20.970562748477143
break 3 -> 1.071836741105424e168
break 4 -> REFUSED (domain)
```

**A document can carry three line breaks before ⊗ leaves its domain.**

> **Correction.** An earlier draft of this contract quoted a ceiling of **4**, taken from a probe that iterated `e ⊗ 1` — a *unit step*, not a bifurcation. Self-⊗ squares rather than steps, so it exits one level sooner. The implementation was right and the contract's number was wrong; caught by the tests failing against the real operator.

This is the *same underlying* ceiling `lattice` curved addressing hits, at a different arity: unit steps reach 4, self-⊗ reaches 3. **The constraint is systemic**, arising from ⊗'s super-exponential growth against a fixed domain wherever it is iterated — any subsystem that iterates ⊗ inherits it, and the exact depth depends on how the operands grow.

**Execution rule:** a document exceeding the bifurcation depth must be **refused**, not silently truncated. Callers can query the limit before crystallising.

## 4. Holographic projection

> "Standard pixel grids are passed through a Continuous Fourier Transform (CFT), converted into spatial frequency maps, and projected onto the internal faces of scalable Tetryen geometry."

On a discrete grid the CFT is the DFT:

$$F(u,v) = \sum_{y}\sum_{x} f(y,x)\, e^{-2\pi i (uy/H + vx/W)}$$

**Verified properties**, both exact enough to assert:

- **Parseval:** spatial energy equals frequency energy over `H·W`. Measured `425.0` vs `425.00000000000006`.
- **DC term:** `F(0,0)` equals the sum of all pixels, exactly.

**Execution rule:** the transform must be invertible — a round trip recovers the grid. A lossy "frequency map" would not be a representation of the image.

### 4.1 — Projection onto four faces

A Tetryen has **four** faces (`_mkb/tetryen.md`: four nodes at tetrahedron vertices). Coefficients distribute across them.

**Execution rule:** the coefficient count must be divisible by 4 to tile the faces evenly; otherwise the projection is refused rather than silently unbalanced.

## 5. Resonant chambers — the oscillator reading only

**Execution rule:** a media sample stream maps to a localised oscillator carrying an ordinary frequency `ν`, using `substrate`'s `Frequency` type so it cannot be confused with the angular carrier `ω_c`.

**Not built:** volumetric time-crystals, 4D spatial rotation of media, and "driving physical vibrations" — the first is undefined (§1.1), and the other two depend on it.

## 6. Consumed, never reimplemented

| From | What |
|---|---|
| `lattice` | `LatticeScalar::otimes` for bifurcation; `tetryen`-adjacent geometry |
| `substrate` | `Frequency` / `AngularFrequency`, bit↔phase translation |

`⊗` has one home. This subsystem calls it; it does not restate it.

## 7. Forbidden constructs

| Forbidden | Because |
|---|---|
| inventing time-crystal semantics | §1.1 — undefined, and no source to distil |
| silently truncating an over-deep document | §3.1 — refuse, do not lose content |
| a lossy frequency map | §4 — must round-trip |
| unbalanced face projection | §4.1 |
| `ω` where `ν` is required | inherited; the newtypes make it a compile error |
| reimplementing ⊗ | §6 |
| hardcoding any constant from `constants.json` | one home per fact |

## 8. Open questions

**One, recorded not closed:** the volumetric time-crystal reading of PRD §8 has no operational definition in the corpus. It is not blocking — the oscillator reading covers the pipeline's buildable half — but the PRD sentence cannot be fully implemented until a source defines it.
