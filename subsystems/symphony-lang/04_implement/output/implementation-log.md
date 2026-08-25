---
type: implementation-log
subsystem: symphony-lang
stage: 04_implement
status: complete
toolchain: rustc 1.97.1 / cargo 1.97.1
result: 69 passed, 0 failed (448 workspace-wide) — see instruction-executing state machine addendum
consumes: [symphony-kernel, substrate, lattice]
---

# Symphony-lang — Implementation Log

## Result

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 282 passed; 0 failed
cargo test  -p symphony-lang → 27 passed; 0 failed
```

**The seventh and last record.** Every subsystem in `_spec/architecture-map.md` now has working code.

## Files written

| Path | Role |
|---|---|
| `neos/symphony/lang/Cargo.toml` | manifest — `symphony-kernel` only |
| `src/lib.rs` | crate root, `LangError`, `run` |
| `src/lexer.rs` | tokeniser — **the A2 refusal lives here** |
| `src/parser.rs` | grammar → `Stmt` tree |
| `src/interpreter.rs` | `RuntimeTask`, `Execution`, the `TaskModel` impl |
| `neos/tests/symphony_lang.rs` | 27 assertions |

No `build.rs`. This crate reads no constants — every number it touches comes back from the kernel. That absence is the clearest evidence the split was drawn in the right place.

## The seam is closed

`symphony_kernel::bifurcation::TaskModel` has carried this doc comment since the kernel was written:

> *"Nothing in the kernel depends on this trait yet — it exists to fix the shape of the seam, not to be used."*

`RuntimeTask` is the first implementor. `runtime_task_implements_the_kernel_seam` takes it through a generic `fn takes_a_model<T: TaskModel>`, so the impl is exercised as a trait, not merely declared.

The shape the kernel guessed turned out to be right on all three methods. `frequency()` feeds `E = C_H·ν`, `guard_phase()` feeds `evaluate_branch`, and `fork_unit()` feeds `fork` — no method was unused, and none was missing. That is worth stating: the deferral was justified precisely because designing a language for an *imagined* runtime is how seams end up wrong, and this one was designed for a real one.

## A sabotage that broke nothing — and was right not to

The test plan predicted that treating `opposes` as `else` would fail Group 2. **It failed nothing. 26/26 still passed.**

The prediction was wrong, and the reason is a fact about A2:

```
Phase has exactly two inhabitants
  ⇒ evaluate_branch is a two-valued predicate
  ⇒ Constructive and Destructive partition
  ⇒ (Opposes, Destructive) ≡ ¬(Aligns, Constructive)
