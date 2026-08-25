---
type: math-contract
subsystem: symphony-lang
stage: 01_derive
status: complete
prd_sections: ["3"]
binds_axioms: ["A1", "A2"]
consumes: [symphony-kernel]
---

# Symphony-lang — Math Contract

## What this record is downstream of

Almost nothing in `_mkb/` directly. That is the point of the deferral.

A language does not have its own physics. Every quantity it manipulates is priced, split, or evaluated by [[symphony-kernel]], which has already distilled the relevant law:

| Law | Home | Reached through |
|---|---|---|
| A1 — `1 ⊗ 1 = 2` | [`_mkb/axioms.md`](../../../../_mkb/axioms.md) | `symphony_kernel::fork` |
| A2 — phase logic `φ ∈ {−π/2, +π/2}` | [`_mkb/axioms.md`](../../../../_mkb/axioms.md) | `symphony_kernel::{Phase, evaluate_branch}` |
| `E = C_H·ν` | [`_mkb/resonance.md`](../../../../_mkb/resonance.md) | `symphony_kernel::Task::energy_joules` |
| ⊗ domain limit | [`_mkb/operators.md`](../../../../_mkb/operators.md) | `fork` returning `Err` |

**This record derives no new mathematics.** If it did, that would be evidence the split was wrong — the kernel would have left law on the table.

## §0 — Slice 2 addendum: all three gates

The first pass of this contract read PRD §3 as *"no Boolean operators"* and built the interference gate. That was **an under-reading of the section it derives from.** §3 names three gates:

> *"constructive/destructive interference, **phase shifts**, and **scale modulation** as logic gates."*

Only the first had an operational definition anywhere in the corpus. The other two were named and undefined — the same shape of gap as PRD §8's volumetric time crystal, and closed the same way: by **synthesis** from law that already exists, recorded in [`_mkb/gates.md`](../../../../_mkb/gates.md) as a synthesis rather than a distillation.

| Gate | PRD §3 term | Derived from |
|---|---|---|
| 1 | interference | already law — `evaluate_branch` |
| 2 | phase shift | A2's orientations are exactly a teardown `π` apart |
| 3 | scale modulation | `ξ(r)` × the standing-wave `±π/4` criterion → band `1/8` |

Sections §6 and §7 below carry the contract terms. Nothing in §1–§5 changed.

## §1 — The binding constraint: A2 is syntactic

A2 states that logic states are phase orientations rather than Boolean values. Every other subsystem honours this *negatively* — by not defining a `bool`-shaped type. `symphony_kernel::Phase` goes as far as a type can: it has no `From<bool>`, no `Into<bool>`, and exactly two inhabitants.

A language can go further, and this is the one place in NEOS where it can:

> **Contract §1.1** — `true`, `false`, `if`, `else`, `&&`, `||`, `!`, `==`, `!=`, `and`, `or`, `not`, and `bool` are **not tokens of this language**. Encountering any of them is an error that names A2.

> **Contract §1.2** — There is no Boolean expression grammar. Not "unused"; absent. A programmer who wants a conditional has exactly one construct available, and it is phase interference.

> **Contract §1.3** — The A2 refusal fires **before** parsing and before name resolution. A programmer must not be able to fix a syntax error and only then discover the construct was forbidden.

§1.2 is what makes §1.1 more than a blacklist. Banning `if` while providing `cond ? a : b` would be theatre.

## §2 — Branching (A2)

> **Contract §2.1** — The only conditional is `when A ⟨aligns|opposes⟩ B { … }`, where `A` and `B` name declared oscillators. The predicate is `symphony_kernel::evaluate_branch(A.guard_phase(), B.guard_phase())`.

> **Contract §2.2** — `aligns` runs its body on `Interference::Constructive`; `opposes` on `Interference::Destructive`. The language does not compute interference; it asks.

> **Contract §2.3** — Interference is **symmetric**: `when A aligns B` and `when B aligns A` are the same test. Superposition does not order its operands.

> **Contract §2.4** — Per operand pair, the two forms **partition**. A2 admits exactly two orientations, so alignment is a two-valued predicate and `opposes` is the complement of `aligns` on the same pair.

§2.4 is stated because it is easy to over-claim the opposite. What distinguishes this from `if`/`else` is *structural*, not per-branch:

> **Contract §2.5** — The two forms are **independent statements**, not arms of one conditional. A program may take both, or neither. `if`/`else` can express neither outcome, because exactly one arm always runs.

## §3 — Bifurcation (A1)

> **Contract §3.1** — `fork A` calls `symphony_kernel::fork(A.fork_unit())`. The child count is `u ⊗ u`, never a literal `2` written in this crate.

> **Contract §3.2** — Every task the surface syntax declares has `fork_unit = 1`, so the count is exactly `2.0` — bit-exact, since `sinh(arcsinh 1) = 1` identically.

> **Contract §3.3** — A non-integral child count is **refused**, not rounded. `2 ⊗ 2 = 20.9706…`; there is no such thing as `0.97` of an execution unit. This is the same refusal discipline the ⊗ domain limit already uses.

