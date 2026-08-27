---
type: subsystem
subsystem: symphony-kernel
tier: 2
language: Rust
stage: 04_implement
status: complete
result: "79 tests passing. Quantization, resonance, equilibrium, scheduler policy, deadlock detection, a resource-acquisition tracker feeding the wait-for graph, ConcurrentPool synchronising substrate::MemoryPool, ConcurrentTracker giving real OS threads real blocking acquire/release over that same tracker, A1/A2 handlers, and all three PRD 3 logic gates (interference, phase shift, scale modulation). Deadlock resolution is deliberately out of scope here — demonstrated in neos/src/main.rs at application level instead, using this crate's own unmodified public API, now for a genuinely concurrent scenario too."
slices: ["quantization + resonance + equilibrium", "scheduler policy + deadlock + axiom handlers", "the three gates", "resource tracker feeding the wait-for graph", "concurrent access to substrate::MemoryPool", "concurrent access to the resource tracker, with real blocking"]
prd_sections: ["4"]
binds_axioms: ["A1", "A2"]
split_from: symphony
consumes: [lattice]
---

# Symphony-kernel — scheduler, quantization, equilibrium

One job: run processes as energy states on a self-stabilising harmonic field, rather than as time-sliced threads on a priority queue.

## The build loop

| Stage | Job | Output |
|---|---|---|
| `01_derive` | pull the exact law that binds this subsystem | `math-contract.md` |
| `02_design` | types and interfaces against that contract | `design.md` |
| `03_tests` | physics assertions, written before code | `test-plan.md` |
| `04_implement` | write the Rust into `neos/symphony/kernel/` | `implementation-log.md` |

## Scope

**Owns:** `neos/symphony/kernel/**` — `scheduler.rs`, `quantization.rs`, `equilibrium.rs`
**PRD sections:** §4 (Kernel and Resource Management)
**Axioms that bind it:** A1 (process bifurcation, $1\times1=2$), A2 (phase-based branching, no `bool`)
**Equations that bind it:** Howard Equation; Harmonic Force Equilibrium; Resonance Correction — all in [`_mkb/resonance.md`](../../_mkb/resonance.md)
**Constants read:** `howard_comma`, `logic_phases`, `resonance.*`

## Consumes `lattice` — does not rebuild it

Core topology and cell naming come from the `lattice` crate: `Tiling`, `Cell::neighbors()`, `CellId`. The `{5,4}` neighbour-naming framework is **already built and tested** there (38 assertions).

**Do not reimplement tiling or neighbour resolution here.** One home per fact applies to code as firmly as to prose. This record maps cores onto cells and reads adjacency; it does not generate geometry.

That is what makes "naming surrounding nodes without runtime discovery overhead" true: adjacency is a closed-form group operation in `lattice`, not a search.

## Four hard constraints

Carried from [reconciliation R5/R6](../../_mkb/reconciliation.md) and non-negotiable:

- **Mean-centre task density.** `Σρᵢ = 0` is the solvability condition for the field equation. Absolute load has no solution.
- **Derive the coupling from topology.** `α < 2/λ_max(L)`. A hardcoded `α` oscillates at some core count — the thrashing the model exists to prevent.
- **`ξ(r)` must stay bounded.** It sits in the clock path; an unbounded correction is worse than none.
- **Deadlock detection is still required.** Load equilibrium eliminates thrashing and bottlenecks, not circular waits on resource acquisition.

## The resource tracker feeds the graph; it does not resolve anything

`WaitForGraph::detect_cycle` only ever sees edges someone else recorded — its own implementation log said so directly: "nothing yet acquires or releases resources." `resources::ResourceTracker::acquire`/`release` is that someone: it tracks which task holds which opaque `ResourceId` and turns acquire/release calls straight into the graph's `add_wait`/`remove_wait` edges, so no caller computes "who holds this" by hand.

Kept strictly on the detection side of the boundary this record has stated from the start (§8 above — deadlock *detection* lives here, *resolution* is explicitly application-level): granting a freed resource to the next queued waiter is bookkeeping the tracker must do to keep the graph *accurate* — a released resource cannot still show a wait edge pointing at its old holder — not deadlock resolution. Nothing here ever breaks a cycle that already exists; a held cycle stays held.

One invariant makes the bookkeeping exact rather than approximate: a task has **at most one outstanding wait edge**, enforced by refusing a second `acquire` on a different resource while one is already pending (`ResourceError::AlreadyWaiting`). Every deadlock the contract names — two locks taken in opposite orders — blocks on exactly one resource at a time, so this is not a restriction beyond what the law requires. It is also what lets `release` retarget every remaining queued waiter's edge from the departing holder to the newly granted one with a plain loop, rather than needing to reason about which of a waiter's several edges is the stale one.