```

Per operand pair, `opposes` **is** the Boolean complement of `aligns`. The mutation was not a mutation; it was an equivalent implementation.

Two things changed as a result.

**The test was renamed and its claim corrected.** `opposes_is_not_the_complement_of_aligns` claimed something false. It is now two tests: `alignment_partitions_the_two_phase_orientations`, which asserts the complementarity outright across all four phase pairs, and `branch_forms_are_independent_statements`, which asserts the thing that is actually distinctive — that the two forms are *separate statements over separately chosen operands*, so a program can take both or neither. `if`/`else` can express neither outcome, because exactly one arm always runs.

**The interpreter kept the explicit pairing anyway**, with the reason written next to it. `match (alignment, interference)` and `!aligns_taken` compute the same function today; the pairing is kept because it follows from the *law* (constructive means aligned) rather than from `Interference` happening to have two variants.

This is the same shape as `crystallisation`'s `non_unitary_modulation_is_refused`: a test that looked like it was asserting a distinction, discovered by sabotage to be asserting nothing. **Sabotage caught it here before it shipped rather than after.**

## Doctrine checks — four performed

| Sabotage | Tests failed |
|---|---|
| disable the A2 refusal in the lexer | **3 of 26** |
| treat `opposes` as `else` | **0** — see above; not a mutation |
| run every branch body regardless of interference | **4 of 26** |
| truncate a fractional child count instead of refusing | **1 of 26** |

The first is the one that matters. With the refusal disabled, `boolean_constructs_are_refused_at_lex_time`, `the_a2_refusal_explains_itself`, and `a2_refusal_precedes_every_other_error` all go red — and `only_two_phase_literals_exist` correctly stays green, because it tests a different claim.

## A guard that would have been decoration

`LangError::NonIntegralFork` refuses a fractional child count. Surface syntax cannot reach it: every declared task forks at the canonical unit, where `1 ⊗ 1 = 2` exactly.

The first version of `a_fractional_child_count_is_refused` therefore asserted nothing about the guard — it computed `2 ⊗ 2` outside the interpreter, checked *that* was fractional, and confirmed the canonical program still worked. Precisely the pretence `crystallisation` had already been caught in.

Fixed by adding `execute_with(program, seed)`, an embedding seam letting a host supply tasks the source did not declare. The guard is now reachable, the test drives it, and the sabotage above confirms it bites. `seeding_does_not_open_a_hole_in_scoping` pins that seeding *adds* to scope rather than bypassing it.

The seam is not scaffolding invented for a test. `TaskModel` is a trait precisely so a host can supply its own tasks; `execute_with` is where that becomes possible.

## The ⊗ ceiling, fourth appearance

`lattice` addressing (unit steps, depth 4), `symphony-kernel` (domain guard), `crystallisation` (self-⊗ bifurcation, depth 3), and now `symphony-lang` (fork unit). Four subsystems, four arities, one constraint.

Consistent with the earlier finding: the *exact* depth depends on how fast the operands grow, and the constraint itself is systemic to iterating ⊗ against a fixed domain. Refused in all four places; clamped in none.

## Absolute-vs-relative, applied in advance

Exactly one tolerance in the suite, and it is relative from the first draft. Energies here are of order `1e-32` J, so an absolute threshold at any plausible magnitude would pass on nothing.

**This trap has cost four debugging sessions across this workspace** — `symphony-kernel` convergence, `ftg` cancellation, `gui` scale-free, `crystallisation` Floquet. It is the first time it was avoided rather than discovered. The rule that now holds: *a threshold has to be expressed in the units of the thing it bounds.*

## A2 enforced by refusing to tokenise

Every other subsystem honours A2 negatively, by not defining a `bool`-shaped type. `Phase` goes as far as a type can — no `From<bool>`, no `Into<bool>`, two inhabitants.

A language can go further, and this is the only place in NEOS where it can. `true`, `false`, `if`, `else`, `&&`, `||`, `!`, `==`, `!=`, `and`, `or`, `not`, and `bool` are rejected by `lex` with an error naming the axiom and the replacement construct. There is no expression grammar to fall back to.

Two implementation details that are load-bearing:

- **Punctuation is checked against the raw line, before whitespace splitting.** Otherwise `a&&b` arrives as a single word and slips through as an identifier.
- **The refusal fires before parsing and before name resolution.** `a2_refusal_precedes_every_other_error` pins the ordering with a source that is both Boolean-contaminated and name-broken. A programmer who fixes the syntax first and only then learns the construct was forbidden has been told the wrong thing twice.

## Deviation from the target tree

`_spec/target-tree.md` lists `symphony/compiler/` and `symphony/interpreter/` as separate crates. Built as **one crate** with `lexer`, `parser`, `interpreter` modules.

They would share `Token`, `Stmt`, and `LangError`, depend on each other in one direction only, and have no independent consumer. The surgical-split rule asks for splits along *distinct concerns*; compiler-vs-interpreter here is a split along stage names, which is the thing it warns against. Recorded in the math contract rather than done silently.

## What is not built

- **An expression grammar.** No arithmetic, no assignment, no loops, no functions. Adding one would immediately raise "what does `a == b` mean?", and under A2 the answer is *nothing*.
- **Deadlock resolution.** The kernel *detects* circular waits; nothing in the language declares resource acquisition, so there is nothing here to resolve. That boundary is deliberate and unchanged.
- **A module or import system.** A program is one source string.
- **Scale modulation.** PRD §3 lists it alongside interference and phase shift as a geometric gate. `ξ(r)` exists in the kernel and `Task::with_scale` accepts it, but no syntax exposes it — a task's scale is always `1`. **This is the one PRD §3 clause with law behind it that has no surface syntax**, and it is recorded rather than invented, since the PRD does not say what a source-level scale annotation would mean.

## Human check

Read `boolean_constructs_are_refused_at_lex_time` and the `opposes`-as-`else` sabotage entry above.

The first is the only place in NEOS where an axiom is enforced by refusing to tokenise — the strongest form available, because a convention survives only until someone is in a hurry.

The second is the more useful read. A sabotage that breaks nothing is usually a weak test; this time it was a **false claim** in a passing test, and the correction made the record state a weaker true position instead of a stronger false one.

---

# Slice 2 — Phase Shift and Scale Modulation

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 311 passed; 0 failed
cargo test  -p symphony-lang → 44 passed
```

Workspace total: **282 → 311**. `symphony_lang` 27 → 44, `symphony_kernel` 26 → 38.

## What slice 1 got wrong

Slice 1 read PRD §3 as *"a language with no Boolean operators"* and built the interference gate. The section actually names **three** gates:

> *"constructive/destructive interference, **phase shifts**, and **scale modulation** as logic gates."*

I built one of three and reported the record complete. The other two were named in the PRD, had no operational definition anywhere in the corpus, and I recorded only the third as missing — as "the one PRD §3 clause with law behind it and no surface syntax". That framing was wrong twice: phase shift was missing too, and both were derivable.

**Distillation alone was the error.** Both gaps close by *composing* law that already exists, which is what [`_mkb/gates.md`](../../../../_mkb/gates.md) now does — recorded as a synthesis, the same way `timecrystal.md` was.

## Gate 2 — the derivation was already sitting in the constants file

A2 fixes the orientations at `±π/2`. Their separation is

```
(+π/2) − (−π/2) = π
```

which is **exactly** `thresholds.teardown_phase_shift`, already stored, already used by `ftg` for session teardown.

So the teardown shift is not merely compatible with A2's set — it is the map between its two elements, and the set is closed under it. That closure is what makes it a *gate* rather than an operation that can leave the logic. Verified bit-exactly: the shift is `π` to the last bit, the map is an involution, and a phase superposed with its inversion gives **exactly `0.0`** — the `f_total = 0` identity, which is why FTG teardown needs no acknowledgement.