> **Contract §3.4** — A fork unit outside ⊗'s domain surfaces as a language error, not a panic and not a clamp. **This is the fourth subsystem in which the ⊗ ceiling appears** (`lattice` addressing, `symphony-kernel`'s domain guard, `crystallisation` bifurcation depth, and now here).

## §4 — Energy (`E = C_H·ν`)

> **Contract §4.1** — A declared task's frequency must be finite and strictly positive. `E = C_H·ν` gives negative energy for `ν < 0`, and a task at `ν = 0` is born reclaimable — it would be admitted and immediately swept. Both are refused at declaration.

> **Contract §4.2** — The language never multiplies by `C_H`. Emitted work is `symphony_kernel::Task`, and cost is whatever the kernel says it is.

> **Contract §4.3** — Program energy is a property of the emitted oscillators, not of the control flow that selected them. Two programs emitting the same set cost the same.

## §5 — The seam

> **Contract §5.1** — The language supplies the first implementor of `symphony_kernel::bifurcation::TaskModel`. That trait's own doc comment records that nothing depended on it yet and that it existed to fix the shape of the seam. **This record closes it.**

> **Contract §5.2** — Emitted work is the kernel's own `Task` type, so it reaches `Scheduler::ingest` without translation. A translation layer would be a second home for the definition of a task.

## §6 — Phase shift (gate 2)

> **Contract §6.1** — `invert A` applies the exact `π` shift, which is the map between A2's two orientations. Delegated to `symphony_kernel::Phase::invert`; the language does not compute it.

> **Contract §6.2** — The gate is **total**. A2's set is closed under the shift, so there is no failure case and no domain check. An implementation that can return an error here has left the axiom.

> **Contract §6.3** — Not implemented as `-φ`. For this set the two coincide numerically, but only because the orientations happen to be symmetric about zero; `-φ` is derived from nothing and would diverge if A2's orientations were ever re-centred.

> **Contract §6.4** — Inversion must change what the interference gate subsequently answers. The gates compose; an inversion that no downstream branch can observe is decorative.

## §7 — Scale modulation (gate 3)

> **Contract §7.1** — A task may declare an observation scale. Omitting it means the reference scale `R`, where `ξ(R) = 1` exactly — so the default is not a special case in the interpreter, it is the identity correction.

> **Contract §7.2** — Effective frequency is `ν·ξ(r)`. The language never pre-multiplies it: the scale is carried into `symphony_kernel::Task::with_scale`, and the kernel applies `ξ`. One home for the correction.

> **Contract §7.3** — `when A resonates B` / `detunes B` is decided by `symphony_kernel::resonates`, using the derived band `1/8`. **The band is never written into this crate.**

> **Contract §7.4** — **The gate is not a scale comparison.** A2 admits no relational operator, so `when A above B` is forbidden — it would be a Boolean relational operator wearing a geometric name. The gate asks whether a standing wave between the two would survive, which is two-valued for a physical reason.

> **Contract §7.5** — Refuse when the mean effective frequency is not positive and finite. `ξ(r) → 0` at large scale, so the detuning ratio becomes `0/0`. Refusal, not a default answer.

> **Contract §7.6** — A scale outside `ξ`'s domain (`[0, ∞)`) is refused at declaration. `r = 0` is **valid** and evaluates by limit to the supremum — it is a legal observation point, not an error.

## Forbidden

- Any Boolean type, operator, or literal anywhere in the crate's surface language.
- **A scale comparison gate.** See §7.4.
- **A gate combinator** (`resonates AND aligns`). It would need a truth table, which is exactly what A2 removes. Nesting a branch inside another already composes gates.
- Writing the resonance band, `ξ`, or the `π` shift into this crate. All three have one home, and none of them is here.
- Recomputing ⊗, `C_H`, interference, or fork multiplicity. All four have exactly one home, and none of them is here.
- Rounding a fractional child count.
- An expression grammar. It would immediately raise "what does `a == b` mean?", and under A2 the answer is *nothing* — so the question is not raised.

## Open questions — none

The deferral existed so the kernel could settle runtime semantics first. It has: `TaskModel` fixes the task shape, `evaluate_branch` fixes branching, `fork` fixes bifurcation, `Scheduler::ingest` fixes the hand-off. There is nothing left to wait for.

**Slice 2 note on "this record derives no new mathematics".** That claim was true of slice 1 and is still true here in the sense that matters — every quantity is computed by the kernel. But deriving *what the missing gates are* did require new law, and it went where new law goes: [`_mkb/gates.md`](../../../../_mkb/gates.md), upstream of this contract, not into the crate. The precedence cascade held.

## Deviation from the target tree, recorded

[`_spec/target-tree.md`](../../../../_spec/target-tree.md) lists `symphony/compiler/` and `symphony/interpreter/` as separate crates. This record builds **one crate**, `symphony/lang/`, with `lexer`, `parser`, and `interpreter` modules.

Reason: the two would share `Stmt`, `Token`, and `LangError` and would depend on each other in one direction only. Splitting them would produce a crate boundary carrying three type definitions and no independent consumer — the opposite of the surgical-split rule, which asks for splits along *distinct concerns*, not along stage names. Recorded here rather than done silently.
