---
type: implementation-log
subsystem: symphony-kernel
stage: 04_implement
status: complete
toolchain: rustc 1.97.1 / cargo 1.97.1
result: 79 passed, 0 failed (453 workspace-wide) across symphony_kernel + symphony_scheduler — see ConcurrentPool and ConcurrentTracker addenda
consumes: [lattice, substrate]
---

# Symphony-kernel — Implementation Log

## Result

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 62 passed; 0 failed
                           24 lattice metric · 14 lattice tiling · 24 kernel
```

Tests were written before the implementation and confirmed failing first.

## Files written

| Path | Role |
|---|---|
| `neos/symphony/kernel/Cargo.toml` | crate manifest, depends on `lattice` |
| `neos/symphony/kernel/build.rs` | constants from `_mkb/constants.json` |
| `neos/symphony/kernel/src/lib.rs` | crate root, `KernelError` |
| `neos/symphony/kernel/src/quantization.rs` | `E = C_H·ν`, frequency types |
| `neos/symphony/kernel/src/resonance.rs` | `ξ(r)`, `DriftIntegrator` |
| `neos/symphony/kernel/src/equilibrium.rs` | `CoreTopology`, `LoadField` |
| `neos/tests/symphony_kernel.rs` | 24 assertions |

## The ν/ω separation is enforced by the compiler

Contract §2.1 called this the subsystem's highest-risk defect: `ν` and `ω` differ by `2π`, and **the units cannot tell them apart** — both are J·s × s⁻¹ = J. No runtime assertion would catch a substitution.

`Frequency` and `AngularFrequency` are distinct newtypes with no shared arithmetic and no `From` impl. Conversion is explicit (`to_angular` / `to_ordinary`).

**Verified by compile check.** A probe test passing an `AngularFrequency` to `energy()` was written and compiled:

```
error[E0308]: mismatched types
   |             ------ ^^^^^ expected `Frequency`, found `AngularFrequency`
