---
type: subsystem
subsystem: symphony
tier: 2
language: custom DSL
stage: 01_derive
status: not-started
prd_sections: ["3", "4"]
binds_axioms: ["A1", "A2"]
unblocked_by: _mkb/resonance.md
---

# Symphony — the kernel logic DSL

One job: execute kernel logic through geometric rules — interference, phase shift, scale modulation — instead of Boolean operators, and schedule processes as energy states rather than time slices.

## The build loop

| Stage | Job | Output |
|---|---|---|
| `01_derive` | pull the exact law that binds this subsystem | `math-contract.md` |
| `02_design` | types and interfaces against that contract | `design.md` |
| `03_tests` | physics assertions, written before code | `test-plan.md` |
| `04_implement` | write the DSL + kernel into `neos/symphony/` | `implementation-log.md` |

## Scope

**Owns:** `neos/symphony/**` — `compiler/`, `interpreter/`, `kernel/scheduler.rs`, `kernel/quantization.rs`
**PRD sections:** §3 (Symphony Layer), §4 (Kernel and Resource Management)
**Axioms that bind it:** A1 (process bifurcation, $1\times1=2$), A2 (phase-based branching, no `bool`)
**Equations that bind it:** Howard Equation $E = C_H\omega$ (quantization + GC); Harmonic Force Equilibrium $\nabla\cdot\mathbf{E} = \rho/\epsilon_0$ (scheduling); Resonance Correction $\xi(r)$ (jitter damping)
**Constants read:** `howard_comma`, `logic_phases`, `resonance.*`

## Unblocked

Previously blocked: Harmonic Force Equilibrium had no execution rule, and the Howard Comma had four conflicting definitions one of which ($C_H \approx 0$) would have collapsed the scheduler.

**Both closed** — see [`_mkb/resonance.md`](../../_mkb/resonance.md) and [reconciliation.md R5/R6](../../_mkb/reconciliation.md).

Four constraints from that resolution bind this subsystem and are not negotiable:

- **Mean-centre task density.** `Σρᵢ = 0` is the solvability condition for the field equation, not a modelling preference. Absolute load has no solution.
- **Derive the coupling from topology.** `α < 2/λ_max(L)`. A hardcoded `α` will oscillate on some core count — which is the thrashing the model exists to prevent.
- **`ξ(r)` is bounded** above by `1.1565176427496657` and must stay so. It sits in the clock path; an unbounded correction is worse than none.
- **Deadlock detection is still required.** Load equilibrium eliminates thrashing and bottlenecks, not circular waits on resource acquisition. Four balanced cores still deadlock on two locks taken in opposite orders.

## Scale note

This is the largest subsystem — a language (parser, compiler, interpreter) *and* a kernel. If `02_design` runs long, that is the signal to split this record into two: `symphony-lang` and `symphony-kernel`. Splitting later is cheap; a stage doing two jobs is not.

## Do not

Load other subsystems' records. They don't share state; they share the factory (`_mkb/`, `_spec/`).
