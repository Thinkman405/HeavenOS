---
type: test-plan
subsystem: crystallisation
stage: 03_tests
derived_from: ["../02_design/output/design.md", "../01_derive/output/math-contract.md"]
doctrine: _mkb/test-doctrine.md
---

# Crystallisation — Test Plan

Target: `neos/tests/crystallisation.rs`. **[D]** marks assertions a conventional serialiser could not pass.

## Group 1 — linguistic

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 1.1 | characters become sequential nodes | 1:1, in order | none | — |
| 1.2 **[D]** | one line break gives extent **exactly 2** | `2.0` | **none — exact** | A1 via ⊗, bit-exact. A conventional model leaves extent unchanged or doubles by copying. |
| 1.3 **[D]** | bifurcation is geometric, not doubling | `2 ⊗ 2 = 20.9706` | `1e-9` | Copying gives 4. Self-⊗ squares, so it diverges much faster than a unit step would. |
| 1.4 **[D]** | over-deep document is **refused** | `Err(TooDeep)`, limit **3** | none | Refuse, never truncate — losing content silently is worse than declining. |
| 1.5 | the ceiling is derived, not hardcoded | matches live ⊗ iteration | none | If the operator's domain changes, the limit follows. |
| 1.6 | empty text is an empty crystal | not an error | none | — |
| 1.7 | node phases are A2's pair | `±π/2` only | none | — |

> **Correction to 1.3/1.4.** This plan first specified extent `4.828` and a ceiling of **4**, both taken from a probe that iterated `e ⊗ 1` — a *unit step*, not a bifurcation. A1 defines a bifurcation as `u ⊗ u`, which squares the product and exits the domain one level sooner. The implementation was correct; the plan's numbers were not, and the tests caught it by failing against the real operator.

## Group 2 — holographic

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 2.1 **[D]** | **Parseval holds** | spatial = frequency energy | `1e-9` rel | What makes this a representation rather than a summary. Hand-computed energy `425.0`. |
| 2.2 | DC term is the pixel sum | exact match | `1e-9` | — |
| 2.3 **[D]** | the transform round-trips | grid recovered | `1e-9` | A map that could not be inverted would describe the image, not be it. |
| 2.4 **[D]** | projection covers **four** faces | 4 faces, no coefficient lost | none | A Tetryen has four faces; the array type carries it. |
| 2.5 | uneven projection refused | `Err(UnevenProjection)` | none | Better than handing one face extra and calling it balanced. |
| 2.6 | malformed grid refused | `Err(MalformedGrid)` | none | — |
| 2.7 | a flat image is pure DC | all other coefficients vanish | `1e-9` | Confirms the transform is doing real work, not returning noise. |

## Group 3 — resonant

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 3.1 | a tone maps near its frequency | 200 Hz ± 2% | 2% rel | Zero-crossing estimate; honest about being one. |
| 3.2 | silence oscillates at nothing | `0.0` | exact | — |
| 3.3 | empty media refused | `Err(EmptyMedia)` | none | An empty stream has no frequency at all. |
| 3.4 | reports **ordinary** frequency | compile-time | — | Cannot reach the angular carrier path. |

## Not tested, because not built

**Volumetric time-crystals.** PRD §8's second reading of media has no definition in `_mkb/` and no paper defines it. There is no type, so there is nothing to assert — and a stub would imply a semantics nobody specified.

## Human check

Read 1.4 and 2.1. The first is the systemic ⊗ ceiling appearing in a third subsystem; the second is what separates a representation from a summary.