```

The probe was removed after confirming. This is the one guarantee in the subsystem that is stronger than a test — it cannot be violated at all, rather than merely detected.

`build.rs` additionally asserts `howard_comma.frequency_variable == "nu"` and fails the build otherwise, so the JSON cannot drift into the other convention silently.

## Doctrine checks — three performed

| Sabotage | Result |
|---|---|
| `ξ` → the rejected `sinh(1)/sinh(r/R)·e^(−r/R)` form | **3 failed** — bounded, exact-at-reference, and monotonicity all caught it |
| `C_H` → Planck `h` in the JSON | **4 failed** — including the ratio test pinning the deliberate `0.3989×` departure |
| `ω` passed to `energy()` | **compile error** — cannot be expressed |

All reverted; full suite re-confirmed at 62/62 with no markers remaining.

## A test I got wrong

`drift_integral_converges_under_damping` initially modelled a geometrically alternating decaying error and asserted `H(κ) → 0`. That series converges to `A·dt/(1+r) = 0.0067`, **not** to zero — the test failed, and the test was what was wrong.

The invariant is *cancellation across scales*, not decay. Rewritten as paired `+e, −e` observations under a decaying envelope, which drives the integral to zero genuinely. A second test now pins the distinction explicitly, asserting that a decaying non-cancelling sequence settles at `0.0067` and is correctly flagged as *not* converging — so the difference cannot be mistaken for a loosened tolerance later.

## `lattice` is consumed, not reimplemented

`CoreTopology::from_tiling` grows a `lattice::Tiling` and takes adjacency from `Cell::neighbors()`. No `{5,4}` geometry is recomputed here — contract §6.

Cores occupy cells in BFS ring order, so an *n*-core machine is a compact patch around the origin. Adjacency is then a closed-form group operation rather than a search, which is what "naming surrounding nodes without runtime discovery overhead" means concretely.

**One consequence worth stating:** a bounded patch is not vertex-transitive. Interior cells have 5 face-neighbours; boundary cells have as few as 1 *inside the patch*. The Laplacian is built from in-patch degree, and `max_degree()` is measured rather than assumed — a balancer hardcoding degree 5 would mis-weight the boundary. Asserted in test 3.6.

## Verified equilibrium behaviour

Load `[4n, 0, 0, …]` relaxed on real `{5,4}` adjacency:

| cores | degrees min/max | α bound | spread before → after | total |
|---|---|---|---|---|
| 7 | 1 / 5 | 0.2000 | 28.0 → 2.7e-15 | conserved |
| 16 | 1 / 5 | 0.2000 | 64.0 → 3.6e-15 | conserved |
| 31 | 1 / 5 | 0.2000 | 124.0 → 1.5e-14 | conserved |
| 64 | 1 / 5 | 0.2000 | 256.0 → 2.3e-14 | conserved |

Both mandatory constraints are structural rather than checked:

- **Solvability (§5.1):** `task_density()` only ever returns mean-centred values. There is no accessor for absolute load as a density, because absolute load makes `Lφ = −ρ/ε₀` unsolvable.
- **Stability (§5.2):** `relax` rejects `α ≥ stability_bound()` with `Err(Unstable)` rather than oscillating. `λ_max ≤ 2·d_max` by Gershgorin — cheap, safe, no eigensolver.

## An encoding hazard found and fixed

Rewriting `constants.json` from PowerShell added a UTF-8 BOM and mojibake'd the em-dashes, which broke **both** crates' builds — `serde_json` cannot parse a leading BOM.

The file is now ASCII-only with an explicit note in its `$comment` saying so. Since it is machine-parsed at build time by every crate, non-ASCII punctuation buys nothing and risks exactly this.

## What is not built

- **Deadlock detection** — required by contract §8. Load equilibrium eliminates thrashing and bottlenecks; circular waits on resource acquisition are orthogonal and still need handling. A separate slice with its own contract.
- **A1 bifurcation forking** and **A2 phase branching** — both need the runtime task model that [[symphony-lang]] will define. `logic_phases` is therefore unread by this slice.
- **The scheduler proper** — quantization, resonance, and equilibrium are the three mechanisms it composes; the policy that drives them from real task arrivals is the next slice.

## Human check

Run `cargo test --workspace`. Read `xi_is_bounded_everywhere` and `load_converges_to_uniform_equilibrium` — the first is what stops a bad timing sample from stalling the scheduler, the second is the claim that the field model actually balances load, tested against real tiling adjacency rather than a toy ring.

---

# Slice 2 — scheduler policy, deadlock detection, axiom handlers

Added `scheduler.rs`, `deadlock.rs`, `bifurcation.rs`, and `neos/tests/symphony_scheduler.rs` (26 assertions). Workspace total: **90 passing**.

## A1/A2: implemented, not stubbed

The slice asked for "structural stubs" for the axiom handlers. The *arithmetic* of both axioms is already settled law, so stubbing it would have hidden finished work:

- **A1** — `fork()` computes multiplicity via `lattice::LatticeScalar::otimes`, giving exactly `2.0` for the unit fork. The operator has one home and this is not it.
- **A2** — `Phase`, `evaluate_branch`, and `superpose` implement phase-alignment logic against `{−π/2, +π/2}` read from the MKB.

`Phase` deliberately has **no `From<bool>` and no `Into<bool>`**. A conversion would reintroduce classical logic at whichever call site used it.

**The genuine stub is `TaskModel`** — the trait `symphony-lang` will implement to say what a task *is* and what a condition *is*. Nothing in the kernel depends on it yet; it exists to fix the shape of the seam.

## Two design bugs the tests caught

**1. Absolute tolerance on physical magnitudes.** `schedule()` used a fixed `1e-12` convergence threshold while task energies are of order `1e-25` J — so a pathologically imbalanced field reported itself converged before relaxing at all, and `relaxation_steps` came back 0.

Fixed with `LoadField::relative_spread()`, and convergence is now judged on it. A new test (`convergence_criterion_is_scale_free`) asserts a field scaled by `1e-25` converges in exactly the same number of steps as the unscaled one — the property an absolute tolerance cannot have.

**2. `relax_to_equilibrium` skipped validation on early exit.** An already-converged field returned `Ok` without ever checking `α` against the stability bound, so an unstable coupling passed silently. The bound is now validated **before** the loop. Pinned by `already_converged_field_still_rejects_bad_coupling`.

Both were found by tests failing, not by inspection.

## Two migration bugs, same cause

The first migration implementation violated its own stated design in two ways:

**Multi-hop per pass.** The outer loop let a task chain `2 → 5 → 9` inside one `schedule()` call. Each hop was edge-local; the net displacement was not. Fixed: a task moves **at most once per pass**. Work still diffuses further across repeated passes — which is what diffusion means.

**Migration made balance worse.** Measured spread *grew* `5.3e-25 → 1.6e-24`. Load is quantized into whole tasks while the relaxed target is continuous, so an unguarded greedy move overshoots. Fixed with a **strict-improvement criterion**: a move is taken only if it reduces total absolute imbalance across the two cores involved.

## Deadlock detection

Three-colour DFS over the wait-for graph; roots sorted so output is deterministic. `None` is an exhaustive result, not "none found".

**The scope boundary is now executable.** `perfectly_balanced_load_can_still_deadlock` relaxes a 16-core field to broad balance and then asserts a two-task cycle is still detected. That is contract §8 stated as a test rather than a caveat — balanced load must never be mistaken for absence of deadlock.

## Doctrine checks — two performed

| Sabotage | Result |
|---|---|
| `detect_cycle` always returns `None` | **5 failed**, including the balanced-but-deadlocked assertion |
| migration allowed to any core (teleport) | **1 failed** — `task TaskId(1) jumped from core 1 to non-adjacent 30` |

Both reverted; full suite re-confirmed at 90/90 with no markers and no hardcoded MKB constants.

## Still not built

- **Resource acquisition itself.** `WaitForGraph` detects cycles in waits that something else must record. Nothing yet acquires or releases resources.
- **Deadlock *resolution*.** Detection only. Which task to abort, and how to unwind it, is a policy question the runtime task model has to inform.
- **Preemption and time-bounded execution.** Tasks currently occupy a core until reclaimed.
- **`symphony-lang`** — parser, compiler, interpreter. Still deferred; `TaskModel` marks where it plugs in.

---

# Slice — the three gates, and a boundedness defect in `xi`

Driven by [[symphony-lang]] needing PRD §3's other two logic gates. The gates are physics, so they were built **here**, not in the language. Law: [`_mkb/gates.md`](../../../../_mkb/gates.md).

`symphony_kernel` tests: **26 → 38**. Workspace: **282 → 311**.

## Added

| Symbol | Gate | Law |
|---|---|---|
| `Phase::invert` | 2 — phase shift | `gates.md` §2 |
| `resonates(ν, r, ν, r)` | 3 — scale modulation | `gates.md` §3 |
| `detuning(ν, r, ν, r)` | the ratio itself | `gates.md` §3.3 |
| `RESONANCE_BAND` | derived `1/8` | generated from `constants.json` |
| `PHASE_INVERSION_SHIFT` | the exact `π` | generated from `constants.json` |

`build.rs` now **asserts the derivation**: `link_stability_phase_variance / (2π)` must equal `gates.resonance_band` to within one epsilon. They are the same criterion expressed at different points, so editing one without the other stops the build rather than shipping two thresholds that disagree.

That assertion is the same species as the existing `frequency_variable == "nu"` check — a build-time guard on a relationship the type system cannot see.

## The defect: `xi` violated its own boundedness law

Found by building gate 3, which evaluates `xi` at arbitrary user-declared scales — the first time anything asked what `xi` does far from the operating range.

```rust
r.sinh() / (r * 1.0_f64.sinh()) * (1.0 - r).exp()   // the shipped form
```

`sinh(r)` overflows `f64` at `r ≈ 710.5`, **before** `exp(1-r)` can rescue the product:

```
xi(710.0) = 1.6289e-3    fine
xi(710.5) = +inf         returned as Ok
xi(1000)  = NaN          returned as Ok
```

`resonance.md` §1.2 states boundedness as law and gives the reason: *"the correction can never diverge, which is what makes it safe in a clock path."* The implementation violated the invariant it exists to satisfy — and returned it as a **success value**, so nothing downstream could distinguish it from a real correction.

The blast radius is not hypothetical. `Task::energy_joules` catches `Err` and falls back to uncorrected energy, but `Ok(inf)` passes straight through into `load_per_core`, the field mean, `spread`, and every migration decision.

### The fix is algebraic

`e^r · e^(1−r) = e` identically, so

```
xi(r) = (e − e^(1−2r)) / (2r·sinh 1)
```

is the same function and cannot overflow. It loses precision as `r → 0`, where it differences two nearly-equal numbers — so each branch is used where it is exact, split at the reference scale `R`:

- **`r ≤ 1`** — `sinh(r) ≤ sinh(1)` and `exp(1-r) ≤ e`; overflow impossible
- **`r > 1`** — `e^(1−2r) ≤ e⁻¹`; cancellation impossible

The split point is `R` itself, not a tuned threshold, and both branches evaluate to exactly `1.0` there.

**Not a clamp.** Clamping to the supremum would have made the tests pass while leaving the arithmetic wrong — the values above `710.5` are not "too large", they are the correct small values computed through an intermediate that overflows.

### Verified before shipping

| Property | Result |
|---|---|
| agreement with the original form on `[1e-8, 700]` | **2.5 ulp** worst case |
| `xi(1)` | exactly `1.0`, both branches |
| finite, positive, `≤` supremum | every input to `f64::MAX` **and `INFINITY`** |
| strict monotonicity | exact across `[1e-6, 1e6]` |

Below `~1.6e-12` the `f64` representation plateaus — `xi` is flat to `O(r²)` there, which is under one ulp — so consecutive samples tie. That is the float grid, not the function, and it is outside any scale the system declares.

### Why the existing test missed it

`xi_is_bounded_everywhere` sweeps `r ∈ [0, 30]`. The naive form is flawless on that interval, so the test passed continuously and looked like coverage of a law that says *bounded* without qualification.

**A correct assertion over an unrepresentative domain.** That is a distinct failure mode from the two this workspace has catalogued — it is not a wrong tolerance, and not a vacuous assertion. The test asserted the right thing about the wrong set of inputs.

Now split in two: `xi_is_bounded_everywhere` keeps the dense operating-range sweep, and `xi_is_bounded_across_the_entire_representable_domain` covers the range the law actually claims, with a dense pass straight through `[700, 760]`. A third, `both_algebraic_forms_of_xi_agree_where_both_are_valid`, pins the equivalence so a future "simplification" back to one branch has to argue with a test.

## Gate 3 refuses rather than defaults

`xi(r) → 0` as `r → ∞`, so far enough out both effective frequencies collapse and the detuning ratio is `0/0`. `resonates` returns `Err(UndefinedScale)`.

This is the fourth refusal-not-default in the kernel, and the reasoning is the same each time: an out-of-domain input has no answer, and inventing one hides the condition at exactly the point a caller could still handle it.

## Doctrine checks

| Sabotage | Failures |
|---|---|
| revert `xi` to the naive single expression | **5** (3 here, 2 in `symphony-lang`) |
| gate 3 ignores observation scale | **6** (3 here, 3 in lang) |
| widen the band from `1/8` to `1/4` | **4** (2 here, 2 in lang) |
| `invert` becomes a no-op | **7** (2 here, 5 in lang) |

**Not attempted: `invert` as `-φ`.** A2's orientations are symmetric about zero, so the two coincide numerically — not a mutation. Recorded rather than run, per the `opposes`-as-`else` lesson.

## Scope note

The gates live here because they are physics; the *syntax* for them is `symphony-lang`'s. `Phase::invert` and `resonates` know nothing about `when` or `invert` statements, and the language writes neither the band, nor `xi`, nor the `π` shift into its own crate.

Nothing about deadlock changed. Detection is still here, resolution still is not.

## Human check

Read `xi_is_bounded_across_the_entire_representable_domain` and the defect section above. A stated law invariant was violated in shipped code, and the test guarding it passed throughout — because it swept a domain where the bug does not appear.

---

# Addendum — resource tracker feeding `WaitForGraph`

Closes the bullet this log used to carry: "`WaitForGraph` detects cycles in waits that something else must record. Nothing yet acquires or releases resources." `resources::ResourceTracker` is that something.

`acquire(task, resource, graph)` grants immediately if the resource is free or already held by `task` (reentrant no-op); otherwise it queues `task` and calls `graph.add_wait(task, holder)` — the caller never computes who holds what. `release(task, resource, graph)` hands the resource straight to the next queued task, if any, and retargets every *remaining* queued task's edge from the departing holder to the new one, so the graph never shows a wait pointing at a task that no longer holds anything.

One invariant does the load-bearing work: a task may have **at most one outstanding wait edge** (`acquire` on a second, different resource while already blocked is refused as `ResourceError::AlreadyWaiting`). Every deadlock the contract itself names — two locks, opposite acquisition order — blocks one resource at a time, so this is not a restriction beyond what the law requires; it is what makes `release`'s retargeting loop exact (there is only ever one edge per waiter to move) instead of needing to disambiguate which of several edges is the stale one.

`WaitForGraph` gained `remove_wait(waiter, holder)` to support this — deletes exactly that one edge, unlike `clear_waits` which drops everything the waiter has recorded.

## A blind spot the doctrine check found, not one I noticed first

Sabotaging `remove_wait` into behaving like `clear_waits` (dropping *every* edge for the waiter instead of just the named one) passed **the entire suite, including all ten new resource-tracker tests, with zero failures.** The reason is the same invariant that makes the tracker's bookkeeping exact: since a task can never have more than one outstanding edge while going through `ResourceTracker`, "remove one edge" and "remove all edges" are indistinguishable on every path the tracker exercises. The gap wasn't in the tracker's logic — it was that nothing exercised `WaitForGraph::remove_wait` on its own, independent of the tracker's invariant. Closed with `remove_wait_drops_only_the_named_edge`, built directly against `WaitForGraph` with a waiter holding two edges (a shape the tracker itself never produces, deliberately, but the graph's own API contract has to hold regardless of who calls it).

## Doctrine checks — three performed

| Sabotage | Failures |
|---|---|
| `release`'s retargeting loop over remaining queued waiters removed | **1** (`releasing_retargets_remaining_waiters_to_the_new_holder`) |
| the idempotent same-resource repeat-acquire branch removed | **1** (`repeated_acquire_of_the_same_pending_resource_is_idempotent`) |
| `remove_wait` reverted to `clear_waits` behaviour | **0 through `ResourceTracker`** — see the blind-spot note above; **1** (`remove_wait_drops_only_the_named_edge`) once that dedicated test existed |

All reverted after confirming; workspace re-confirmed at 396/396, `python _system/status.py --check` clean (0 dead wikilinks, 0 broken links, 0 encoding faults).

## Human check

Read `two_tasks_contending_in_opposite_orders_deadlocks` (the dining-philosophers-shaped scope-boundary test, built entirely through `acquire` rather than hand-placed edges) and the blind-spot note above it. The note is the reminder that an invariant which makes code *correct* can simultaneously make a sabotage of unrelated code *invisible* — "zero failures" needed the same scrutiny here as anywhere else in this project's doctrine checks, not an automatic pass.

---

# Addendum — `ConcurrentPool`: the synchronisation decision `substrate` left open

Closes the item `substrate`'s own implementation log named explicitly: "Whether locks live here or in `symphony-kernel` is a scheduler decision, not a substrate one." `memory::ConcurrentPool` wraps `substrate::MemoryPool` in one coarse `Mutex`, exposing `allocate`/`free`/`write`/`read`/`available`/`cell_count`/`total_capacity` — every call takes the lock for its own duration and nothing else.

## Verified the lock is load-bearing before trusting it

A disposable scratch harness (outside the crate, deleted after use) wrapped a raw `MemoryPool` in `UnsafeCell` + a deliberately wrong `unsafe impl Sync`, giving 32 real threads concurrent `&mut` access with zero synchronisation, each thread writing its own fingerprint byte across many allocate/write/read/free cycles and checking the read-back matched. First attempt: every thread used the *same* fill byte, and 10/10 runs looked clean — for the wrong reason: an aliased allocation reads back "correctly" if what overwrote it happened to match. Fixed the harness to use a distinct fingerprint per thread; re-run: **10/10 unsynchronised runs corrupted**, **0/10 through a `Mutex`** on the identical workload.

## A genuine defect surfaced, entirely unrelated to concurrency in the end

Stress-testing the real `ConcurrentPool` against the new tests initially **failed** — `concurrent_allocations_never_corrupt_or_alias` caught real corruption even with the `Mutex` correctly held for every call. Traced it down with a sequential (zero-thread) reproduction before concluding anything: `substrate::MemoryPool::free` reset a cell's usage unconditionally, so freeing one of two allocations sharing a cell wiped out its still-live sibling's reservation too, letting a third allocation overlap live data. Nothing to do with locking — the `Mutex` was correctly serialising calls into a `MemoryPool` whose own `free()` had a real, pre-existing, purely single-threaded bug. Fixed in `substrate` (`Slab::live`, a reference count gating when `used` actually resets) — full account in that subsystem's own addendum. `ConcurrentPool` needed no changes at all once that landed; it was never the source of the corruption, only the first caller to exercise the input shape that found it.

## Doctrine checks — two performed, both against the real `ConcurrentPool`

| Sabotage | Result |
|---|---|
| `Mutex<MemoryPool>` replaced with `UnsafeCell` + `unsafe impl Sync` (zero synchronisation) | **`concurrent_allocations_never_corrupt_or_alias` failed** — real fingerprint mismatches, matching the scratch harness exactly |
| same sabotage, `concurrent_admission_never_oversubscribes_capacity` | **passed anyway** — not a gap in the test, a property of data races: fewer operations per thread (one `allocate` call each, no write/read/free cycle) gives the race a narrower window, and a race that doesn't manifest in one run is expected, not evidence of safety. Recorded honestly rather than treated as a clean pass. |

Reverted after confirming; workspace re-confirmed at 399/399, `python _system/status.py --check` clean.

## Human check

Read `concurrent_allocations_never_corrupt_or_alias` in `neos/tests/symphony_scheduler.rs`, then `freeing_one_allocation_does_not_free_a_sibling_sharing_its_cell` in `neos/tests/substrate.rs`. The first found the bug; the second is the same bug with every thread removed — proof it was never about concurrency, only found by it.

---

# Addendum — `ConcurrentTracker`: real blocking, not just real safety

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 453 passed; 0 failed
cargo test  -p symphony-kernel → 79 passed (38 symphony_kernel + 41 symphony_scheduler)
```

