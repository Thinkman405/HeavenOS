---
type: test-plan
subsystem: symphony-kernel
stage: 03_tests
derived_from: ["../02_design/output/design.md", "../01_derive/output/math-contract.md"]
doctrine: _mkb/test-doctrine.md
---

# Symphony-kernel — Test Plan

Every value below was computed and verified before this plan was written.

Target: `neos/tests/symphony_kernel.rs`.

**[D]** marks assertions a conventional implementation could not pass — a Planck-constant quantizer, a priority-queue scheduler, or a heuristic load balancer.

## Group 1 — quantization

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 1.1 | `C_H` equals `h/√(2π)` | `2.6434195357408632e-34` | `1e-45` | Contract §2. Sourced from JSON, checked against the closed form. |
| 1.2 **[D]** | `C_H` is neither `h` nor `ħ` | differs from both | none | The deliberate departure of R5a. A Planck-based quantizer fails here. |
| 1.3 | `E(1 GHz)` | `2.643419535740863e-25` J | `1e-12` rel | Hand-computed. |
| 1.4 **[D]** | `E = C_H·ν` is `0.3989×` Planck `hν` | `0.3989422804014327` | `1e-12` rel | Pins the intended ratio so nobody "corrects" `C_H` toward `h`. |
| 1.5 | energy is linear in `ν` | `E(2ν) = 2E(ν)` | `1e-12` rel | — |
| 1.6 | `ν → 0` ⇒ reclaimable | `is_reclaimable` true | none | Contract §2 — GC falls out of the equation. |
| 1.7 **[D]** | `ν`/`ω` cannot be confused | `to_angular` = `×2π` | `1e-12` rel | The type separation is compile-time; this pins the conversion. Feeding `ω` to `energy()` **must not compile** — noted in the log, not testable at runtime. |

## Group 2 — resonance

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 2.1 | `ξ(R) = 1` | `1.0` | **none — exact** | Contract §3. Verified bit-exact. A correction factor must be unity at reference. |
| 2.2 **[D]** | `ξ` is bounded | `< e/sinh(1) = 2.3130352854993315` | none | **The safety property.** Sampled across `[0, 30]`. The rejected `sinh(1)/sinh(r/R)` form reaches 1.2e6 near zero and fails this. |
| 2.3 | `ξ(0)` is the supremum, not `NaN` | `2.3130352854993315` | `1e-12` | `0/0` handled by limit. Contract §3. |
| 2.4 | `ξ` strictly decreasing | monotone over `[0,30]` | none | Verified. |
| 2.5 | `ξ(r < 0)` rejected | `Err(UndefinedScale)` | none | Genuinely outside the domain, unlike `r = 0`. |
| 2.6 | `H(κ)` converges under damping | `residual → 0` | `1e-9` | Contract §4 — the health invariant. |
| 2.7 | `H(κ)` diverging is detected | `is_converging` false | none | The alarm condition. A monitor that never fires is not a monitor. |

## Group 3 — equilibrium

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 3.1 | task density is mean-centred | `Σρᵢ = 0` | `1e-12` | Contract §5.1 — the solvability condition. |
| 3.2 **[D]** | load converges to uniform | spread `< 1e-9` | — | Verified at N = 7/16/31/64 on real `lattice` adjacency: `2.7e-15` … `2.3e-14`. |
| 3.3 | total load conserved | exact | `1e-9` | The Laplacian is conservative. A balancer that loses work is worse than none. |
| 3.4 | `α ≥ 2/λ_max` rejected | `Err(Unstable)` | none | Contract §5.2. Hardcoding `α` is the failure this prevents. |
| 3.5 | `stability_bound` from topology | `2/(2·d_max)` | `1e-12` | Must be derived, never constant. |
| 3.6 **[D]** | boundary cores have lower degree | min `< max` | none | A bounded patch is not vertex-transitive — degree 1 at the boundary, 5 in the interior. Verified. A balancer assuming uniform degree 5 mis-weights the boundary. |

## Group 4 — topology from `lattice`

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 4.1 | adjacency comes from `lattice` | in-patch neighbours ⊆ `Cell::neighbors()` | none | Contract §6 — consumed, not reimplemented. |
| 4.2 | adjacency symmetric | no asymmetric pair | none | Inherited from `lattice`; re-asserted because the patch restriction could break it. |
| 4.3 | interior degree is 5 | max degree = 5 | none | `lattice`'s guarantee surviving the patch. |
| 4.4 | topology is connected | one component | none | A disconnected patch makes `L`'s nullspace larger than the constants and silently breaks §5.1. |

## Tolerance notes

Two assertions need **no tolerance**: 2.1 (`ξ(R) = 1` bit-exact) and 2.2 (a bound, not a comparison). No tolerance here was chosen by reaching for `f64::EPSILON`.

## Deliberate omissions

- **Deadlock detection** — required by contract §8, not built in this slice. Its absence is recorded in the design, not hidden.
- **A1 bifurcation and A2 phase branching** — need the runtime task model `symphony-lang` will define.

## Human check

Read 2.2 and 3.2. The first is the property that keeps a bad timing sample from stalling the scheduler; the second is the claim that the field model actually balances load, tested against real tiling adjacency rather than a toy ring.