Two things follow that are worth stating because both are easy to get wrong:

- **Gate 2 is total.** No domain check, no error case. An implementation that can fail here has left the axiom.
- **It is not `-φ`.** For this set the two coincide numerically. That is an accident of the orientations being symmetric about zero; the law is the `π` shift, and `-φ` is derived from nothing.

## Gate 3 — the band is derived, not chosen

The hard part was **what shape a scale gate can have**. A2 forbids comparison — "alignment is measured, not compared" — so `when A above B` is out. It would be a Boolean relational operator wearing a geometric name, which is exactly the failure the axiom exists to prevent.

The law supplies a physical criterion instead. `ξ(r)` multiplies nominal frequency at observation scale, so each oscillator has an effective `ν·ξ(r)`. Their relative phase drift over one period of the pair's mean effective frequency is `2π·Δν/ν̄`. Holding that inside the standing-wave stability variance of `±π/4`:

```
2π·|Δν|/ν̄ ≤ π/4   ⇒   |Δν|/ν̄ ≤ 1/8
```

**The band is `1/8` — it is `(π/4)/(2π)`, and `π/4` was already a stored constant.** `build.rs` now asserts the relationship, so editing one threshold without the other stops the build rather than shipping two disagreeing criteria.

The derivation produced a **relative** ratio on its own. That is worth noting given this workspace's history: the absolute-vs-relative trap has cost four debugging sessions, and here there was never an absolute threshold available to get wrong.

Measured boundaries against `R = 1` at 440 Hz: **`r ≈ 1.1892236`** above, **`r ≈ 0.8241412`** below. **Not symmetric** — `+18.92%` against `−17.59%` — because `ξ` is nonlinear. Anyone rewriting this as a symmetric percentage tolerance has replaced the gate with a different function, so the asymmetry is asserted rather than left to be rediscovered.

## The gates are three, not one gate with three spellings

Asserted rather than assumed, because "we added two more branch keywords" would be a fair suspicion:

| Gate | Reads | Ignores |
|---|---|---|
| interference | phase | `ν`, `r` |
| inversion | phase → phase | `ν`, `r` |
| resonance | `ν`, `r` | phase |

The witness: `A(+, 440 Hz, r=1)` and `B(−, 440 Hz, r=1)` **interfere destructively and resonate**. Two gates, same pair, opposite answers. Neither is expressible through the other, and `each_gate_ignores_the_other_gates_inputs` pins the disjointness in both directions.

## A defect in shipped code, found by deriving the gate

Gate 3 evaluates `ξ` at arbitrary user-declared scales, which meant looking at `ξ`'s domain properly for the first time. The implementation was:

```rust
r.sinh() / (r * 1.0_f64.sinh()) * (1.0 - r).exp()
```

`sinh(r)` overflows `f64` at `r ≈ 710.5` — **before** `exp(1-r)` can rescue the product. Measured:

```
xi(710.0) = 1.6289e-3    fine
xi(710.5) = +inf         returned as Ok
xi(1000)  = NaN          returned as Ok
```

`ξ`'s boundedness is **law**: *"bounded above by `e/sinh(1)` — the correction can never diverge, which is what makes it safe in a clock path."* The implementation violated the invariant it was written to satisfy, and returned the violation as a success value. Since `Task::energy_joules` only catches `Err`, an infinite `ξ` propagates straight into the load field, the mean, and the spread.

**The fix is algebraic, not a clamp.** Since `e^r · e^(1−r) = e` identically:

```
ξ(r) = (e − e^(1−2r)) / (2r·sinh 1)
```

which cannot overflow — but loses precision as `r → 0`, where it differences two nearly-equal numbers. So each branch is used where it is exact, split at the reference scale `R`:

- `r ≤ 1` — `sinh(r) ≤ sinh(1)`, overflow impossible
- `r > 1` — `e^(1−2r) ≤ e⁻¹`, cancellation impossible

The split point is `R` itself, not a tuned threshold, and both branches give exactly `1.0` there. Verified: the two agree to **2.5 ulp** across `[1e-8, 700]`; the result is finite, positive and under the supremum for every input **up to and including `f64::INFINITY`**; monotonicity is exact across `[1e-6, 1e6]`.

### Why the existing test did not catch it

`xi_is_bounded_everywhere` sweeps `r ∈ [0, 30]`. The naive form is flawless there.

The test was **too narrow, not wrong** — and that is the more dangerous failure, because it passes and looks like coverage. A law that says *bounded* and a test that checks part of the domain is how an invariant gets violated in shipped code. The range is now the range the law claims.

This is a different failure mode from the ones this workspace has catalogued. Not a wrong tolerance and not a vacuous assertion, but **a correct assertion over an unrepresentative domain**.

## Doctrine checks — four performed

| Sabotage | Failures |
|---|---|
| revert `ξ` to the naive single expression | **5** (3 kernel, 2 lang) |
| gate 3 ignores observation scale | **6** (3 kernel, 3 lang) |
| widen the band from `1/8` to `1/4` | **4** (2 kernel, 2 lang) |
| `invert` becomes a no-op | **7** (2 kernel, 5 lang) |