Workspace total: **448 → 453**. `symphony_kernel` unchanged at 38; `symphony_scheduler` 39 → 41.

Built to close a limit [[symphony-lang]]'s `vm::Vm` states about itself directly: a blocked `acquire` traps because "there is no scheduler able to suspend a blocked program and resume it once the holder releases." `ConcurrentPool` proved a `Mutex` can make an already-total operation safe to share; that proof doesn't extend to `acquire`, which can legitimately answer "not yet" — a `Mutex` around it changes nothing about what a caller does with that answer. `ConcurrentTracker` (`Mutex<(ResourceTracker, WaitForGraph)>` + `Condvar`) is the missing piece: `blocking_acquire` loops on `tracker.acquire` and, on `Blocked`, calls `Condvar::wait` — the calling OS thread genuinely sleeps. `release` updates the tracker then calls `notify_all`, so every blocked waiter wakes and re-checks; `ResourceTracker::acquire`'s own documented idempotence (a repeated ask while still blocked is a no-op) is exactly what makes a spurious wake harmless.

## Verified as a real wait — timed, not inferred

`blocking_acquire_really_suspends_the_calling_thread_until_released` holds a resource for a measured `Duration::from_millis(150)` on one thread, then times a second thread's `blocking_acquire` call for the same resource with `Instant::now()`. A sequential `Blocked`-then-return implementation (what `ResourceTracker` alone already gives) would return in microseconds regardless of when the release happens; this must take at least the hold duration, confirmed directly rather than assumed from the presence of a `Condvar` in the type.

