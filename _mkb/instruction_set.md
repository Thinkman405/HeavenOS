---
type: subsystem-law
layer: law
status: canonical
closes: "A conversation-borne proposal for symphony-lang virtualization (register/stack state, arithmetic, branching) — scoped instead to a full instruction set that stays inside A2, plus real memory and resource wiring"
synthesis_of: ["gates.md", "axioms.md A1", "axioms.md A2", "axioms.md A3", "substrate memory.rs", "symphony-kernel resources.rs"]
---

# The Symphony Instruction Set — a full ISA, not an AST walk

A proposal arrived asking for "a core bytecode or AST interpreter supporting register/stack state, arithmetic, branching" in `symphony-lang`, as the base of a four-phase virtualization plan. Checked against law before writing anything: **A2 binds `symphony-lang` by name** — *"No `bool` in Symphony-layer logic; no `if (x == true)`"* — and `symphony-lang`'s own completed record already rejected exactly this, on purpose: *"Adding an expression grammar would immediately raise 'what does `a == b` mean?', and under A2 the answer is nothing."* Register/stack arithmetic is that expression grammar under a different name. It was not built.

What follows instead is a **full** instruction set — comprehensive, not a minimal stub — built entirely from constructs that already have real, tested meaning: the three geometric gates ([gates.md](gates.md)), A1 bifurcation, and two subsystems' already-real APIs (`substrate::memory`, `symphony_kernel::resources`). Nothing here is a new formula. Every instruction is a composition, verified by the fact that the primitive it calls is already law-tested elsewhere — the same discipline [gates.md](gates.md) and [tetryen_recurrence.md](tetryen_recurrence.md) both establish.

## What composes it