All four bite, each failing the group it was aimed at.

**Not attempted: `invert` as `-φ`.** Numerically identical on A2's set, so it is not a mutation — the same non-sabotage as slice 1's `opposes`-as-`else`. Recorded rather than run; the lesson was paid for once already.

## A boundary test that was off by a rounding

`identical_frequencies_detune_across_scale` first asserted resonance at `r = 0.82414`, truncated from the bisected boundary `0.8241412282`. That truncation lands on the **detuned** side, and the test failed on first run.

Fixed by straddling each boundary from both sides with exact values rather than asserting near it. A tolerance around the boundary would have absorbed the error and left the gate's edge untested — which is the only part of a threshold gate worth testing.

## Design note: two shapes, not three

The gates do not share a signature, and forcing one would have been wrong. Inversion **transforms** rather than **tests**, so it is a statement (`invert A`), not a `when` relation. Making it a relation would have required inventing a question it answers; the `π` shift has no truth value, it has a result.

The other two both return `Interference`, so they share the two-polarity pattern: `aligns`/`opposes`, `resonates`/`detunes`.

## Still not built

- **A gate combinator.** No `resonates AND aligns`. It would need a truth table, which is what A2 removes. Nesting a branch inside another already composes gates.
- **A scale comparison.** Forbidden by §7.4 of the contract, for the reason above.
- **Expressions, assignment, loops, functions, modules.** Unchanged from slice 1.
- **Deadlock resolution.** The kernel detects; nothing in the language declares resource acquisition. Boundary unchanged.

## Human check

Read `identical_frequencies_detune_across_scales` and the `ξ` defect above.

The first is what makes gate 3 *scale* modulation rather than a frequency comparison — same nominal frequency, different observation scale, out of resonance.

The second is the more important read: a **stated law invariant was violated in shipped code** because the test that checked it swept a domain where the bug does not appear. It was found by asking what happens at the edge of a function's domain — a question the new gate forced, and one no previous subsystem had needed to ask of `ξ`.

---

