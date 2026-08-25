---
type: test-doctrine
layer: law
status: canonical
---

# Physics-Based TDD Doctrine

Unit tests evaluate correctness via **physical wave interaction properties**, not binary equality checks. A test asserting `assert_eq!(x, true)` is a doctrine violation regardless of whether it passes.

Tests are written **before** implementation. That is why every subsystem record has `03_tests/` ahead of `04_implement/` — the human gate on the assertions is the cheapest place to catch a misread of the math.

## The rule

| Instead of asserting | Assert |
|---|---|
| equality of values | superposition sum reaching a target amplitude |
| a boolean flag | phase alignment within threshold |
| a counter incremented | bifurcation produced the correct structural split |
| resource freed | $\omega \to 0$ and therefore $E \to 0$ |

## Canonical test cases

These two are the reference shape. Every subsystem's `test-plan.md` derives its assertions in this style.

### Test Case 1 — Destructive Interference Teardown

- **Input:** Waveform $A$ at phase $\phi = 0$; Waveform $B$ at phase $\phi = \pi$.
- **Assertion:** Superposition sum must evaluate to absolute zero, triggering instantaneous resource reclamation ($E = 0$).
- **Proves:** [A2](axioms.md#a2--logic-gate-override), Phase Inversion Teardown in [equations.md](equations.md#phase-inversion-teardown)
- **Target:** [[ftg]] — specifically its **session** concern (PRD §7), not Layer 3/4 routing
- **Status:** ✅ implemented — `test_case_1_destructive_interference_teardown` in `neos/tests/ftg.rs`, with `cancellation_is_continuous_not_sampled` extending it across 40 periods. Tolerance is `ftg::cancellation_floor(ω, t)`, which **scales with the phase argument** because `x + π` rounds; a constant floor was tried first and failed.

**Both canonical tests are now implemented.**

### Test Case 2 — Lynchpin Bifurcation Fork

- **Input:** Execution of process split with unit parameter $1 \times 1$.
- **Assertion:** Resulting child process count *and* address space scale must equal $2$, confirming non-linear multiplicative override.
- **Proves:** [A1](axioms.md#a1--multiplicative-identity-override)
- **Target:** [[symphony-kernel]]
- **Status:** ✅ implemented — `unit_fork_yields_exactly_two` in `neos/tests/symphony_scheduler.rs` asserts both `children == 2.0` and `address_scale == 2.0`, exactly as worded above. Also satisfied independently by [[substrate]]'s `unit_pool_split_is_exactly_two` for memory-pool extent.

> **Retargeting note.** Test Case 2 previously named `[[symphony]]`, a record retired when it split into `symphony-kernel` and `symphony-lang`. Bifurcation is kernel work, so the target is now `symphony-kernel`. A doctrine pointing at a dead record is the "schema mandating names the files stopped using" failure — in the law layer, where it matters most.

## Floating-point caveat

"Absolute zero" and "exactly $2$" are physical claims meeting IEEE-754 reality. Each subsystem's `test-plan.md` must state its chosen epsilon and justify it against the amplitude scale in play. Do not silently pick `f64::EPSILON` — state the tolerance, and state why a value inside it counts as dissipated.

This is the single most likely place for the doctrine to quietly degrade into conventional equality testing. Watch it.
