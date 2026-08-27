---
type: subsystem
subsystem: symphony-lang
tier: 2
language: custom DSL
stage: 04_implement
status: complete
result: "78 tests passing. Lexer, parser, interpreter. All three of PRD 3's geometric gates. A2 enforced by refusing to tokenise Boolean constructs; the TaskModel seam is closed. A second execution engine (vm) compiles the same grammar into a flat, program-counter-addressed instruction sequence with real store/load through substrate::MemoryPool (both cell-ordinal and real ⊗-fold path addressing) and real acquire/release through symphony_kernel::resources, isolating a runtime fault per program rather than per Rust process. vm::Vm::run_program_trapped adds real dynamic fault routing on top — a handler called on every memory fault with &mut MemoryPool, able to retry the exact faulting instruction, the direct language-level counterpart to substrate::Hypervisor::allocate_trapped. vm::Domain/Vm::reserve_cells add privilege domains as a stated engineering convention (explicitly not law, since none exists) enforced at the resolved address. A third engine, concurrent::run_program/run_batch_concurrent, closes vm's stated 'no scheduler' limit for real: real OS threads sharing a real symphony_kernel::ConcurrentPool/ConcurrentTracker, blocking acquire verified by timing an actual wait. sandbox::Sandbox composes that same real concurrency with a per-tenant CellId->Owner ownership map into a genuine multi-tenant sandbox: several mutually untrusted programs run at once, each provably confined to its own admitted memory."
prd_sections: ["3"]
binds_axioms: ["A1", "A2", "A3"]
split_from: symphony
consumes: [symphony-kernel, substrate, lattice]
slices: ["lexer + parser + interpreter, interference gate", "phase shift and scale modulation gates", "the instruction-executing state machine (vm)", "real concurrency (concurrent)", "a genuine multi-tenant sandbox (sandbox)"]
---

# Symphony-lang — the kernel DSL

One job: a language whose logic gates are geometric — constructive/destructive interference, phase shifts, scale modulation — with no Boolean operators.

## Status: complete

The seventh and last record. Deferred by decision until [[symphony-kernel]] settled the runtime semantics; resumed once it had.

**The deferral paid off.** `symphony_kernel::bifurcation::TaskModel` was written as a guess at the seam's shape, with a doc comment saying nothing depended on it yet. All three of its methods turned out to be exactly what the language needed — `frequency()` for `E = C_H·ν`, `guard_phase()` for `evaluate_branch`, `fork_unit()` for `fork`. None unused, none missing. A language designed ahead of the kernel would have guessed a shape for an imagined runtime.

## All three gates, in two slices

PRD §3 names three geometric replacements for Boolean logic. Slice 1 built one and I reported the record complete — **that was an under-reading of the section.** Slice 2 built the other two, which required deriving them into [`_mkb/gates.md`](../../_mkb/gates.md) first.

| Gate | PRD §3 term | Syntax | Reads |
|---|---|---|---|
| 1 | interference | `when A aligns B` / `opposes` | phase |
| 2 | phase shift | `invert A` | phase → phase |
| 3 | scale modulation | `when A resonates B` / `detunes` | frequency and scale |

```
task carrier at 440 hz phase +
task probe   at 440 hz phase - scale 1.15

when carrier resonates probe {   # gate 3: detuning 0.0999 <= 1/8 — runs
    invert probe                 # gate 2: the exact pi shift
}
when carrier aligns probe {      # gate 1, now that probe flipped
    fork carrier                 # A1: exactly 2 children
}
```

The gates are **independent**, not three spellings of one test: `carrier` and `probe` above interfere destructively *and* resonate. Each ignores what the others read.

## A2 is enforced by the lexer

Every other subsystem honours A2 *negatively*, by not defining a `bool`-shaped type. `Phase` goes as far as a type can — no `From<bool>`, no `Into<bool>`, two inhabitants.

A language can go further, and this is the only place in NEOS where it can. `true`, `false`, `if`, `else`, `&&`, `||`, `!`, `==`, `!=`, `and`, `or`, `not`, and `bool` are **rejected at lex time**, with an error naming the axiom and the construct to use instead. There is no expression grammar to fall back to, so A2 cannot be violated by a programmer in a hurry.

**There is no scale comparison gate.** `when A above B` would be a relational Boolean operator wearing a geometric name. Gate 3 asks instead whether a standing wave between the two would survive — two-valued for a physical reason rather than by fiat.

## Scope