# Slice 3 — The Instruction-Executing State Machine (`vm`)

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 435 passed; 0 failed
cargo test  -p symphony-lang → 56 passed
```

Workspace total: **423 → 435**. `symphony_lang` 44 → 56.

## The proposal this closes, and the one it declined

A conversation-borne architecture doc proposed four phases toward "true virtualization": (1) a bytecode/AST interpreter with register/stack state, arithmetic, and branching; (2) routing instruction load/store through `substrate::MemoryPool` via curved addresses; (3) wiring memory faults into `Hypervisor::allocate_trapped`-style dynamic fault routing; (4) `KernelSpace`/`GuestSpace` privilege enforcement at instruction decode.

Checked against `_mkb/axioms.md` before writing anything, per this workspace's own precedence rule (axioms outrank spec outrank code). **A2 binds this record by name**: *"No `bool` in Symphony-layer logic; no `if (x == true)`."* Phase 1 as specified — register/stack arithmetic and Boolean-style branching — is exactly what A2 forbids, at exactly the subsystem A2 names, and this record's own prior slices had already declined it once (§"Not built" above). Investigated rather than refused outright: `substrate`'s memory bus (Phase 2) turned out to already be fully built and curved-address-only by construction; `Hypervisor::allocate_trapped` (Phase 3) already traps `allocate` faults, deliberately not `read`/`write`, for the stated reason that nothing called them in a context where trapping meant anything — a reason that stops applying the moment a real instruction loop exists. Phase 4 has no law behind it anywhere in `_mkb/` and was left alone.

What got built is [`_mkb/instruction_set.md`](../../../../_mkb/instruction_set.md)'s **full** ISA — comprehensive, not a minimal stub, per explicit instruction — composed entirely from primitives that were already real and already tested: the three gates, A1's `fork`, and (newly reachable from the language) `substrate::MemoryPool` and `symphony_kernel::resources::ResourceTracker`.

## `EVAL`/`RESONATE`/`SHIFT` — one new mechanism, not three

`vm::compile` flattens the parsed `Stmt` tree into a flat, program-counter-addressed `Instruction` sequence — the actual definition of an instruction-executing state machine, as distinct from a tree-walker. A `Branch`'s body is flattened in place directly behind its `Eval` instruction, with a `skip` count recording exactly how far to jump when the gate does not fire — control flow over a flat address space instead of recursion, semantics otherwise unchanged: `EVAL` and `RESONATE` are the *same* `Instruction::Eval`, discriminated by the same `Alignment` the tree-walker's `Stmt::Branch` already carries, not two new mechanisms wearing the proposal's names.

**Verified as behaviourally identical to the tree-walker it replaces**, not merely "compiles and runs." `bytecode_dispatch_matches_the_tree_walker_exactly` runs the same real programs (including nested branches) through both engines and asserts the `declared`/`emitted`/`forks`/`inversions`/`branches` outputs are exactly equal. A second, more surgical test (`nested_branches_compile_with_correct_skip_targets`) pins the compiled `skip` values directly, so a future failure localises to `compile` rather than requiring a diff against the tree-walker.

## `STORE`/`LOAD` — a task's own physical state, not general memory

Not general byte-addressable scratch space. There is no instruction that writes an arbitrary program-chosen byte string — only a typed save/restore of the three quantities `TASK` already declares (frequency, phase, scale), because those are the only things in this language's type system with anywhere meaningful to be stored; A2 and the "no expressions" decision mean there is no integer, string, or boolean type to spill. Encoding is three `f64` little-endian (24 bytes) — a pure byte round-trip with no arithmetic on the stored value, so `f64::from_le_bytes(x.to_le_bytes()) == x` makes reconstruction exact by construction, not a claim needing separate numerical verification.

Addressing is `MemoryPool::address_at(n)` — real `LatticeAddress`, never a flat offset, A3 honoured by construction the same way `substrate` already guarantees it. `⊗`-fold `AddressPath` addressing was considered and deliberately not built for this slice — it is a second grammar (signed step sequences) the ask did not need; recorded as a scope boundary in `_mkb/instruction_set.md`, available for a later slice.

Memory nothing ever `store`d into is zero-initialised (`substrate::memory::MemoryPool::new`), which decodes to `frequency = 0.0` — already refused by `RuntimeTask::at_scale`'s existing `UnphysicalFrequency` guard, reused rather than re-derived. `loading_a_never_stored_cell_is_refused_not_fabricated` pins that garbage memory is refused as garbage, not silently accepted as a valid task.

## `ACQUIRE`/`RELEASE` — closing a gap this record named twice

Both prior slices' "Not built" sections said the same thing: *"nothing in the language declares resource acquisition."* `Instruction::Acquire`/`Instruction::Release` are real calls into `symphony_kernel::resources::ResourceTracker::acquire`/`release` — the same API `neos/src/main.rs`'s kernel-level deadlock demonstration already exercises, now reachable from a program rather than only from hand-written Rust.

**A real, stated limit, not hidden**: `Vm::run_batch` runs one program to completion at a time. There is no scheduler here able to suspend a program that would block on `acquire` and resume it once another program releases. `Acquired::Blocked` is therefore a trap (`VmFault::Blocked`), not a hang and not a silent "granted anyway" — `a_blocked_acquire_traps_rather_than_hanging_or_silently_granting` verifies this directly, and the sabotage table below confirms it was actually load-bearing, not decorative.

## Fault isolation — the substantive part of Phase 3, without Phase 4

`Vm::run_batch` runs several compiled programs against one shared `MemoryPool`/`ResourceTracker`/`WaitForGraph` — the real, shared kernel-lattice state. A fault in one program's instruction stream stops only that program's own dispatch loop (`VmFault`, a real `Result` value at every fault site, never a Rust panic); the rest of the batch runs on against the same, undamaged shared state. `a_fault_isolates_only_the_faulting_program_in_a_batch` proves it directly: three programs share a pool, the middle one traps on a `load` from a never-stored cell, and the third still loads the first program's real stored state back successfully. Wired into `neos/src/main.rs` too, so the isolation claim is demonstrated live in the boot-to-report pass, not only in the test suite.

This closes the proposal's Phase 3 claim — *"a bad load instruction... traps out the running symphony-lang thread... leaves the rest of the kernel lattice running cleanly"* — entirely at the interpreter/VM level, without needing Phase 4's `KernelSpace`/`GuestSpace` privilege domains, which remain unbuilt because no law names them.

## Doctrine checks — three performed

| Sabotage | Result |
|---|---|
| `Eval`'s not-taken jump off by one (`skip` instead of `1 + skip`) | **1 of 56 failed** — `bytecode_dispatch_matches_the_tree_walker_exactly`: the not-taken branch's first instruction ran anyway. Notably, `nested_branches_compile_with_correct_skip_targets` stayed green — it pins `compile`'s output, not the dispatcher's use of it, a useful confirmation the two tests check different layers rather than the same thing twice. |
| `run_batch` stops the whole batch at the first trap | **1 of 56 failed** — `a_fault_isolates_only_the_faulting_program_in_a_batch`: only 2 of 3 programs ran instead of 3 |
| `Acquired::Blocked` treated as `Granted` | **1 of 56 failed** — `a_blocked_acquire_traps_rather_than_hanging_or_silently_granting`: no trap where one was required |

All three reverted after confirming; full suite re-confirmed at 56/56, 435/435 workspace-wide.

## `RuntimeTask::set_phase` — a `pub(crate)` seam, not a widened public API

`vm.rs` needed to mutate a `RuntimeTask`'s phase for `Instruction::Invert`, the same way `interpreter::Runner` already does for `Stmt::Invert` — but as a private field in a different module, it couldn't. Added `RuntimeTask::set_phase`, `pub(crate)` rather than `pub`: both execution engines (same crate) can flip a phase, but nothing outside the crate can construct an arbitrary orientation directly — a host still only reaches phase mutation through `invert NAME`/`Instruction::Invert`.

## Two execution engines, by capability, not by redundancy

`interpreter::execute`/`run` are unchanged and still the right tool for a program that only declares, branches, forks, and emits — no side inputs, no memory pool to thread through every call site. The five new `Stmt` variants parse into the same grammar but are refused by the tree-walker (`LangError::RequiresVm`) rather than silently ignored or given fake meaning: it has no `MemoryPool` or `ResourceTracker` to act against, and giving `execute` one would make every existing call site's simple signature a lie.

## Human check

Read `bytecode_dispatch_matches_the_tree_walker_exactly` first — it is the proof the new engine did not quietly become a different language while wearing the old grammar. Then read `a_fault_isolates_only_the_faulting_program_in_a_batch` and the `neos/src/main.rs` output it corresponds to (`symphony-lang: the instruction-executing state machine` in `cargo run`'s report) side by side — the test and the live demo make the same claim two ways, the way this workspace's cross-cutting slices generally do.

---

# Addendum — `⊗`-fold path addressing for `store`/`load` (closing Phase 2 in full)

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 440 passed; 0 failed
cargo test  -p symphony-lang → 61 passed
```