| Ingredient | Home | Status |
|---|---|---|
| The three geometric gates | [gates.md](gates.md) | already law, already implemented |
| A1 — bifurcation (`fork`) | [axioms.md](axioms.md#a1--multiplicative-identity-override) | already law, already implemented |
| A3 — curved addressing, `LatticeAddress` | [axioms.md](axioms.md#a3--spatial-addressing-override) | already law, already implemented (`substrate::memory`) |
| Real resource acquisition/release | — | already implemented (`symphony_kernel::resources::ResourceTracker`), previously **unreachable from the language** — `symphony-lang`'s own record named this gap: *"nothing in the language declares resource acquisition, so there is nothing here to resolve"* |

## The instructions

| Mnemonic | Operands | Semantics | Composes |
|---|---|---|---|
| `TASK` | name, frequency, phase, scale | declare an oscillator | existing, unchanged |
| `SHIFT` | name | gate 2 — exact `π` phase flip | existing, unchanged (`invert`) |
| `EVAL` | left, `aligns`\|`opposes`, right, body | gate 1 — interference test, conditional dispatch | existing, unchanged (`when ... aligns/opposes`) |
| `RESONATE` | left, `resonates`\|`detunes`, right, body | gate 3 — scale-corrected standing-wave test, conditional dispatch | existing, unchanged (`when ... resonates/detunes`) |
| `FORK` | name | A1 bifurcation | existing, unchanged |
| `EMIT` | name | hand a task to the scheduler | existing, unchanged |
| **`STORE`** | name, cell | write a declared task's physical state (frequency, phase, scale) into curved memory | **new** — `substrate::MemoryPool::write` at `MemoryPool::address_at(cell)` |
| **`LOAD`** | name, cell | declare a task by reading its physical state back from curved memory | **new** — `substrate::MemoryPool::read`, inverse of `STORE` |
| **`ACQUIRE`** | resource id | acquire a named resource for the running program | **new** — `symphony_kernel::resources::ResourceTracker::acquire` |
| **`RELEASE`** | resource id | release a held resource | **new** — `ResourceTracker::release` |
| **`HALT`** | — | end this program's execution | **new**, trivial |

`EVAL` and `RESONATE` are one instruction in the implementation (`vm::Instruction::Eval`), discriminated by the same `Alignment` enum the existing tree-walk interpreter already uses — not a second gate mechanism, the same one addressed two ways, exactly mirroring `Stmt::Branch` today.

## What `STORE`/`LOAD` deliberately are not

Not general byte-addressable scratch memory, and not registers in the classical sense. There is no instruction that writes an arbitrary byte string chosen by the program — only a **typed** save/restore of a task's own physical state (frequency, phase, scale), the same three quantities `TASK` already declares. This is the only thing in the language's existing type system with anywhere meaningful to be stored: there is no integer, string, or boolean type to spill, because A2 and the "no expressions" decision mean none exist. `STORE`/`LOAD` therefore cannot become a back door into general computation — a program can move a task's state into curved memory and back, and nothing else.

Encoding: three `f64` in IEEE-754 little-endian (frequency, `Phase::radians()`, scale — 24 bytes), a straight byte round-trip with **no arithmetic performed on the stored value**, so reconstruction is bit-exact by construction (`f64::from_le_bytes(x.to_le_bytes()) == x`, always) — not a claim requiring separate numerical verification, a fact about IEEE-754 round-tripping. `LOAD` from a cell nothing ever `STORE`d (zero-initialized memory, per `substrate::memory::MemoryPool::new`) decodes to `frequency = 0.0`, which `RuntimeTask::at_scale` already refuses (`UnphysicalFrequency`) — garbage memory is refused as garbage, not silently accepted as a valid task.

## Addressing: two real forms, both `LatticeAddress`, never a flat offset

`STORE`/`LOAD` name a memory location one of two ways:

- **`cell N`** — an ordinal into the pool's own ring order (`MemoryPool::address_at(n)`).
- **`path START S1 S2 ...`** — a real `⊗`-fold directory-style `lattice::AddressPath`, resolved through `substrate::MemoryPool::resolve_path`, the same addressing `lattice::addressing` already defines and `substrate` already tests (`neos/tests/substrate.rs` Group 7).

Both return a real `LatticeAddress`; A3 is honoured by construction either way, the same way it already is throughout `substrate`. Path addressing was initially deferred (recorded as a scope boundary in this file's first version) and closed in a follow-up slice once Phase 1's core dispatch loop was proven — `vm::Vm::resolve` is the one place both forms converge, so `STORE` and `LOAD` share exactly one resolution path rather than each re-implementing the split. A path's `start`/steps are real `LatticeScalar` values, not indices — no whole-number discipline applies, and `⊗`'s own domain (checked at resolution, via `SubstrateError::AddressUnresolvable`) is the only limit, the identical domain guard `lattice` addressing already enforces everywhere else.

## Dynamic fault routing — the direct counterpart to `allocate_trapped`

`vm::Vm::run_program_trapped` gives `store`/`load`'s memory faults the same real, corrective-action fault dispatch `substrate::Hypervisor::allocate_trapped` already gives `allocate`'s: a caller-supplied handler is called on **every** memory fault, unconditionally, with `&mut MemoryPool` — the same pool the failing operation is against — so a real correction is actually possible (concretely: `pool.write`ing valid state into a cell a `load` found corrupt). A `TrapAction::Retry` re-attempts **the same instruction**, at the same program counter, with everything the program has already accumulated left untouched — not a restart of the program from the top, which would re-run every prior `task`/`fork`/`emit` a second time and risk `DuplicateTask` or double-counted emissions. `max_retries` bounds the total, the identical second guard `allocate_trapped` already carries, so a handler that never actually fixes anything cannot hang the caller.

Scoped identically to `allocate_trapped`'s own scoping discipline: only real memory faults (`VmFault::Memory`, `CellOutOfRange`, `CorruptState`) reach the handler. `UndeclaredTask`/`DuplicateTask` — a program naming a task it never declared, or declaring one twice — are the *program's own* logic errors, not something a handler can fix by acting on the pool, and are never offered to it; `run_program` (unchanged, still the default entry point) is exactly `run_program_trapped` with `max_retries: 0` and a handler that always propagates, so every one of the earlier slice's tests is itself a standing regression guard that this addition changed nothing about the untrapped path.

## Fault isolation, not privilege domains

**This closes the substantive part of the original proposal's Phase 3** ("a bad load instruction... traps out the running symphony-lang thread... leaves the rest of the kernel lattice running cleanly") **without needing Phase 4's privilege domains, which still have no law behind them** — see root `CONTEXT.md`'s scope boundaries, unchanged by this file.

The VM runs a **batch of programs** against one shared `MemoryPool`, `ResourceTracker`, and `WaitForGraph` — the real, shared kernel-lattice state the isolation guarantee is actually about. Each program is its own instruction stream with its own oscillator namespace (`declared` tasks do not leak between programs, matching how `execute()`/`execute_with()` already scope a single program) and its own `symphony_kernel::TaskId` for resource tracking. A fault — a `STORE`/`LOAD` hitting a real `SubstrateError`, an out-of-range cell ordinal, `ACQUIRE` returning `Acquired::Blocked`, `RELEASE` on a resource this program does not hold — **stops that program's own dispatch loop and is recorded against it**; the batch continues with the next program, against the same, undamaged shared state. No Rust panic anywhere in this path: every fault is a `Result`, matching `substrate::Hypervisor::allocate_trapped`'s existing discipline of routing faults through real values rather than unwinding.

**`ACQUIRE` blocking is a stated, real limit, not hidden.** This VM's programs run to completion one at a time within a batch — there is no scheduler here capable of suspending a blocked program and resuming it later. `Acquired::Blocked` is therefore treated as a trap (the program cannot make forward progress) rather than either silently proceeding as if granted, or hanging the whole batch waiting for a release that will never come from within this same sequential pass. A program that needs to wait for a resource genuinely held by another program in the same batch will trap; this is recorded as the honest shape of what a batch of programs, not real concurrent threads, can promise.

## Privilege domains — built, but as a stated convention, not law

Every other instruction in this file composes something real: a gate already derived in `gates.md`, `substrate`'s already-tested memory bus, `allocate_trapped`'s already-proven fault-dispatch shape. Privilege domains have none of that. Checked again before writing a line of code, same as the first time: no axiom, no PRD section, no `_mkb/` file names privilege, isolation levels, or guest/kernel separation anywhere. There is nothing to compose.

Built anyway, once asked a third time with that fact restated each time — as a **deliberately, explicitly labelled engineering convention**, the same footing as the demo binary's deadlock victim policy ("stated plainly as a choice rather than a derived fact") or `crystallisation`'s RGB→grayscale conversion, both real design decisions in this workspace that were never claimed as physics. `vm::Domain` is two-valued (`Kernel`/`Guest`) in the same spirit A2 already keeps `Phase`/`Alignment`/`Acquired` two-valued — a real domain distinction, not a `bool` wearing a different name — and `Vm::reserve_cells` marks specific cells (or a whole Tetryen patch, which is just the set of cells it occupies) off-limits to `Domain::Guest` programs. The check happens against the **resolved** `LatticeAddress`, at the one place `store`/`load` actually touch the pool — the same place a real MMU checks a translated address, not a symbolic one — and is never offered to a `run_program_trapped` handler: a handler able to retry past it would make the boundary meaningless, unlike every other memory fault kind.

What stays genuinely unbuilt, because there's still no law for it either: what privilege *should* protect beyond memory cells (resource acquisition, instruction categories), how many domains beyond two, or anything resembling a derived security model. This section is a mechanism, not a policy — the policy (which cells, for which programs) is left entirely to the host that calls `reserve_cells`.

## What this deliberately does not build

- **General arithmetic, comparisons, or an expression grammar.** A2 forbids the boolean predicate any general comparison (`==`, `<`, `>`) would need. `EVAL`/`RESONATE` remain the only two conditionals, exactly as before.
- **True concurrency.** The batch runner is sequential, one program at a time. `ACQUIRE` blocking traps rather than pretending to schedule — stated above, not hidden.
- **A derived privilege *policy*.** See directly above — what's built is the enforcement mechanism, not a claim about what should be protected or why.

## Binds

- [[gates]] — `EVAL`/`RESONATE`/`SHIFT`, unchanged semantics
- [[symphony-lang]] — `neos/symphony/lang/src/vm.rs`
- [[substrate]] — `MemoryPool::read`/`write`/`address_at`, real curved memory