`WaitForGraph` gained one new primitive for this: `remove_wait(waiter, holder)`, deleting exactly that edge rather than every edge the waiter has (`clear_waits`'s job). The distinction never shows up while going through `ResourceTracker` alone — the one-edge invariant means the two are behaviourally identical on every path the tracker exercises — so `remove_wait`'s own contract is asserted directly against `WaitForGraph`, independent of the tracker. See the addendum in the implementation log for how sabotage caught this.

## `ConcurrentPool` — the synchronisation decision `substrate` left open

`substrate::MemoryPool` is deliberately single-threaded; its own implementation log named the two candidate homes for a lock explicitly: "here or in `symphony-kernel`," calling it "a scheduler decision, not a substrate one." `memory::ConcurrentPool` is that decision, made here: one coarse `Mutex<MemoryPool>`, serialising every operation rather than locking per cell. Per-cell locking was considered and rejected — `MemoryPool::allocate` already grows breadth-first over a dynamically discovered set of adjacent cells in one call, and locking that set correctly (consistent order, no holding-while-waiting) would risk reintroducing the exact deadlock class [`deadlock`](../../neos/symphony/kernel/src/deadlock.rs) exists to catch, for a demo-scale pool that doesn't need the throughput.

Verified before trusting it, not assumed: a disposable scratch harness wrapped a raw `MemoryPool` in an `unsafe impl Sync` with zero synchronisation and hammered it with 32 real threads, each writing its own fingerprint byte and reading it back after a forced yield. Unsynchronised: 10/10 runs corrupted. The identical workload through a `Mutex`: 0/10. (The first version of that harness used the *same* fill byte for every thread and looked clean for exactly the wrong reason — an aliased allocation reads back "correctly" if what overwrote it happened to match.)

Building this surfaced a real, load-bearing finding that turned out to have nothing to do with concurrency at all — see [substrate's own addendum](../substrate/04_implement/output/implementation-log.md): `MemoryPool::free` reset a cell's usage unconditionally, corrupting a still-live sibling allocation sharing that cell. Fixed in `substrate`, not here; `ConcurrentPool` only exposed it by being the first caller to actually hammer shared cells hard enough.

## `ConcurrentTracker` — `ConcurrentPool`'s sibling, but for a *blocking* operation

`ConcurrentPool` only had to prove *safety*: every `MemoryPool` operation it wraps is already total, so a `Mutex` around it just has to prevent corruption of otherwise-instant calls. `resources::ResourceTracker::acquire` is not total the same way — it can legitimately answer "not yet" — and nothing about wrapping it in a `Mutex` alone gives a caller anything to *do* with that answer beyond what the sequential code already did. `concurrent_resources::ConcurrentTracker` (`Mutex<(ResourceTracker, WaitForGraph)>` + `Condvar`) adds the missing piece: `blocking_acquire` suspends the **calling OS thread** on the condvar until `acquire` reports `Granted`, and `release` wakes every waiter after updating the tracker. `ResourceTracker`/`WaitForGraph`'s own semantics are completely unchanged — every existing sequential test still exercises the identical logic; this only adds a real wait/wake mechanism around calls to it.

Built to serve [[symphony-lang]]'s stated real limit — `vm::Vm`'s own docs say plainly that a blocked `acquire` traps because "there is no scheduler able to suspend a blocked program and resume it once the holder releases." This is that scheduler.

**Verified as a real wait, not inferred from the API's shape**: one thread holds a resource for a measured 150ms; a second thread's `blocking_acquire` call for the same resource is timed and must take at least that long, not microseconds. Sabotage (the condvar wait removed, `Blocked` treated as `Granted`) confirmed the guard is load-bearing three separate ways: the timing test returned instantly, a real two-thread deadlock scenario recorded no wait edges at all (nothing to detect), and — the sharpest of the three — `symphony-lang`'s own language-level mutual-exclusion test observed **actual cross-thread data corruption**, one thread reading back a value another thread had just written to a shared cell.

**A real deadlock can now really happen, and resolving it needed one new primitive.** `ResourceTracker::resources_held_by(task)` generalises this workspace's existing sequential deadlock demo, which hand-picks "the victim holds exactly this one resource" because it built the scenario itself — a resolver that doesn't know the scenario in advance has to ask what the victim actually holds. `ConcurrentTracker::force_release_all` uses it to release everything at once. Building the real-threads version of the classic two-lock-inversion test surfaced a genuine subtlety a single-thread scenario structurally cannot: after force-releasing the victim's one held resource, the victim's *own thread keeps running* and eventually tries to release that same resource itself — which it no longer holds. The fix is a task written to tolerate `NotHolder` on release, not a change to `ConcurrentTracker`: this is what a task actually built to survive preemption looks like, a distinction that only exists once there is a second thread to preempt.

## A real coverage gap, found from outside this record

`neos/tests/geometric_testbed.rs` (a cross-cutting harness owned by neither `symphony-kernel` nor `ftg` — see root `CONTEXT.md`'s cross-cutting slices) removed `CoreTopology::stability_bound`'s `.max(1)` guard on `d_max` and found this crate's own 79-test suite did not notice: no existing test ever builds a single-core (`CoreTopology::from_tiling(1)`), zero-degree topology, so the resulting division-by-zero (`stability_bound()` returning `inf` instead of a finite bound) went completely unobserved. Caught immediately by the test bed's own explicit degenerate-extreme test. The guard itself was correct and is unchanged; this is a gap in *test coverage*, not a defect in the guard, recorded here rather than silently left for the next person to rediscover.

## Do not

Load [[symphony-lang]] or other subsystems' records. They don't share state; they share the factory.