Workspace total: **435 → 440**. `symphony_lang` 56 → 61.

Slice 3 above recorded cell-ordinal addressing as a deliberate scope boundary and deferred real `⊗`-fold path addressing to a later pass. Requested explicitly as "Phase 2" of the original architecture proposal — already substantively true (`substrate::MemoryPool` was always `LatticeAddress`-only, never a flat offset, and `vm`'s cell-ordinal `store`/`load` already routed through it) — this closes the one piece that slice genuinely deferred: directory-style paths, the addressing form `lattice::addressing`/`AddressPath`/`MemoryPool::resolve_path` already define and test elsewhere.

## What's new, and what isn't

`store`/`load` now accept `path START S1 S2 ...` alongside the existing `cell N`. `crate::parser::Address` is the new enum both forms share (`Cell(usize)` / `Path{start: f64, steps: Vec<f64>}`), threaded through `Stmt::Store`/`Load`, `Instruction::Store`/`Load`, unchanged. `Vm::resolve` is the single new method — the one place both forms converge to a real `LatticeAddress`, so `Instruction::Store` and `Instruction::Load` share exactly one resolution path rather than each re-implementing the `Cell`/`Path` split. No new physics: `AddressPath::new`/`MemoryPool::resolve_path` are unmodified, already-tested APIs (`neos/tests/substrate.rs` Group 7); this crate's own new code is the parser grammar and the one-method dispatch, nothing more.

`VmFault::CorruptState` changed from carrying a bare `cell: usize` to carrying the full `address: Address` — a path address has no single meaningful "cell number" to report the way an ordinal does, so the fault now names whichever address form actually produced it.

## Grammar: a number list needed no new ambiguity-resolution machinery

`path START S1 S2 ...` takes a variable-length list of steps with no closing delimiter. Safe by construction rather than by a lookahead trick: every statement in this grammar begins with a keyword token, never a bare number, so `Cursor::number_list` can greedily consume `Token::Number` until a non-number token appears and never risk swallowing the next statement's own leading token.

## A borrowed-verification discipline: don't re-derive what's already proven

Rather than build a fresh scratch harness to discover safe path/pool combinations for the new tests, the exact pool sizes and `AddressPath` values already proven in `substrate.rs`'s Group 7 (`identity_path_resolves_to_the_pool_start`, `a_resolvable_path_can_still_be_unmapped_in_a_small_pool`, `dissonant_path_is_refused_not_panicked`, `a_large_enough_pool_maps_an_ordinary_resolved_cell`) were reused directly for the new `vm` tests. This is not a shortcut around verification — it *is* the verification, already paid for and already reviewed, reused rather than re-derived and re-risked.

One case still needed a fresh check, because it wasn't covered by the borrowed cases: whether a path with *any* extra step resolves to a different cell than its bare start, which `a_paths_steps_resolve_to_a_different_real_cell_than_its_bare_start` needed to assert. First attempt used `path 1.0 1.0` against a 200-cell pool and got `outcome.trap == None` instead of the expected `CorruptState` — the assumption was wrong, not the code: a disposable scratch harness (`neos/symphony/lang/examples/scratch_path_addr2.rs`, deleted after use) confirmed `path 1.0 1.0` and bare `path 1.0` resolve to the *same* cell in that pool, while `path 1.0 2.0 1.5` (already used successfully in an earlier scratch check) resolves to a different one. Fixed by using the confirmed-different pair; the false assumption is recorded in the test's own doc comment so it isn't silently re-made later.

## Doctrine check — one more performed

| Sabotage | Result |
|---|---|
| `Address::Path`'s steps dropped in `Vm::resolve` (`AddressPath::new(*start, &[])` regardless of `steps`) | **2 of 61 failed** — `a_path_that_leaves_otimes_domain_traps_as_a_memory_fault` (the six-deep-unit-step path no longer left `⊗`'s domain once the steps were dropped) and `a_paths_steps_resolve_to_a_different_real_cell_than_its_bare_start` (both addresses collapsed to the same cell) |

Reverted after confirming; full suite re-confirmed at 61/61, 440/440 workspace-wide.

