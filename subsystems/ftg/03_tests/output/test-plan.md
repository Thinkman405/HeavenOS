---
type: test-plan
subsystem: ftg
stage: 03_tests
derived_from: ["../02_design/output/design.md", "../01_derive/output/math-contract.md"]
doctrine: _mkb/test-doctrine.md
---

# FTG — Test Plan

Target: `neos/tests/ftg.rs`. Every value below was measured before this plan was written.

**[D]** marks assertions a conventional networking stack could not pass — a CRC-checked, table-routed, message-terminated implementation.

## Group 1 — Layer 1/2 framing

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 1.1 | frame round trip | `encode → decode` identity | none | Over varied payloads including `0x00`, `0xFF`, random. |
| 1.2 **[D]** | clean frame dissonance is **zero** | `0.0` | **none — exact** | Measured exactly. `sin(±π/2) = ±1` and each symbol is cancelled by its complement, so this is exact by construction, not by rounding. |
| 1.3 **[D]** | any single flip gives dissonance **2.0** | `2.0` | **none — exact** | Verified for **every** symbol position, not a sample. A CRC gives a hash mismatch, not an amplitude. |
| 1.4 **[D]** | a dissonant frame cannot be decoded | `Err(Dissonant)` | none | Contract §2.2 — dissipate, never repair. There is no lossy path to test because none exists. |
| 1.5 | **the detection blind spot is real** | correlated flip → dissonance `0.0`, decode **succeeds** | none | Contract §2.3. Asserting the *limitation* so it stays documented and cannot be quietly claimed away. |
| 1.6 | frame length is twice payload bits | `2 × 8 × len` | none | The complement structure, made explicit. |

## Group 2 — Layer 3/4 routing

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 2.1 | address→cell is deterministic | same address → same cell, 1000× | none | The one property the mapping must have. |
| 2.2 | address→cell is total | every sampled address maps in-patch | none | — |
| 2.3 **[D]** | greedy descent always arrives | 100% over ≥2000 pairs | none | Measured 4000/4000. Greedy routing gets stuck on general graphs; that it never does here is the hyperbolic embedding's payoff. |
| 2.4 **[D]** | greedy is **BFS-optimal** | 0 routes longer than BFS | none | Measured 0/400. The sharper claim: descent finds a *shortest* path, not merely some path. Asserted separately from 2.3 so a degradation to "arrives" is visible. |
| 2.4b **[D]** | **any** strict descent is optimal | worst-choice walk still matches BFS | none | Added after the sabotage gate — see the correction note below. Also asserts that branching genuinely occurs (~16% of steps), so the test cannot pass vacuously. |

> **Correction, found by the sabotage gate.** The plan originally implied that picking the *closest* neighbour was load-bearing for optimality. Sabotaging `min_by` to "any closer neighbour" broke **nothing** — all tests still passed. Measurement showed why: 42% of steps have more than one descending option, yet deliberately taking the **worst** still yields a shortest path (0 suboptimal in 1497 routes). `min_by` provides determinism, not correctness. What is actually load-bearing is the **strictness** of the descent — sabotaging that is caught, by the stranded-packet test. Row 2.4b now asserts the real invariant.
| 2.5 | every hop is an edge | consecutive cells adjacent | none | Guards against a "route" that teleports. |
| 2.6 | each hop strictly decreases distance | monotone descent | none | The definition of descent; catches a router that wanders. |
| 2.7 | route to self is trivial | length 1, no hops | none | — |
| 2.8 | no descent ⇒ error, not a loop | `Err(NoDescent)` | none | Contract §3.3 caveat. Constructed by routing toward a cell outside the patch. |
| 2.9 | hop limit is enforced | `Err(HopLimit)` | none | Second guard: a bug must not hang the caller. |

## Group 3 — harmonic multiplexing

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 3.1 | port *n* → `(n+1)·ω_c` | exact ratio | `1e-9` rel | — |
| 3.2 **[D]** | distinct ports are orthogonal | inner product ≈ 0 | `1e-12` | Measured `~1e-17`; ε allows for sampling error. This is what makes ports independent channels rather than a shared medium. |
| 3.3 | self-overlap is `0.5` | `0.5` | `1e-6` | Mean of `cos²`. Confirms the integrator is correct, so 3.2's zero means orthogonality rather than a broken sum. |
| 3.4 | overtone is `AngularFrequency` | compile-time | — | Cannot reach `E = C_H·ν`. |