**Owns:** `neos/symphony/lang/**`
**PRD sections:** §3 (Symphony Layer)
**Axioms that bind it:** A1 (bifurcation semantics), A2 (phase-based branching, no `bool`), A3 (curved addressing — `vm`'s `store`/`load`)
**Depends on:** [[symphony-kernel]], and now [[substrate]] and [[lattice]] directly (`vm`'s real memory bus and `⊗`-fold path addressing)

This record derives **no new mathematics**. Every quantity is priced, split, or evaluated by the kernel. The crate has no `build.rs` because it reads no constants — the clearest evidence the split was drawn in the right place.

## Slice 3 — the instruction-executing state machine (`vm`), and a proposal that would have broken A2

A conversation-borne architecture proposal asked for "a core bytecode or AST interpreter supporting register/stack state, arithmetic, branching" here, as the base of a four-phase virtualization plan. Checked against `_mkb/axioms.md` before writing anything: **A2 binds this record by name** — *"No `bool` in Symphony-layer logic; no `if (x == true)`"* — and this record's own "Not built" list already said why: an expression grammar raises "what does `a == b` mean," and under A2 there is no answer. Register/stack arithmetic is that grammar under a different name. Declined, and [`_mkb/instruction_set.md`](../../_mkb/instruction_set.md) was built instead — a **full** instruction set, composed entirely from constructs that already have real, tested meaning.

`vm::compile` flattens the parsed `Stmt` tree into a program-counter-addressed `Instruction` sequence — `EVAL`/`RESONATE` (the same two gates, one instruction discriminated by `Alignment`, exactly as `Stmt::Branch` already is), `SHIFT` (`Invert`), `FORK`, `EMIT`, unchanged in meaning — plus three genuinely new instructions: `STORE`/`LOAD` (a task's physical state — frequency, phase, scale — moved into and out of real curved memory via `substrate::MemoryPool::address_at`, never a flat offset) and `ACQUIRE`/`RELEASE` (real calls into `symphony_kernel::resources::ResourceTracker`, closing the exact gap this record's own "Not built" section named: *"nothing in the language declares resource acquisition"*).

**`Vm::run_batch`** runs several compiled programs against one shared `MemoryPool`/`ResourceTracker`/`WaitForGraph`. A runtime fault — an out-of-range cell, a `load` from memory nothing ever `store`d into, a resource another program in the batch holds — traps only the faulting program; the rest of the batch keeps running against the same, undamaged shared state. This is the substantive part of the proposal's "true virtualization" claim, delivered without needing its Phase 4 (privilege domains), which still has no law behind it anywhere in `_mkb/` — recorded as a real, open gap, not built around.

Verified before being trusted: the flat dispatcher must produce **exactly** the tree-walker's own `Execution` shape for real programs (declared/emitted/forks/inversions/branches, bit-for-bit), or the flattening would be a different language wearing the same syntax. Confirmed directly, not assumed.

**`⊗`-fold path addressing, initially deferred, closed in a follow-up pass.** The first version of this slice recorded cell-ordinal addressing as a deliberate scope boundary, path addressing left for later. Closed once the dispatch loop itself was proven: `store`/`load` now also accept `path START S1 S2 ...`, a real `lattice::AddressPath` resolved through `substrate::MemoryPool::resolve_path` — the identical `⊗`-fold addressing `lattice::addressing` already defines and `substrate` already tests. `Vm::resolve` is the one place both address forms converge. Values for the new path tests were taken directly from `substrate`'s own already-proven `resolve_path` test cases rather than re-guessed — and one assumption was checked and found wrong before shipping: a path with an *extra* step does not always resolve to a different cell than its bare start (`path 1.0 1.0` and bare `path 1.0` land on the *same* cell in a 200-cell pool; `path 1.0 2.0 1.5` lands on a different one), confirmed with a disposable scratch harness before the test asserting the distinction was written.

**Privilege domains, built as a stated convention rather than composed law.** Every other capability in this slice traces to something real — a gate, `substrate`'s memory bus, `allocate_trapped`'s own fault-dispatch shape. Checked again before writing this: `_mkb/` still names nothing about privilege, guest isolation, or kernel/guest separation, and this record's own prior text called it "the one part of the virtualization proposal genuinely left open — building it now would be invention, not composition." Built anyway, once asked a third time, with that fact restated each time — labelled explicitly as an engineering convention, the same footing as the demo binary's deadlock victim policy. `vm::Domain` (`Kernel`/`Guest`, two-valued in the same A2-compliant spirit as `Phase`/`Alignment`) and `Vm::reserve_cells` mark specific cells — or a whole patch, which is just the set of cells it occupies — off-limits to `Domain::Guest` programs. The check runs against the *resolved* `LatticeAddress`, the one place `store`/`load` actually touch the pool, mirroring where a real MMU checks a translated address rather than a symbolic one; it is never offered to a `run_program_trapped` handler, since a handler able to retry past it would make the boundary meaningless. `_mkb/instruction_set.md` records exactly this distinction — mechanism built, no derived security *policy* claimed.

**Dynamic fault routing, the direct counterpart to `Hypervisor::allocate_trapped`.** `Vm::run_program_trapped` calls a caller-supplied handler on every `store`/`load` memory fault, with `&mut MemoryPool` so a real correction is possible — concretely, seeding a cell a `load` found corrupt, then asking for a retry. A retry re-executes **only the faulting instruction**, at the same program counter; everything the program already accumulated (`declared`, `emitted`, ...) is untouched, so there is no whole-program-restart semantics to reason about, and no risk of a prior `task` being redeclared or a prior `emit` firing twice. `max_retries` bounds the total, identically to `allocate_trapped`'s own second guard. Scoped the same way too: `UndeclaredTask`/`DuplicateTask` — the program's own logic errors — are never offered to the handler, since no amount of retrying fixes an undeclared name. `run_program` is unchanged and is now defined in terms of this: `max_retries: 0` with a handler that always propagates, so the entire existing test suite (65 tests) doubles as a regression guard that the untrapped path's behaviour did not shift.

## Slice 4 — real concurrency (`concurrent`), the one limit `vm` never closes itself

`vm::run_program_trapped`'s own doc names the limit plainly: `Vm::run_batch` runs one program to completion at a time, so a blocked `acquire` has to trap — there is no scheduler to suspend and later resume it. `concurrent::run_program`/`run_batch_concurrent` are that scheduler, built from real OS threads rather than teaching `Vm` to simulate one. Unlike Phase 4's privilege domains, this closes by **composing**, the same category as Phases 1-3: `symphony_kernel::ResourceTracker`'s own semantics (idempotent re-ask while blocked, one outstanding wait per task) are completely unchanged — a new `symphony_kernel::ConcurrentTracker` (`Mutex` + `Condvar`, the resource-side sibling of the existing `ConcurrentPool`) only adds the wait/wake mechanism around calls to that same, already-tested logic. `ACQUIRE` inside `concurrent::run_program` calls `ConcurrentTracker::blocking_acquire`, which suspends the **calling OS thread** until granted — verified by timing a real wait (`Duration::from_millis(150)` held, the second thread's own `blocking_acquire` measured to take at least that long), not inferred from the API's shape.

`concurrent.rs` is a **second dispatch loop**, not a generalisation of `vm::Vm` — `Vm<'a>`'s exclusive-borrow design is the right shape for one thread owning its pool/tracker outright, and the wrong shape for sharing across real threads at the same time, which needs owned `Arc` handles instead. Rather than force one type to serve both shapes behind an abstraction, `concurrent::run_program` duplicates the ~150-line dispatch logic against the `Arc`'d forms — the same trade `ConcurrentPool` already made against plain `MemoryPool` rather than unifying the two behind a trait. `vm::Vm` itself, and all 69 tests written against it, are completely untouched by this slice.

**A real deadlock can now really happen, and resolving it surfaced a genuine subtlety the sequential demo never had to face.** `ConcurrentTracker::force_release_all` (backed by a new `ResourceTracker::resources_held_by`) generalises the sequential demo's hand-picked "the victim holds exactly this one resource" into "release everything the victim currently holds" — necessary once the scenario isn't known in advance. The first version of the real-threads deadlock test had each philosopher `.unwrap()` its own `release` calls and hung on first run: after the watchdog force-releases the victim's one held fork, the victim's *own thread keeps running* and eventually reaches its own `release` call for that exact fork — which it no longer holds, so `NotHolder` fires, and `.unwrap()` turns that into a panic mid-thread, which stalls the whole test on `join()`. A hand-sequenced single-thread scenario never has to reckon with the victim's own control flow continuing past the point it was preempted; a real thread does. Fixed by having each philosopher tolerate `NotHolder` on release (`let _ = tracker.release(...)`) — the honest shape of a task written to survive real preemption, not a workaround.

## Slice 5 — a genuine multi-tenant sandbox (`sandbox`), composing rather than adding a new check

Slice 4's own answer to "what does real concurrency buy this crate" named the thing directly: *"a genuine multi-tenant sandbox — several untrusted symphony-lang programs actually running in parallel against shared curved memory."* `sandbox::Sandbox` builds exactly that, and — checked against `_mkb/` first, same discipline as every other capability in this record — needed no new law and no new enforcement point to do it. `vm::Domain`/`Vm::reserve_cells` are two-valued (`Kernel`/`Guest`): every guest shares the *same* restricted region as every other guest, which is a privilege distinction, not a tenancy one. `Sandbox` adds a `CellId -> Owner` map (`Owner::Kernel` or `Owner::Tenant(TaskId)`) entirely *outside* the privilege check itself: for the tenant about to run, it computes the set of cells everyone else owns and hands that set to `Domain::Guest`'s already-real `PrivilegeViolation` check via `concurrent::run_program`, byte-for-byte unchanged. `Sandbox::run_many` runs every tenant on its own real OS thread against one shared `ConcurrentPool`/`ConcurrentTracker` — the same real-concurrency discipline Slice 4 established, not a second one.

**A real, stated limit, recorded rather than hidden:** `ResourceId`s are a shared namespace across tenants. `Sandbox` does not remap or scope resource ids per tenant, so two tenants that happen to `acquire` the same literal id genuinely contend with each other through the one real `ConcurrentTracker` — intentional if the resource is meant to be shared, a genuine collision otherwise, and `Sandbox` does not attempt to tell the two apart. See `_mkb/instruction_set.md` for the full statement.

Admission is exclusive and checked before any ownership is written: `admit_tenant`/`reserve_kernel_cells` refuse — leaving the map exactly as it was — if any requested cell is already owned by anyone else, so a later admission can never silently steal an earlier tenant's memory. Re-admitting a tenant to cells it already owns is a no-op, not an error, since nothing was taken from anyone.

## Doctrine checks — twelve performed

| Sabotage | Result |
|---|---|
| `Eval`'s not-taken jump off by one (`skip` instead of `1 + skip`) | **1 of 56 failed** — `bytecode_dispatch_matches_the_tree_walker_exactly`: the not-taken branch's first instruction ran anyway |
| `run_batch` stops the whole batch at the first trap | **1 of 56 failed** — `a_fault_isolates_only_the_faulting_program_in_a_batch`: only 2 of 3 programs ran |
| `Acquired::Blocked` treated as `Granted` | **1 of 56 failed** — `a_blocked_acquire_traps_rather_than_hanging_or_silently_granting`: no trap where one was required |
| `Address::Path`'s steps ignored in `Vm::resolve` | **2 of 61 failed** — `a_path_that_leaves_otimes_domain_traps_as_a_memory_fault` (no longer left the domain once the extra steps were dropped) and `a_paths_steps_resolve_to_a_different_real_cell_than_its_bare_start` (both addresses resolved to the same cell) |
| `run_program_trapped`'s retry bound off by one (`retries <= max_retries`) | **1 of 65 failed** — `retries_are_bounded_a_handler_that_never_helps_cannot_hang`: 5 handler calls instead of the expected 4 for `max_retries: 3` |
| `TrapAction::Propagate` ignored (retried regardless of the handler's answer) | **1 of 65 failed** — `propagate_still_traps_immediately_even_with_retries_available`: 11 handler calls instead of 1 |
| Privilege check disabled entirely (`if false && ...`) | **4 of 69 failed** — every reserved-cell test, both `Address::Cell` and `Address::Path` forms |
| Privilege check's domain condition inverted (`Domain::Kernel` instead of `Domain::Guest`) | **4 of 69 failed** — the same four, this time for the opposite reason: `Domain::Kernel` was wrongly blocked and `Domain::Guest` wrongly let through |
| `ConcurrentTracker::blocking_acquire`'s wait removed (`Blocked` treated as `Granted`) | **3 tests failed across two crates** — the timing test returned in microseconds instead of waiting out the real hold; the real-threads deadlock test found no cycle at all (no wait ever recorded); the language-level mutual-exclusion test observed real cross-thread interleaving corruption |
| `ResourceTracker::resources_held_by` returns nothing | **1 failed** — `two_real_threads_deadlock_and_the_watchdog_resolves_it`'s own `released must not be empty` assertion, caught before either thread could be joined |
| `Sandbox::off_limits_for`'s ownership filter inverted (`==` instead of `!=`) | **3 of 78 failed** — every isolation test: `a_tenant_can_freely_use_its_own_admitted_memory` (now refused its own memory), `a_tenant_cannot_touch_another_tenants_memory` (now let through), `tenants_run_concurrently_and_stay_isolated_under_real_contention` (panicked looking up its own declared task, since its own store was wrongly refused) |
| `Sandbox::admit`'s conflict check removed entirely | **2 of 78 failed** — `admitting_a_tenant_to_an_already_kernel_owned_cell_is_refused` and `admitting_two_different_tenants_to_the_same_cell_is_refused`, both `unwrap_err()` on an `Ok` |

All twelve reverted after confirming; full suite re-confirmed at 78/78, 460/460 workspace-wide.

## Three findings worth carrying

**Slice 2's derivations were already in the constants file.** Gate 2 needed only the observation that A2's orientations are separated by exactly `π` — which is exactly `teardown_phase_shift`, already stored and already used by `ftg`. Gate 3's band is `(π/4)/(2π) = 1/8`, and `π/4` was likewise already stored. Neither needed new physics; both needed someone to compose two facts that were sitting next to each other.

**Deriving gate 3 exposed a defect in shipped kernel code.** `ξ` returned `+inf` above `r ≈ 710.5` and `NaN` above `~746`, as `Ok` values — violating the boundedness the law states and relies on for clock-path safety. The guarding test swept `r ∈ [0, 30]`, where the bug does not appear: **a correct assertion over an unrepresentative domain**, which is a distinct failure mode from a wrong tolerance or a vacuous assertion. Fixed algebraically in [[symphony-kernel]], not clamped.



**A sabotage that broke nothing was right not to.** The test plan predicted that treating `opposes` as `else` would fail. It failed nothing — because A2 admits exactly two phase orientations, so alignment is a two-valued predicate and per-branch `opposes` genuinely *is* the complement of `aligns`. A test claiming otherwise was asserting something false. It now states the weaker true position, plus the thing that actually distinguishes this from `if`/`else`: the two forms are **independent statements**, so a program can take both or neither.

**The ⊗ ceiling, fourth appearance.** After `lattice` addressing, the kernel's domain guard, and `crystallisation` bifurcation. Four subsystems, four arities, one constraint — refused in all four, clamped in none.

## Deviation from the target tree

[`_spec/target-tree.md`](../../_spec/target-tree.md) lists `symphony/compiler/` and `symphony/interpreter/` as separate crates. Built as **one** crate with three modules: they share `Token`, `Stmt`, and `LangError`, and have no independent consumer. Splitting along stage names is what the surgical-split rule warns against. Recorded in the math contract.

## Not built

Absent by design: expressions, assignment, loops, functions, modules, general arithmetic. Adding an expression grammar would immediately raise "what does `a == b` mean?", and under A2 the answer is *nothing* — this is why the conversation-borne virtualization proposal above was declined as specified rather than implemented literally.

Also absent, and each for a stated reason:

- **A gate combinator** (`resonates AND aligns`). It would need a truth table — the thing A2 removes. Nesting a branch inside another already composes gates.
- **A scale comparison.** See above.
- **Deadlock *resolution* through the language.** `vm`'s `acquire`/`release` now let a program *declare* resource acquisition (closing the gap this section used to name), and a blocked acquire traps rather than resolving anything — the kernel's detection/resolution boundary is unchanged; resolving a cycle is still application-level, demonstrated in `neos/src/main.rs`, never inside this crate.
- **`vm::Vm` itself still has no scheduler.** `Vm::run_batch` remains sequential by design; real concurrency lives in the separate `concurrent` module (see Slice 4 above), not inside `Vm`.
- **Deadlock resolution inside `concurrent`.** It detects (via the unchanged `WaitForGraph`) but never resolves a cycle — resolution stays application-level, demonstrated in `neos/src/main.rs`'s own watchdog, the identical boundary the sequential kernel demo already keeps.
- **A derived privilege policy.** `vm::Domain`/`Vm::reserve_cells` (built — see above) are the enforcement *mechanism*; still absent, because no law defines it either, is any claim about what should be protected, how many domains beyond two are meaningful, or anything resembling a derived security model. The policy is left entirely to whatever host calls `reserve_cells`.
- **Per-tenant resource namespacing.** `sandbox::Sandbox` (Slice 5) isolates memory ownership; `ResourceId`s stay a namespace shared across every tenant, stated plainly rather than hidden — see Slice 5 above.

## Do not

Load other subsystems' records. They don't share state; they share the factory (`_mkb/`, `_spec/`).