## Human check

Read `a_paths_steps_resolve_to_a_different_real_cell_than_its_bare_start`'s doc comment alongside the sabotage row above — the comment records a wrong assumption caught before the test shipped, and the sabotage is the independent confirmation that the property the (corrected) test asserts is actually load-bearing in the code, not just true of the two specific values chosen.

---

# Addendum — dynamic fault routing (`Vm::run_program_trapped`)

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 444 passed; 0 failed
cargo test  -p symphony-lang → 65 passed
```

Workspace total: **440 → 444**. `symphony_lang` 61 → 65.

Closes "Phase 3" of the original architecture proposal — dynamic fault routing wired to something in the shape of `Hypervisor::allocate_trapped`. The isolation slice above already delivered the proposal's stated *outcome* ("traps out the running thread, leaves the rest of the kernel lattice running cleanly"); what was still missing was the mechanism itself — a real handler, called on the fault, able to take a corrective action and ask for a retry, rather than a fault being unconditionally fatal to the program the instant it occurs.

## The design question: retry *what*, exactly

`allocate_trapped` retries one call — `self.pool.allocate(bytes)` — a single, idempotent operation with no state accumulated before it that a retry could disturb. A `symphony-lang` program is a sequence of instructions with real accumulated state (`declared` tasks, `emitted` tasks, ...), so "retry" needed a real answer to "retry from where?" before any code was written.

Two options were weighed. **Restart the whole program from `pc = 0`**, resetting all accumulated state — rejected: instructions before the fault are not generally safe to re-run (a `task` re-declared hits `DuplicateTask`; a `fork`/`emit` before the fault would double-emit). **Retry only the faulting instruction, at the same `pc`, leaving everything already accumulated untouched** — the one implemented. This is `allocate_trapped`'s own idea applied at finer grain: the "operation" being retried is one instruction, not one program, and it is the direct reason `run_program_trapped` needed no new state-management logic at all — `pc` simply doesn't advance on a retried fault, so the next loop iteration re-executes the identical instruction against whatever the handler just corrected.

Proven correct by construction rather than merely hoped: `a_corrupt_state_fault_can_be_recovered_by_seeding_the_cell_and_retrying`'s program declares and emits a task *before* the faulting `load`. A restart-based implementation would have hit `DuplicateTask` on that declaration the second time through, or doubled the emitted count; same-instruction retry does neither, and the test pins both (`emitted.len() == 2`, not 1 or 3).

## Implementation: one macro, five call sites, zero duplicated loops

Rather than duplicate the ~150-line dispatch loop for a second "trapped" variant (real risk to the 60 tests already passing against the first), `run_program` and `run_program_trapped` share exactly one implementation: `run_program` is now defined as `run_program_trapped(task_id, instructions, 0, |_, _| TrapAction::Propagate)`. Since a handler that always propagates and a retry budget of zero can never take the `Retry` branch, this is provably identical to the old body for every existing call — confirmed directly: all 61 pre-existing tests, none touched, pass unmodified against the new shared implementation.

A local `macro_rules!` (`recover_or_trap!`), not a closure, wraps the five real memory-fault sites (`Store`'s resolve/write, `Load`'s resolve/read/decode) — a closure could not `continue`/`break` the enclosing dispatch loop the way this needed to; a macro expanding inline can. `UndeclaredTask`/`DuplicateTask` — `Store`/`Load` naming a task the program never declared, or declaring one twice — deliberately do **not** go through the macro: they are the program's own logic errors, and `language_level_faults_never_reach_the_handler` pins that the handler is never even called for them, mirroring `allocate_trapped`'s own choice to scope trapping to `allocate` specifically rather than every possible `SubstrateError`.

## Doctrine checks — two performed, both mirroring `allocate_trapped`'s own

| Sabotage | Result |
|---|---|
| Retry bound off by one (`retries <= max_retries` instead of `<`) | **1 of 65 failed** — `retries_are_bounded_a_handler_that_never_helps_cannot_hang`: 5 handler calls instead of the expected 4 for `max_retries: 3` |
| `TrapAction::Propagate` ignored (retried unconditionally while budget remains) | **1 of 65 failed** — `propagate_still_traps_immediately_even_with_retries_available`: 11 handler calls instead of 1 |

Both reverted after confirming; full suite re-confirmed at 65/65, 444/444 workspace-wide. Worth noting: `substrate`'s own implementation log records catching these exact same two mutation shapes on `allocate_trapped` itself — the same guard, independently re-verified at a second call site rather than assumed to still hold because the first one did.

## Wired into the demo

`neos/src/main.rs` gained a `symphony-lang: dynamic fault routing` section: a program's `load` faults on a cell nothing ever wrote to, a real handler seeds that exact cell with encoded task state and asks for a retry, and the retried `load` finds it — printed live, not only asserted in a test.

## Human check

Read the "retry *what*" design question above, then `a_corrupt_state_fault_can_be_recovered_by_seeding_the_cell_and_retrying`'s own doc comment — together they're the argument for same-instruction retry over whole-program restart, and the test is the concrete proof the chosen semantics actually hold rather than merely sounding right.

---

# Addendum — privilege domains (`vm::Domain`, `Vm::reserve_cells`)

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 448 passed; 0 failed
cargo test  -p symphony-lang → 69 passed
```