## `resources_held_by` — the piece a generic resolver needs that a hand-built scenario never did

The existing sequential deadlock demo (`neos/src/main.rs`) resolves its two-lock inversion by hand-picking which one resource its victim holds, because it built the scenario itself and already knows. A resolver watching real threads it did not script cannot assume that — `ResourceTracker::resources_held_by(task)` (new: filters `holders` for matches) answers "what does this task actually hold right now," and `ConcurrentTracker::force_release_all` releases every one of them, notifying waiters after each.

## A real bug, found only because a second thread could keep running

The first version of the real two-lock-inversion test (`two_real_threads_deadlock_and_the_watchdog_resolves_it`, in `neos/tests/symphony_scheduler.rs`) hung indefinitely on its first run — caught by running it under a bounded external timeout rather than letting it block the whole suite. Root cause, traced directly rather than guessed: after the watchdog force-releases the victim's one held fork, the victim's own thread — which is still running, blocked only on its *next* instruction — eventually gets that next fork granted (once the other philosopher finishes and releases it), proceeds to its own cleanup, and calls `release` on the fork that was already taken from it. `ResourceTracker::release` correctly refuses with `NotHolder`; the test's own `.unwrap()` turned that refusal into a panic mid-thread, which stalled `.join()` forever.

This is not a `ConcurrentTracker` defect — `NotHolder` is exactly the right answer to "release something you don't hold," the same guard the sequential `ResourceTracker` has always had. It is a fact about real concurrency a single-thread scenario cannot produce: there, once a resource is force-released, nothing owning that "victim" ever runs again to ask for it back, because there is no separate thread to keep running. Fixed by writing each philosopher to tolerate `NotHolder` on its own release calls (`let _ = tracker.release(...)`) — the shape of a task actually built to survive preemption, not a change to the tracker.

