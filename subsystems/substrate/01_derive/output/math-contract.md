---
type: math-contract
subsystem: substrate
stage: 01_derive
derived_from: ["_mkb/axioms.md", "_mkb/equations.md", "_mkb/operators.md", "_mkb/constants.json", "_spec/prd.md"]
consumes: [lattice]
---

# Substrate — Math Contract

The complete law binding `neos/substrate`. Formulas copied, not paraphrased.

## 1. Binding axioms

**A3 — Spatial Addressing Override.** Cartesian coordinate arrays are replaced by non-Euclidean hyperbolic metric spaces.

This is the subsystem's defining constraint. Substrate is where NEOS meets flat silicon, so it is the **only** place the translation may live — and therefore the place where a leak would poison everything above it.

**A1 — Multiplicative Identity Override.** Memory pool splitting is a structural geometric split: splitting a pool scales address space by `⊗`, not by copying. A unit split yields exactly 2.

**A2 — Logic Gate Override.** Binary states map to phase orientations `{−π/2, +π/2}` in the translation pipeline.

## 2. The flat/curved boundary — the decision this subsystem exists to make

Hardware is flat and byte-addressed. A3 says addressable space is not. The PRD is explicit that NEOS *simulates* a geometric universe "atop traditional discrete Boolean hardware", so the translation is the substrate's job, not something to be avoided.

**The boundary is the public API of `memory`.**

- Flat offsets exist **only inside** the memory module.
- No public type, function, or field exposes a raw pointer, a `usize` byte offset, or any linear address.
- Every public address is a lattice coordinate: a `CellId` from `lattice` plus an intra-cell offset.

**Rationale, and why it is load-bearing:** any consumer that can obtain a flat address will eventually compute with it — and the moment it does, it is working in Euclidean space regardless of what the geometry layer says. `ftg` Layer 3/4 routing must read a native non-Euclidean space, not a flat abstraction wearing geometric names. The guarantee has to be structural, because a convention would not survive contact with a hot loop.

**Enforced by construction**, in the manner of the `ν`/`ω` separation in `symphony-kernel`: the flat offset type is private, so a leak is a compile error rather than a review finding.

## 3. Addressing

Address resolution goes through `lattice`:

- Cells and adjacency come from `lattice::Tiling` and `Cell::neighbors()`. **Not reimplemented.**
- Distance between two addresses is the hyperbolic distance between their cell centres, per `equations.md`.
- Allocation locality follows lattice adjacency: an allocation spanning multiple cells occupies **adjacent** cells, never the next flat index.

Vertex degree 4 and 5 face-neighbours per cell are `lattice`'s guarantees; substrate reads them.

## 4. Pool splitting (A1)

$$\text{split}(u) = u \otimes u$$

computed with `lattice::LatticeScalar::otimes`. For the unit pool this is exactly `2.0` — bit-exact.

**Execution rule:** splitting a memory pool doubles addressable extent by geometric bifurcation, not by allocating a second copy. `⊗`'s domain limit (`a·b < 805.56`) applies and must be enforced, not documented.

## 5. Binary ↔ wave translation (A2)

$$W(t) = \sum_{k=0}^{N-1} \left[ A \cos(\omega_c t + \phi_k) \right], \qquad \phi_k \in \{-\pi/2, +\pi/2\}$$

Bit `0` maps to `−π/2`, bit `1` to `+π/2`.

### 5.1 — The carrier is information-free at zero crossings

Since `cos(x + π/2) = −sin(x)` and `cos(x − π/2) = +sin(x)`, the two bit states differ **only in the sign of the sine component**. Measured:

| `ω_c t` | bit 0 | bit 1 | separation |
|---|---|---|---|
| 0 | 0.000 | 0.000 | **0.000** |
| π/2 | +1.000 | −1.000 | **2.000** |
| π | 0.000 | 0.000 | **0.000** |
| 3π/2 | −1.000 | +1.000 | **2.000** |

**Execution rule:** demodulation must sample at an odd quarter period (`ω_c t = π/2 + nπ`), where separation is maximal. Sampling at `t = 0` or any half-period recovers **nothing** — both bits read as zero. This is a correctness constraint, not a quality one.

Quarter period at `ω_c`: `2.5e-10 s`.

### 5.2 — Round trip must be lossless

`bits → phases → carrier → phases → bits` must be the identity for arbitrary byte sequences. Any loss here corrupts every layer above.

### 5.3 — Destructive interference is exact

Opposite phases cancel for **all** `t`, not merely at sample points — verified to `~1e-16`. This is what makes phase teardown work without an acknowledgement.

## 6. Clock

`ω_c = 6283185307.179586 rad/s` (`baseline_carrier_frequency`), the resting synchronisation parameter.

> **⚠ `ω_c` is ANGULAR.** It must never be substituted into `E = C_H·ν`, which takes ordinary frequency. They differ by `2π` and the units do not distinguish them. Frequency types must keep them apart — see `reconciliation.md` R5a.

**Layering consequence:** the frequency newtypes are a substrate concern, because substrate is the lowest layer that uses them (the clock). `symphony-kernel` currently defines its own copies; those must move here and be re-exported, or the two will drift. Dependency direction is `lattice ← substrate ← symphony-kernel`, matching the PRD's tiering.

## 7. Forbidden constructs

| Forbidden | Because |
|---|---|
| any public flat/linear address | §2 — the whole point of the subsystem |
| reimplementing tiling, adjacency, or the metric | §3, one home per fact |
| scalar duplication on pool split | A1 |
| `bool` in the translation pipeline | A2 |
| demodulating at a carrier zero crossing | §5.1 |
| substituting `ω_c` where `ν` is required | §6 |
| duplicate frequency types across crates | §6 |
| hardcoding any constant from `constants.json` | one home per fact |

## 8. Constants consumed

| JSON path | Use |
|---|---|
| `constants.baseline_carrier_frequency.value` | `ω_c` |
| `constants.hyperbolic_curvature.value` | `K = −1` |
| `logic_phases.*` | bit ↔ phase mapping |
| `operators.otimes_domain_max_product.value` | pool split domain |

## 9. Open questions

**None blocking.** The flat/curved boundary was the subsystem's open design question; §2 settles it.