Workspace total: **444 → 448**. `symphony_lang` 65 → 69.

Closes "Phase 4" of the original architecture proposal, and it is unlike every other piece of that proposal built so far. Phases 1-3 each turned out to be composable from something already real — a gate, `substrate`'s memory bus, `allocate_trapped`'s fault-dispatch shape. Privilege domains are not: checked again, a third time, before writing anything, `_mkb/` still names nothing about privilege, isolation levels, or guest/kernel separation. There was nothing to compose.

## Why this got built anyway, and what that does and doesn't mean

This record's own text had already called Phase 4 "the one part of the virtualization proposal genuinely left open — building it now would be invention, not composition," across three separate summaries. The request to proceed came a third time regardless, with that fact restated in front of it each time. Building it at that point is not a reversal of the discipline — it's the discipline applied one level up: this workspace already has real precedent for shipping a design decision that has no law behind it, **as long as it is labelled as a decision and not smuggled in as physics** — the demo binary's deadlock victim policy ("stated plainly as a choice rather than a derived fact"), `crystallisation`'s RGB→grayscale conversion, `tetryen_recurrence.md`'s coupling structure ("an algorithmic choice, not a law citation"). Privilege domains join that list. No new `_mkb/` *law* file was written for it — `_mkb/instruction_set.md` gained a section, explicitly headed "built, but as a stated convention, not law," the same way the other examples are recorded in place rather than pretending to a derivation that doesn't exist.

## Staying inside A2 anyway

The proposal's own language was `KernelSpace`/`GuestSpace` — a binary distinction, which is exactly the shape A2 exists to police if implemented carelessly (`if domain == KernelSpace { ... } else { ... }` is `if`/`else` wearing a different name). `vm::Domain` is `#[derive(PartialEq, Eq)] enum Domain { Kernel, Guest }` — two-valued in the same spirit `Phase`/`Alignment`/`Acquired` already are: a real, named domain distinction with exactly two inhabitants, not a `bool` renamed to dodge the lexer's forbidden-word list. The check itself (`domain == Domain::Guest && self.reserved.contains(&addr.cell())`) is ordinary Rust control flow the way every other fault check in this file already is — A2 binds *symphony-lang source*, not the Rust implementing its runtime, the same distinction that already lets `Runner`/`Vm` be written with `match`/`if` throughout.

## What "reserved" actually protects, and why the check sits where it does

`Vm::reserve_cells` takes any `impl IntoIterator<Item = CellId>` — a single cell or a whole Tetryen patch (a patch, geometrically, *is* the set of cells it occupies, so no separate "patch" concept was needed). The check runs against the address `Store`/`Load` **resolves to**, not the `Address` syntax a program wrote — deliberately: the same way a real MMU's protection fault fires against the translated physical address, not whatever virtual address a program named, this stays correct regardless of which addressing mode (`cell N` or `path ...`) a guest used to reach a reserved cell. `reservation_applies_to_path_addresses_too` pins this directly — the identical reserved cell, reached via a `⊗`-fold path instead of an ordinal, is refused all the same.

**Never offered to a `run_program_trapped` handler**, unlike every other memory fault kind. This is a real, deliberate asymmetry, not an oversight: a handler that could "fix" a privilege violation and ask for a retry would let any program with a large enough `max_retries` budget talk its way past the boundary, which makes the boundary decorative. `a_privilege_violation_is_never_offered_to_the_handler_even_with_retries` uses a handler that always returns `Retry` with a budget of 100 and confirms it is called zero times.

## Doctrine checks — two performed, one in each direction

| Sabotage | Result |
|---|---|
| Privilege check disabled entirely (`if false && domain == Domain::Guest && ...`) | **4 of 69 failed** — every reserved-cell test, both `Address::Cell` and `Address::Path` forms |
| Domain condition inverted (`Domain::Kernel` checked instead of `Domain::Guest`) | **4 of 69 failed** — the same four, for the opposite reason: `Domain::Kernel` (meant to be unrestricted) was wrongly blocked, and `Domain::Guest` (meant to be restricted) was wrongly let through |

Both reverted after confirming; full suite re-confirmed at 69/69, 448/448 workspace-wide. Testing both directions independently, not just "the check exists," is what confirms the boundary actually discriminates by domain rather than blocking (or passing) everything regardless of who's asking.

## Wired into the demo

`neos/src/main.rs` gained a `symphony-lang: privilege domains` section reserving the exact two cells the report's own first section allocated 8192 real bytes into — genuinely system-critical, not a cell picked only for this demo — and shows a `Domain::Guest` program refused from touching them.

## Human check

Read "What 'reserved' actually protects, and why the check sits where it does" alongside `reservation_applies_to_path_addresses_too` — the design claim (checked at the resolved address, not the source syntax) and its test are the same claim stated two ways. Then read the two-sabotage table above: testing only "disabled" would have left an inverted, backwards check equally green, since both mutations remove the *distinguishing* behaviour a privilege boundary exists for.