## Doctrine checks — two performed

| Sabotage | Result |
|---|---|
| Condvar wait removed from `blocking_acquire` (`Blocked` treated as `Granted`) | **Failed three ways across two crates**: the timing test returned in microseconds; the real-threads deadlock test recorded no cycle at all (nothing ever actually waited, so no edge was ever added); `symphony-lang`'s own mutual-exclusion test observed real cross-thread data corruption |
| `resources_held_by` returns nothing | **1 failed** — the deadlock test's own `released must not be empty` assertion, caught immediately, before either thread could be joined (no hang) |

Both reverted after confirming; full suite re-confirmed at 79/79, 453/453 workspace-wide.

## Wired into the demo

`neos/src/main.rs` gained `symphony-kernel: the same deadlock, for real` — the identical two-lock-inversion scenario the sequential section above already demonstrates, rebuilt from two real OS threads via `ConcurrentTracker` directly, with a watchdog polling `detect_cycle` while both threads are genuinely, concurrently blocked. Confirmed stable across repeated manual runs.

## Human check

Read the hang-and-fix account above, then compare `two_tasks_contending_in_opposite_orders_deadlocks` (the existing sequential test, just above this addendum) against `two_real_threads_deadlock_and_the_watchdog_resolves_it` in `neos/tests/symphony_scheduler.rs` side by side. Both build the identical scenario; only the real-threads version could have surfaced the preemption bug, because only it has a second thread still running after the victim's resource is taken.