## Group 4 — §7 session lifecycle

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 4.1 | aligned oscillators lock | `Ok`, `is_resonant` | none | — |
| 4.2 | variance `≥ π/4` refuses to lock | `Err(NoLock)` | none | Bound is **strict**; verified exclusive at the boundary. Tested just below, at, and just above. |
| 4.3 | standing wave matches `2A sin(kx)cos(ωt)` | closed form | `1e-12` | — |
| 4.4 | a collapsed link cannot be reused | `Err(Collapsed)` | none | A connection surviving amplitude zero would contradict the physics. |

## Group 5 — **Test Case 1** (canonical doctrine)

The last unimplemented canonical test in `_mkb/test-doctrine.md`.

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 5.1 **[D]** | A at `φ=0`, B at `φ=π` ⇒ superposition **zero** | ≈ 0 | `cancellation_floor(ω,t)` | Doctrine wording: "must evaluate to absolute zero." True zero is unavailable in IEEE-754 because `x + π` **rounds**. See the correction below. |
| 5.2 **[D]** | cancellation holds at **every** `t` | 2000 instants over 40 periods | `cancellation_floor(ω,t)` | Continuity is what lets teardown work without an acknowledgement. Swept across many periods, so the residual's growth is actually exercised. |
| 5.2b | the floor **scales** with `ω·t` | far ≫ near | none | Pins the reason a constant cannot be used. Fails loudly if someone substitutes a fixed number. |
| 5.3 | teardown drives amplitude to zero | residual within floor | `cancellation_floor(ω,t)` | `teardown()` returns the residual, so the assertion is on the real value rather than on the call having been made. |
| 5.4 | teardown shift is exactly `π` | from `constants.json` | none | Uses `thresholds.teardown_phase_shift`; not a literal. Also asserted in `build.rs`, which fails the build if the MKB value drifts off `π`. |

> **Correction: the tolerance is not a constant.** This plan first specified `1.2e-16`, taken from one measurement. The implementation failed against it — measured `5.55e-16` at a larger `t`, and a peak of `1.08e-14` sweeping 40 periods.
>
> The cause is arithmetic, not physics: `cos(x)` and `cos(x + π)` are exact negatives analytically, but **`x + π` rounds**, and that absolute error grows with `|x|`. Since `|d(cos)/dx| ≤ 1`, it transfers to the result roughly one-for-one, giving `residual ~ ε·|ω·t|`.
>
> The tolerance is therefore a function, `ftg::cancellation_floor(ω, t)`, living in the library so the test and any caller share one derivation. A fixed floor chosen from a single sample passes at that sample and fails elsewhere — which is exactly what happened.

## Group 6 — consumed, not reimplemented

| # | Assertion | Justification |
|---|---|---|
| 6.1 | routing distance matches `lattice`'s metric | Cross-checked directly against `lattice`, as `substrate` does. |
| 6.2 | bit/phase mapping matches `substrate`'s | Same values, one home. |
| 6.3 | no MKB constant literal in `ftg` source | `grep` in verification, not a Rust test. |

## Tolerance notes

**Six assertions need no tolerance at all** — 1.2, 1.3, 1.4, 2.3, 2.4, 4.2. The two exact-value ones (dissonance 0.0 and 2.0) are exact by construction rather than by luck, because `sin(±π/2)` is exactly `±1` in IEEE-754.

Where tolerance is required it is **measured, not guessed**: `1.2e-16` sits just above an observed `1.11e-16`, and `1e-12` on orthogonality covers sampling error over a `~1e-17` true value.

## Deliberate omissions

Socket I/O, fragmentation, retransmission, and §8 crystallisation — all recorded in the design with reasons.

## Human check

Read 2.4 and 1.5. The first is the claim that hyperbolic routing is *optimal*, not just functional — the strongest result in this subsystem. The second asserts a **limitation**, so the frame check's blind spot stays visible instead of being rounded up to "detects corruption."
