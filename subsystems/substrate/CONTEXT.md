---
type: subsystem
subsystem: substrate
tier: 1
language: Rust
stage: 04_implement
status: complete
result: "43 tests passing. Non-Euclidean memory, binary-wave translation, clock, curved-path resolution, MemoryPool wired to lattice's LogicalArea for size reporting, a real free() correctness fix (shared cells), sub-cell allocation reuse (real interval tracking, not a whole-cell bump allocator), and Hypervisor::allocate_trapped (real fault dispatch with recovery). Concurrency synchronisation lives in symphony-kernel's ConcurrentPool, by design. No open items remain — guest isolation/privilege levels have no _mkb/ backing and no guest execution engine exists to isolate; recorded as a deliberate boundary, not a gap."
prd_sections: ["3"]
binds_axioms: ["A1", "A3"]
consumes: [lattice]
---

# Substrate — the Rust hypervisor

One job: provide the foundational virtual machine that translates wave functions into optimized hardware instructions, with strict memory safety and concurrency for the raw binary computation underneath.

This is the floor. Everything else in NEOS runs on it, which makes it the honest place to start building.

## The build loop

| Stage | Job | Output |
|---|---|---|
| `01_derive` | pull the exact law that binds this subsystem | `math-contract.md` |
| `02_design` | types and interfaces against that contract | `design.md` |
| `03_tests` | physics assertions, written before code | `test-plan.md` |
| `04_implement` | write the Rust into `neos/substrate/` | `implementation-log.md` |

## Scope

**Owns:** `neos/substrate/**`, root `neos/Cargo.toml`
**PRD sections:** §3 (Substrate Layer)
**Axioms that bind it:** A1 (memory pool splitting), A3 (non-Euclidean memory addressing)
**Equations that bind it:** carrier baseline $\omega_c$ from `constants.md`; addressing goes through the hyperbolic distance function
**Constants read:** `baseline_carrier_frequency`, `hyperbolic_curvature`

## The tension, resolved

Rust's memory model is flat and Euclidean; A3 says addressable space is not.

**Settled: the boundary is the public API of `memory`.** `FlatOffset` is private and is the return type of the private `resolve` — the one function performing the translation. No public type, method, or field yields a pointer, byte index, or linear address. Verified by a compile probe: four separate leak attempts, all refused.

Consequence for everything downstream — [[ftg]] especially — is that hyperbolic routing reads a native non-Euclidean space, not a flat abstraction wearing geometric names.

**Layering settled too:** the frequency newtypes live here, since substrate's clock is `ω_c`. Dependency direction is `lattice ← substrate ← symphony-kernel`, matching the PRD's tiering.

## Size reporting, wired to `lattice::LogicalArea`

`lattice` proved fragmentation structurally zero against a bare cell count; `MemoryPool::total_area`/`occupied_area`/`available_area` and `Allocation::logical_area` wire that same type to a real pool's real allocation lifecycle — pure derived reads over `self.slabs`, no new state. A cell counts as occupied the instant it holds any bytes, matching `LogicalArea`'s own all-or-nothing cell model.

Sabotage caught a blind spot in the test design itself before it caught a code defect: every test allocates in exact multiples of `cell_capacity` (to sidestep a *different* ambiguity, two allocations sharing one cell's byte capacity), which meant every occupied cell in those tests was also completely full — so a mutation redefining "occupied" as "completely full" passed all of them silently. A dedicated partial-fill test closed it. Full account in the implementation log's addendum.

## `free`: from broken, to safe-but-pessimistic, to precise

Found while building `symphony-kernel::ConcurrentPool` — real under plain single-threaded use too, no concurrency required to trigger it. `free` reset a cell's `used` byte count to zero unconditionally, without checking whether another still-live allocation also had bytes in that same cell. Freeing the first of two allocations sharing a cell reported the *whole* cell available again, and a subsequent `allocate` would then hand the second allocation's still-live bytes to someone else — silent data corruption, reproduced deterministically with zero threads (two 64-byte allocations in one 256-byte cell, free the first, allocate again, read the second: wrong data comes back).

First fix: `Slab::live`, a count of allocations currently touching a cell; `used` only reset to zero once that count reached zero. Safe, but pessimistic — freeing one allocation from a shared cell reclaimed none of its bytes until *every* allocation touching that cell was also gone.

That was superseded the same session, closing "allocation reuse below cell granularity" (this record's own long-standing "not built" item): `Slab::live` became a real interval set, `(offset, len)` per live span rather than a bare count, with `first_gap` finding reusable holes and `Allocation` tracking exactly which byte range it holds per cell. `free` now releases precisely what was granted, immediately — a hole freed in the *middle* of a cell is offered to the next `allocate` ahead of untouched space further out (first-fit), not just at the cell's edges. Every existing test exercising `free` uses exact `cell_capacity` multiples specifically to avoid cell-sharing (their own comments say so), so none of them could have caught the original bug, and both fixes are behaviourally identical to the original code whenever a cell holds exactly one live allocation — neither changes anything any existing test asserted. Full account, including the sabotage gate for each step, in the implementation log's addendum.

## Fault trapping — the honest half of "virtualisation proper"

The PRD names "trapping, guest isolation, privilege levels" under "Virtual Machine," but `_mkb/` has zero mentions of any of the three — no law to derive from, unlike every other synthesis this project has done. Worse: there is no guest execution engine anywhere in the workspace for isolation or privilege levels to apply to. `symphony-lang`'s own docs are explicit that it deliberately stays a narrow declarative DSL — no arithmetic, no memory access, no general loops, no function calls — not a Turing-complete language a "guest" could run arbitrary code in. Building real isolation around that would mean inventing a general-purpose execution engine first, reversing a documented design decision, then inventing privilege semantics with nothing to check them against.

What *does* have real substance: `SubstrateError`'s variants already read as fault codes (`Exhausted`, `Unmapped`, `OffsetOutOfCell`, ...) but nothing dispatched them anywhere — they were just `Result`s. `Hypervisor::allocate_trapped(bytes, max_retries, handler)` is a genuine trap path: the handler is called on **every** fault, unconditionally (real transfer of control, not a conditional notification), receives `&mut MemoryPool` directly — the same pool the fault came from — and decides `Retry` or `Propagate`. A handler that frees an allocation it knows about and asks to retry gets a real, working retried allocation back, the same shape as a page fault a real OS resolves by evicting a page. `max_retries` is a second guard, the same shape as `ftg::Router::route`'s `max_hops`, so a handler that never actually resolves anything cannot hang the caller.

Deliberately scoped to `allocate` only, not `read`/`write`: an `Exhausted` fault has a real corrective action a handler can take; `Unmapped`/`OffsetOutOfCell` are wrong-argument errors from the caller's own logic that no handler acting on the pool could fix, so trapping them would add an interface with nothing real behind it.

## Do not

Load other subsystems' records. They don't share state; they share the factory (`_mkb/`, `_spec/`).
