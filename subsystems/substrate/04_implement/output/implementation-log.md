---
type: implementation-log
subsystem: substrate
stage: 04_implement
status: complete
toolchain: rustc 1.97.1 / cargo 1.97.1
result: 43 passed, 0 failed (412 workspace-wide) — see free()/sub-cell reuse/fault-trapping addenda
consumes: [lattice]
---

# Substrate — Implementation Log

## Result

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 114 passed; 0 failed
                           24 lattice metric · 14 tiling · 24 substrate
                           26 kernel · 26 scheduler
cargo run -p substrate   → boots, translates, allocates, splits
```

Live output:

```
NEOS substrate
  carrier        6.283185e9 rad/s (angular)
  quarter period 2.500000e-10 s
  memory         31 cells x 4096 bytes = 126976 total
  translation    "NEOS" -> 32 phases -> "NEOS" (round trip ok)
  allocation     8192 bytes across 2 adjacent cells
  extent         1
  after split    2  (axiom A1: 1 (x) 1 = 2)
  uptime         1.000000e-9 s after 4 ticks
```

## The flat/curved boundary — verified by compiler, not by test

Contract §2, the decision this subsystem existed to make. Settled as: **the public API of `memory` is the boundary.**

`FlatOffset` is private, and it is the return type of the private `MemoryPool::resolve` — the single function performing curved-to-flat translation. Every public read and write routes through it. Because the type cannot leave the module, neither can the translation's result.

A probe was written attempting four separate leaks. All four were refused:

```
error[E0603]: struct `FlatOffset` is private
error[E0599]: no method named `as_ptr` found for struct `MemoryPool`
error[E0599]: no method named `as_slice` found for struct `MemoryPool`
error[E0624]: method `resolve` is private
```

The probe was removed after verification. `ftg` cannot obtain a linear address even by accident — which is the point, because a consumer that *could* would eventually do arithmetic on it and be working in Euclidean space regardless of what the geometry layer claims.

**A correction during the build.** The first version declared `FlatOffset` but never constructed it — the compiler said so. It was a comment wearing a type: the guarantee held only because no public API happened to expose `usize`. Giving it the `resolve` job made the boundary structural rather than symbolic.

## The zero-crossing hazard

The most consequential finding of `01_derive`, and it came out of arithmetic rather than the corpus.

`cos(x + π/2) = −sin(x)` and `cos(x − π/2) = +sin(x)`, so the two bit states differ **only in the sign of the sine component**:

| `ω_c t` | bit 0 | bit 1 | separation |
|---|---|---|---|
| 0 | 0.000 | 0.000 | **0.000** |
| π/2 | +1.000 | −1.000 | **2.000** |
| π | 0.000 | 0.000 | **0.000** |

At `t = 0` and every half period the carrier holds **no information at all**. `demodulate` returns `Err(ZeroCrossing)` there rather than producing bits, because producing them would be fabricating them.

This is the kind of defect that ships. It would have surfaced as intermittent bit corruption at every layer above, with no local cause and nothing in the units or types to hint at it. `Hypervisor::tick` therefore advances by a **quarter period** — the clock and the demodulator agree by construction rather than by a caller remembering.

## Frequency types consolidated

`Frequency` and `AngularFrequency` moved from `symphony-kernel` to `substrate`, which is the lowest layer that uses them (its clock is `ω_c`). `symphony-kernel` now depends on `substrate` and re-exports.

Dependency direction is now `lattice ← substrate ← symphony-kernel`, matching the PRD's tiering: Symphony runs *on* the Substrate.

Two copies of a type this load-bearing would have drifted, and the entire value of the separation is that the compiler catches `ν`/`ω` confusion. Test 5.5 asserts the consolidation directly by assigning a `symphony_kernel::Frequency` to a `substrate::Frequency` — two distinct types would not compile.

## Doctrine check

| Sabotage | Result |
|---|---|
| `distance` replaced by flat index difference | **2 failed** — including the cross-check against `lattice`'s own metric |

Reverted, suite re-confirmed at 114/114, no markers.

## Allocation locality

`allocate` grows breadth-first over `Cell::neighbors()`, so a multi-cell allocation occupies **adjacent** cells rather than consecutive indices. Test 2.1 asserts every cell after the first touches an earlier one.

This is what `ftg` depends on: addresses near in the metric are near in the allocation, so routing by hyperbolic distance is meaningful rather than decorative. A flat allocator would satisfy the type signature and break the property silently.

## An encoding rule, learned twice

PowerShell's `Set-Content` added a BOM and mojibake'd em-dashes into `tests/substrate.rs`, exactly as it had into `constants.json` in the previous slice. Repaired with Python and normalised to pure ASCII.

**Rule going forward: never write source or JSON through PowerShell redirection.** Use the editor tools, or Python when a bulk transform is needed.

## What is not built

- **Concurrent allocation.** `MemoryPool` itself stays single-threaded, deliberately — synchronisation was left as "a scheduler decision, not a substrate one." That decision is made: `symphony-kernel::ConcurrentPool` wraps a `MemoryPool` in one coarse `Mutex`. See this log's own free() addendum for the real, single-threaded correctness bug that verifying the concurrent wrapper surfaced here.
- **`MemoryPool` reporting size via `lattice::addressing::LogicalArea`.** `lattice` built area preservation and proved fragmentation structurally zero (`fragmentation_is_exactly_zero`, `scaling_adds_cells_without_resizing_them`) in its own addressing slice, after this log was written — this bullet is now about consumption, not absence. `MemoryPool` still reports its own `cell_count`/`total_capacity` directly rather than through `LogicalArea`; nothing is wrong with that, it just means the two haven't been joined.

## Human check

Run `cargo run -p substrate` and read the output. Then read tests 4.3 and 2.1 — the first stops silent bit corruption at every layer above, the second is what makes `ftg`'s hyperbolic routing meaningful.

---

# Addendum — `MemoryPool` wired to `LogicalArea`

Closes the bullet this section used to carry: `lattice::addressing::LogicalArea` proved fragmentation structurally zero, but only against a bare cell count handed to `LogicalArea::of` from nowhere — never against a real pool's real allocation lifecycle.

`Allocation::logical_area()` reads `self.cells.len()` — already-tracked state, nothing new stored — into a `LogicalArea`. `MemoryPool::total_area`/`occupied_area`/`available_area` do the same over `self.slabs`, classifying a cell as occupied the moment it holds *any* bytes: `LogicalArea` has no fractional-cell concept, which is exactly this pool's own zero-fragmentation invariant restated — there is no in-between state to represent. All four methods are pure derived reads; no new field, no state that could drift from `self.slabs`.

## A blind spot in my own test design, caught by sabotage rather than noticed first

Every other test in this addendum allocates in exact multiples of `cell_capacity`, deliberately — a partial allocation can share a cell's *byte* capacity with another allocation, which would make "whose cell is this, geometrically" ambiguous. That choice quietly meant every occupied cell in those tests was also completely full, so a sabotage that redefined "occupied" as "completely full" instead of "holds any bytes" passed cleanly through all of them — caught only once a dedicated partial-allocation test (`a_partially_used_cell_counts_as_fully_occupied`, 30 of 64 bytes in one cell) was added specifically to separate the two definitions. The methodological point: avoiding one ambiguity (shared cells) silently manufactured coverage that couldn't see a different, real bug — worth naming since it's the kind of gap that would otherwise ship quietly.

## Area conservation holds only up to floating point, verified before relying on it

`occupied.area() + available.area() == total.area()` is **not** bit-exact in general: `(a+b)*c != a*c+b*c` for arbitrary integer cell-count splits in `f64`. Measured directly (outside the crate, before writing any assertion): worst absolute gap `~1.8e-12` on a several-thousand-cell split, which is `~1.5e-16` relative — the `f64` epsilon floor, not a real divergence. `area_is_conserved_across_allocation_and_freeing` uses a relative tolerance (`1e-9`) for exactly this reason, on a 441-cell pool large enough to actually exercise the effect, rather than an exact-equality assertion that would have been fragile for a reason that has nothing to do with correctness.

## Doctrine checks — three performed

| Sabotage | Failures |
|---|---|
| `occupied_area` requires a cell to be *completely* full | **0 of 36** on first pass — see the blind-spot note above; **1 of 36** after adding the missing partial-cell test |
| `available_area` returns the total cell count instead of only untouched cells | **4 of 36** |
| `Allocation::logical_area` reads byte length instead of cell count | **1 of 36** |

## Human check

Read `a_partially_used_cell_counts_as_fully_occupied` and the blind-spot note above it. The test itself is short; what it is doing is closing a hole that three *other*, already-passing tests in this same addendum could not see, because they were all built to dodge a different problem.

---

# Addendum — `free()` freed too much when a cell was shared

Closes a real, previously-invisible defect, found while building and verifying `symphony-kernel::ConcurrentPool` — but the bug itself has nothing to do with concurrency. `free` reset a cell's `used` byte count to zero unconditionally:

```rust
pub fn free(&mut self, alloc: &Allocation) {
    for cell in &alloc.cells {
        if let Some(s) = self.slabs.get_mut(cell) { s.used = 0; }
    }
}
```

If two separately-tracked allocations shared a cell (one only partially filling it, leaving room the next allocation bump-appended into), freeing the *first* of them reset the *whole* cell to unused — including the second allocation's still-live bytes. A subsequent `allocate` would then be handed exactly that live range. Reproduced deterministically outside the crate, with zero threads, before touching any code:

```
pool = MemoryPool::new(1, 256)      // one cell, room for four 64-byte spans
a = pool.allocate(64)  // offset 0
b = pool.allocate(64)  // offset 64
write a <- [1;64], write b <- [2;64]
pool.free(&b)
pool.available()  // 256 -- wrong: only b's 64 bytes should be back, a is still alive
c = pool.allocate(64)  // lands at offset 0 -- directly on top of a
write c <- [3;64]
read(a.start(), 64)  // [3;64] -- a's data, silently gone
```

## Why nothing caught this earlier

Every existing test exercising `free` allocates in exact multiples of `cell_capacity` — the same discipline the `LogicalArea` addendum above already names, there to dodge a *different* ambiguity (which allocation "owns" a shared cell geometrically). That choice had the same side effect twice over: it also meant no cell in any existing test was ever shared between two separately-freed, still-both-alive allocations, so the free-too-much path was never exercised. This is the third time in this subsystem's own history that avoiding one ambiguity has silently manufactured a blind spot for a different, real bug — worth naming as a pattern, not just a one-off.

## The fix, and its stated cost

`Slab` gained a `live: usize` field — a count of allocations currently touching that cell. `allocate` increments it once per cell an allocation touches; `free` decrements it, and only resets `used` to zero once `live` reaches zero. The tradeoff is stated rather than hidden: while any allocation sharing a cell remains live, a freed sibling's bytes are not reclaimed for reuse *in that cell* — internal fragmentation until the cell empties entirely, in exchange for never handing the same bytes to two owners at once. Given `allocate`'s own bump-forward growth (never reuses a hole below the high-water mark, only ever grows it), this fix costs nothing beyond what that existing model already accepted; it doesn't make the allocator worse, it makes `free` correctly match the guarantee `allocate` was already making.

## Doctrine check

| Sabotage | Result |
|---|---|
| Revert to unconditional `s.used = 0`, run the new regression test | **1 of 37 failed** — `freeing_one_allocation_does_not_free_a_sibling_sharing_its_cell`, for exactly the reason above |

All 36 pre-existing tests were re-run against the fix and pass unchanged — confirmed before, not just claimed: every one of them frees a cell with exactly one live allocation on it, where `live` reaching zero on the first (only) `free` call is behaviourally identical to the old unconditional reset.

## Human check

Read `freeing_one_allocation_does_not_free_a_sibling_sharing_its_cell` in `neos/tests/substrate.rs`. It needs no threads, no `Mutex`, no `ConcurrentPool` — the bug it guards against was always reachable from ordinary single-threaded use; concurrency just happened to be what finally exercised the input shape that finds it.

---

# Addendum — sub-cell allocation reuse

Closes the item this log's own "What is not built" section carried since the subsystem was first built: "Allocation reuse below cell granularity. `free` releases whole cells; a partially-used cell is not repacked." The `Slab::live` refcount from the addendum above was the safe-but-pessimistic first step; this generalises it into a real allocator.

## From a count to an interval set

`Slab::live` changed shape from `usize` (a count) to `Vec<(usize, usize)>` (live byte spans, sorted by offset, non-overlapping). Three new operations:

- `used()` — sum of span lengths, replacing the bare counter.
- `first_gap(capacity)` — the first free gap in offset order, `None` if the cell is full. First-fit, not best-fit: an allocation takes from at most one gap per cell it touches, since `write`/`read` each address one contiguous range per cell (`resolve`'s own contract), so a single allocation can never occupy two disjoint spans in the same cell.
- `insert_live`/`remove_live` — keep the sorted invariant on grant and release.

`Allocation` gained a private `spans: Vec<(CellId, usize, usize)>` — exactly which byte range it was granted in each cell it touches, so `free` can release precisely that, regardless of what else lives in those cells. `allocate`'s BFS growth loop is otherwise unchanged in shape: seed on the first cell with any free gap, walk lattice-adjacent neighbours, take what each cell's own gap offers, move on until the request is satisfied.

## What this actually buys

The prior refcount fix was safe but wasteful: freeing one of several allocations sharing a cell reclaimed nothing until every one of them was gone. Now a freed span is real free space the instant `free` returns — `first_gap` will offer it to the very next `allocate` call, including a hole opened in the *middle* of a cell, ahead of untouched space further out. Observed, not rigorously benchmarked, in the `neos` demo's own concurrent-allocation section (8 threads, a 4-cell/1024-byte pool, real contention): completed allocate/write/read/free cycles per run sat in the low 700s-800s before this change and around 800 after a handful of runs — consistent with the mechanism (allocations that would previously have stalled on "wait for the whole cell to empty" now landing sooner), but not a controlled measurement, and stated as such rather than oversold as one.

## Doctrine checks — two performed

| Sabotage | Result |
|---|---|
| `remove_live` reverted to clearing every span in the cell (regression to the original bug this whole line of fixes closes) | **2 of 38 failed** — `freeing_one_allocation_does_not_free_a_sibling_sharing_its_cell` and `freeing_a_middle_allocation_creates_a_reusable_gap` |
| `first_gap` changed to return the *last* gap instead of the first (worst-fit instead of first-fit) | **1 of 38 failed** — `freeing_a_middle_allocation_creates_a_reusable_gap`: the new allocation landed in the untouched tail (offset 192) instead of the freed middle gap (offset 64) |

Both reverted after confirming; full suite re-run clean (38/38 substrate, 400/400 workspace-wide), `python _system/status.py --check` clean.

## Human check

Read `freeing_a_middle_allocation_creates_a_reusable_gap`. It's the test that actually distinguishes "sub-cell reuse" from "the refcount fix, but the assertion is looser" — a gap that isn't the most recently freed one, and isn't at either edge of the cell, still has to be found and reused ahead of unrelated free space.

---

# Addendum — fault trapping (`Hypervisor::allocate_trapped`)

Closes the "Virtualisation proper" bullet this log carried since the subsystem was first built — but not as originally worded. Before touching any code, checked whether there was law to build on: `_mkb/` (every file — `axioms.md`, `equations.md`, `gates.md`, `resonance.md`, `constants.md`, `timecrystal.md`, `reconciliation.md`, `test-doctrine.md`, `operators.md`, `tetryen.md`, the papers index) has **zero** mentions of "virtual," "privilege," "trap," "guest," or "isolation." Unlike `timecrystal.md` or `gates.md`, which were real syntheses composed from law fragments that did exist (Howard Comma + Tetryen geometry; A2 + teardown phase + standing-wave band), there was nothing here to compose — the PRD phrase has no physics under it.

Worse for the literal ask: there is no guest execution engine anywhere in the workspace for isolation or privilege levels to apply to. `symphony-lang`'s grammar is `task | branch | fork | invert | emit` — no arithmetic, no variables, no memory access, no general loops, no function calls — and its own module docs state this is a deliberate scope boundary, not an oversight. Building real trapping/isolation/privilege levels as specified would mean inventing a general-purpose instruction-executing language first (reversing a documented design decision) and then inventing privilege semantics with nothing to verify them against — not a slice, a new subsystem with no law to ground it.

## The one piece with real substance

`SubstrateError`'s variants already read as fault codes — `Exhausted`, `Unmapped`, `OffsetOutOfCell`, `SplitDomain`, and so on — but nothing dispatched them anywhere; they were ordinary `Result`s returned to the direct Rust caller. `Hypervisor::allocate_trapped(bytes, max_retries, handler)` makes that dispatch real:

- The handler is called on **every** fault, unconditionally — a genuine transfer of control, not a conditional notification.
- It receives `&mut MemoryPool` directly, the same pool the failing allocation is against, so it can take a real corrective action (free an allocation it knows about) rather than merely simulate one via closure-captured state disconnected from the actual resource.
- `max_retries` bounds how many times the operation is retried afterward — a second guard, the same shape as `ftg::Router::route`'s own `max_hops`, so a handler that never actually resolves anything cannot hang the caller.

Deliberately scoped to `allocate` only. `Exhausted` has a real corrective action a handler can take (free something, retry — the same shape as a page fault a real OS resolves by evicting a page). `Unmapped`/`OffsetOutOfCell` from `read`/`write` are wrong-argument errors from the caller's own logic; no handler acting on the pool can fix those, so trapping them would add an interface with nothing real behind it.

## Verification: real recovery, not a demonstration of one

`handler_can_recover_by_freeing_and_retrying` exhausts a 1-cell pool, then calls `allocate_trapped` with a handler that frees the earlier (real, live) allocation on the first fault and asks to retry. The retried allocation actually succeeds — asserted directly, not assumed. Four more tests cover the rest of the contract: the handler sees every fault exactly once when it declines (`declining_the_fault_propagates_without_retrying`), is never invoked at all on a successful allocation (`a_successful_allocation_never_touches_the_handler`), and — the safety-critical one — a handler that always asks to retry without ever actually resolving anything is still bounded, called exactly `max_retries + 1` times before the caller gets its `Err` back (`retries_are_bounded_even_when_the_handler_never_gives_up`).

## Doctrine checks — two performed

| Sabotage | Result |
|---|---|
| Retry bound changed from `attempts >= max_retries` to `attempts > max_retries` (off-by-one, one extra retry allowed) | **1 of 43 failed** — `retries_are_bounded_even_when_the_handler_never_gives_up` reported 5 calls where exactly 4 were expected. Still terminated (the guard was off by one, not absent) — the exact discriminator this test exists to catch. |
| The handler's returned `TrapAction` ignored entirely (loop always retries until `max_retries`, regardless of `Propagate`) | **1 of 43 failed** — `declining_the_fault_propagates_without_retrying` ran to that test's own retry bound instead of stopping on the first `Propagate`. Still terminated safely, bounded by the same `max_retries` guard — confirming the two safeguards are independent and either alone still prevents a hang. |

Both reverted after confirming; full workspace re-run clean at 412/412.

## Human check

Read `handler_can_recover_by_freeing_and_retrying`. It's the test that separates a real trap path from a decorative one: the handler doesn't just get *told* about the fault, it gets a `&mut MemoryPool` and the retried operation has to actually succeed against real pool state, not a mock.
