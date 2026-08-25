---
type: test-plan
subsystem: substrate
stage: 03_tests
derived_from: ["../02_design/output/design.md", "../01_derive/output/math-contract.md"]
doctrine: _mkb/test-doctrine.md
---

# Substrate — Test Plan

Target: `neos/tests/substrate.rs`. Values verified before writing.

**[D]** marks assertions a conventional flat-memory, byte-addressed implementation could not pass.

## Group 1 — the flat/curved boundary

The subsystem's reason to exist. The strongest guarantee here is **compile-time** and therefore not a runtime test; it is verified by a probe and recorded in the log.

| # | Assertion | Expected | Justification |
|---|---|---|---|
| 1.1 **[D]** | no public flat address exists | probe does not compile | `FlatOffset` is private; `MemoryPool` exposes no ptr/slice/`usize` index. Verified by compile probe, like the `ν`/`ω` separation. |
| 1.2 **[D]** | address distance is hyperbolic, not arithmetic | `distance` = `d_H` between cell centres | A Euclidean implementation returns `|a − b|`. Cross-checked against `lattice`'s metric directly. |
| 1.3 **[D]** | distance is not the offset difference | differs from any linear measure | Guards against a "geometric" API backed by arithmetic. |
| 1.4 | `distance(a, a) == 0` | exact | — |

## Group 2 — allocation locality

| # | Assertion | Expected | Justification |
|---|---|---|---|
| 2.1 **[D]** | multi-cell allocations occupy **adjacent** cells | every cell adjacent to a predecessor | Contract §3. This is the property `ftg` routing depends on. A flat allocator gives consecutive indices, which are not adjacent in the lattice. |
| 2.2 | allocation cells are distinct | no repeats | — |
| 2.3 | read-back equals written bytes | round trip | Data integrity across the boundary. |
| 2.4 | writes to one allocation do not disturb another | isolation | — |
| 2.5 | over-capacity request fails | `Err(Exhausted)` | Not a panic, not silent truncation. |
| 2.6 | offset beyond cell capacity rejected | `Err(OffsetOutOfCell)` | — |
| 2.7 | freed cells become reusable | allocate → free → allocate succeeds | — |

## Group 3 — pool splitting (A1)

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 3.1 **[D]** | unit split yields exactly 2 | `2.0` | **none — exact** | A1 via `lattice`'s `⊗`. Scalar duplication gives 1. |
| 3.2 **[D]** | split is not doubling by copy | address scale from `⊗`, not `×2` | none | For a non-unit pool `u⊗u ≠ 2u`, which a copy-based implementation cannot reproduce. |
| 3.3 | split respects `⊗`'s domain | `Err(SplitDomain)` past 805.56 | none | Enforced, not documented. |

## Group 4 — binary ↔ wave translation

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 4.1 | bit 0 → `−π/2`, bit 1 → `+π/2` | from MKB `logic_phases` | `1e-15` | A2. |
| 4.2 | round trip is lossless | `bits → phases → bits` identity | none | Over varied byte patterns including `0x00`, `0xFF`, and random. Any loss corrupts every layer above. |
| 4.3 **[D]** | **zero crossing recovers nothing** | `Err(ZeroCrossing)` at `t = 0` and half periods | none | Contract §5.1 — measured separation is exactly 0 there. Must error, not return garbage. This is the finding most likely to have shipped as intermittent corruption. |
| 4.4 **[D]** | quarter-period separation is maximal | `2.0` | `1e-12` | Verified. Confirms the sampling rule is the right one, not merely a safe one. |
| 4.5 | demodulation at a quarter period is exact | round trip via carrier | none | The end-to-end pipeline. |
| 4.6 **[D]** | opposite phases cancel for **all** `t` | `≈ 0` at many `t` | `1e-15` | Verified to ~1e-16. Cancellation is continuous, not sampled — what makes phase teardown work without an ack. |
| 4.7 | off-axis phase rejected | `Err(IndeterminatePhase)` | none | A2 admits exactly two orientations; anything else must not be rounded into one. |

## Group 5 — clock and frequency types

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 5.1 | `ω_c` from MKB | `6283185307.179586` | `1e-6` | — |
| 5.2 | `ω_c` is angular-typed | compile-time | — | Cannot reach `E = C_H·ν`. Verified by the same probe as 1.1. |
| 5.3 | quarter period matches `ω_c` | `2.5e-10 s` | `1e-22` | — |
| 5.4 | `tick` advances one quarter period | uptime accumulates | `1e-18` | Clock and demodulator agree by construction. |
| 5.5 | one home for frequency types | `symphony_kernel::Frequency` **is** `substrate::Frequency` | none | Asserted by cross-crate assignment; two distinct types would not compile. |

## Tolerance notes

Seven assertions need **no tolerance**: 1.4, 3.1, 3.2, 3.3, 4.2, 4.3, 4.5, 4.7. Exactness is available here because the phase pair and `⊗`'s unit case are exact by construction.

## Deliberate omissions

- **Virtualisation proper** (trapping, guest isolation) — needs the Symphony instruction model.
- **Concurrency** — `MemoryPool` is single-threaded; where locks live is a scheduler decision.
- **Fractal area preservation on resize** — PRD §5 assigns it to `lattice`, which has not built it.

## Human check

Read 4.3 and 2.1. The first stops silent bit corruption at every layer above; the second is what makes `ftg`'s hyperbolic routing meaningful rather than decorative.
