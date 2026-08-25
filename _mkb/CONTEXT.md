---
type: router
layer: L3
---

# _mkb — the Mathematical Knowledge Base

The factory. Stable across every subsystem build. This is the law NEOS code must obey; nothing in here is per-run or per-subsystem.

## What's here

| File | Holds | Load when |
|---|---|---|
| [axioms.md](axioms.md) | the 3 overrides + agent directives | **always** — first file of any build step |
| [reconciliation.md](reconciliation.md) | which definition won, and why | **always** — before trusting any formula |
| [operators.md](operators.md) | ⊗ ⊕ ⊘ and powers, reconciled | any arithmetic on lattice values |
| [constants.md](constants.md) | $C_H$, $\omega_c$, $K$ explained | always |
| [constants.json](constants.json) | the authoritative numeric values | when code needs a number |
| [equations.md](equations.md) | the wave equations + execution rules | when deriving a subsystem's math contract |
| [tetryen.md](tetryen.md) | the core geometric primitive | rendering or embedding geometry |
| [tetryen_recurrence.md](tetryen_recurrence.md) | discrete time evolution of Tetryen node states | animating/stepping a Tetryen's own wave state over time |
| [resonance.md](resonance.md) | timing correction + load equilibrium | scheduling, clock domain, quantization |
| [gates.md](gates.md) | the 3 geometric logic gates, derived | any branching or conditional logic |
| [instruction_set.md](instruction_set.md) | the full symphony-lang ISA (EVAL/RESONATE/SHIFT/FORK/EMIT/STORE/LOAD/ACQUIRE/RELEASE/HALT) | building or extending symphony-lang's instruction-executing state machine |
| [timecrystal.md](timecrystal.md) | media as quantised 4D spatiotemporal structure | crystallising audio/video |
| [test-doctrine.md](test-doctrine.md) | physics-based TDD rules | when writing `03_tests` |
| [schemas/](schemas/) | Tetryen Node JSON schema | when defining data structures |
| [papers/](papers/_index.md) | 8 source PDFs | **rarely** — only to settle a contested derivation |

## Loading discipline

A subsystem build step loads: `axioms.md` + `constants.md` + the *specific* equations that bind its subsystem. Not this whole folder. The `Binds:` line on each equation tells you which subsystems care about it — use it to filter.

Never load `papers/` into a routine build step. That is what the distilled layer is for.

## Distillation vs synthesis

Two files here are **not** distilled from any paper, and say so in their own frontmatter:

| File | Synthesised from | Closes |
|---|---|---|
| [timecrystal.md](timecrystal.md) | `C_H` + Tetryen geometry | PRD §8's second reading of media |
| [gates.md](gates.md) | A2 + teardown shift + standing-wave variance + `ξ(r)` | PRD §3's second and third logic gates |
| [tetryen_recurrence.md](tetryen_recurrence.md) | Tetryen node dynamics + the real hyperbolic metric | the undistilled finite-universe paper's `f(ψ_n, ψ_{n-1})` placeholder |
| [instruction_set.md](instruction_set.md) | gates.md + A1 + A3 + substrate's memory API + the kernel's resource tracker | a conversation-borne virtualization proposal that would have broken A2 |

The corpus names things it never defines. Where the named thing can be *derived* by composing law that already exists, deriving it is right and leaving a hole is not — the system has to be whole to run. Where it cannot, the gap is recorded rather than invented; that is why `{5,4}`-in-H⁴ is still an open scope boundary and not a guessed Schläfli symbol.

The test is whether every step is a composition of existing law, evaluated rather than asserted. Both files above carry their verification inline.

## The precedence rule

axioms > reconciliation > operators/equations > constants.json > spec > code > **papers**

Papers rank **last**, below code. They are exploratory work containing mutually incompatible definitions and several formulas that do not evaluate to their stated results. [reconciliation.md](reconciliation.md) is the single consistent reading extracted from them — implement from it, never from a paper directly.

If a PRD statement in [`_spec/prd.md`](../_spec/prd.md) contradicts an axiom here, the axiom wins and the PRD needs correcting. If code contradicts anything here, the code is wrong.

## Human check

`constants.md` carries no numeric values by design — only names, JSON keys, and meaning. If you ever find a number written into the prose, delete it and point at the key instead. That is the one edit most likely to rot this folder.
