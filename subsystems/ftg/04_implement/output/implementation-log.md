---
type: implementation-log
subsystem: ftg
stage: 04_implement
status: complete
toolchain: rustc 1.97.1 / cargo 1.97.1
result: 61 passed across ftg's own suites (373 workspace-wide) — see Slice 5 addendum
consumes: [lattice, substrate]
---

# FTG — Implementation Log

## Result

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 143 passed; 0 failed
                           38 lattice · 24 substrate · 52 symphony-kernel · 29 ftg
```

Tests were written before the implementation. **Test Case 1 — the last unimplemented canonical doctrine test — is now green.**

## Files written

| Path | Role |
|---|---|
| `neos/ftg/Cargo.toml`, `build.rs` | manifest; constants from `_mkb/constants.json` |
| `neos/ftg/src/lib.rs` | crate root, `FtgError` |
| `neos/ftg/src/layers_1_2.rs` | framing, dissonance validation |
| `neos/ftg/src/layers_3_4.rs` | address mapping, metric-descent routing, port overtones |
| `neos/ftg/src/session.rs` | handshake, teardown, `cancellation_floor` |
| `neos/tests/ftg.rs` | 29 assertions |

## The sabotage gate found a false claim in my own contract

Three sabotages were run. The results are worth recording individually, because one of them **failed to break anything** — and that was the informative one.

| Sabotage | Tests failed | What it revealed |
|---|---|---|
| `min_by` → any closer neighbour | **0** | the claim was wrong (below) |
| descent no longer strict | 1 | strictness is what matters |
| dissonance check removed from `decode` | 1 | validation is load-bearing |

### `min_by` is not load-bearing, and the contract said it was

`01_derive` §3.2 said "forward to the neighbour **minimising** hyperbolic distance." Relaxing that to *any* closer neighbour broke nothing at all.

Measured why: **42% of steps have more than one strictly-descending neighbour**, yet deliberately taking the **worst** of them still produced a shortest path in **0 of 1497 routes**. Any strict descent is optimal on this tiling. `min_by` buys determinism, not correctness.

That is a stronger and more surprising property than the one originally claimed, so it is now asserted directly: `any_strict_descent_is_also_optimal` walks the worst-choice path and requires it to match BFS. It also asserts that branching genuinely occurs (~16% of steps along that path), so it cannot pass vacuously.

The real invariant is the **strictness** of the descent — sabotaging that *is* caught, by the stranded-packet test.

A sabotage that fails to break anything is not a wasted check. It is the only way to discover that a test was asserting a property the code did not depend on.

## A tolerance that could not be a constant

`03_tests` specified a cancellation floor of `1.2e-16`, taken from one measurement. The implementation failed it: `5.55e-16` at a larger `t`, and a peak of `1.08e-14` sweeping 40 periods.

The cause is arithmetic, not physics. `cos(x)` and `cos(x + π)` are exact negatives analytically, but **`x + π` rounds**, and that absolute error grows with `|x|`. Since `|d(cos)/dx| ≤ 1` it transfers to the result roughly one-for-one:

```
residual ~ ε · |ω·t|
```

The tolerance is now a function — `ftg::cancellation_floor(ω, t)` — living in the library so the test and any caller share one derivation rather than duplicating a magic number. A dedicated test asserts the floor **scales**, so replacing it with a constant fails loudly.

This is what the doctrine's floating-point caveat is for: a value picked from a single sample looks justified and is not.

## Layer 1/2 — validation by interference, no CRC

A frame is its payload phases followed by their complements. Since `sin(±π/2)` is exactly `±1`, a clean frame cancels **exactly**:

| Frame state | Dissonance |
|---|---|
| clean | `0.0` exactly |
| any single symbol flipped | `2.0` exactly — verified at **every** position, not sampled |

`decode` refuses a dissonant frame and there is deliberately **no lossy variant** — the contract says dissipate, never repair, and offering one would invite callers around the check.

### The blind spot is asserted, not hidden

`correlated_flip_of_a_symbol_and_its_partner_is_undetected` verifies that flipping a symbol *and its complement* returns dissonance `0.0`, reports clean, and yields a corrupted payload.

The method is named `is_clean()` rather than `is_valid()` for this reason. Interference checking measures net amplitude, so it cannot separate "no error" from "errors that cancel" — the same blind spot parity has. Asserting the limitation keeps it from being quietly rounded up to "detects corruption."

## Layer 3/4 and §7

**No routing table exists as a field.** `Router` holds the tiling; `next_hop` is a pure function of the metric. `bfs_hops` exists only so tests can compare against optimal — a BFS in the forwarding path would be exactly the table the contract forbids.

**Ports are orthogonal.** Distinct port channels have inner products `~1e-17` over a fundamental period; self-overlap is exactly `0.5`. The self-overlap test matters: without it, the orthogonality zero could just mean a broken integrator.

**The address→cell map is a stated assumption**, not derived law. FNV-1a over the address, indexed into the patch. Deterministic and total, which is what routing needs. It does **not** preserve locality, and the code says so rather than implying otherwise.

**A collapsed link is terminal.** Reuse would mean a connection surviving amplitude zero.

## Consumed, not reimplemented

The metric comes from `lattice`; bit↔phase and carrier synthesis from `substrate`. Both are cross-checked in tests (6.1, 6.2) rather than assumed. `grep` confirms no MKB constant literal in `ftg` source.

`build.rs` asserts `teardown_phase_shift` is exactly `π` and fails the build otherwise — a drifted value would make cancellation partial and surface as a mysterious tolerance failure in Test Case 1 rather than as a clear cause.

## What is not built

- **Socket I/O.** This is the translation and routing layer, not a driver. Nothing binds a NIC.
- **Fragmentation and reassembly.** A frame is one unit; MTU handling needs a transport policy the PRD does not give.
- **Retransmission.** None by design — a dissonant frame dissipates. Whether a higher layer re-sends is that layer's concern.
- **Multi-hop delivery of actual frames.** Routing computes paths; nothing yet carries a `Frame` along one. That is the natural next slice and would be the first end-to-end test.
- **§8 crystallisation**, which is its own record.

## Human check

Read `any_strict_descent_is_also_optimal` and `correlated_flip_of_a_symbol_and_its_partner_is_undetected`. The first asserts a property stronger than the contract originally claimed — found only because a sabotage failed to break anything. The second asserts a **limitation**, so the frame check's blind spot stays visible.

---

# Slice 2 — end-to-end transport

Added `neos/ftg/src/transport.rs` and `neos/tests/ftg_transport.rs` (13 assertions). Workspace total: **182 passing**.

Closes the gap recorded at the end of slice 1: *"nothing yet carries a `Frame` along a computed route."*

## The first assertions that cross subsystem boundaries

Every prior test exercised one crate. These exercise three at once:

- **`lattice`** — the hyperbolic metric chooses the path
- **`substrate`** — transduction carries the wave across each hop
- **`ftg`** — framing decides whether it survives

A hop is a real transmission, not a pointer move: the frame is synthesised onto the carrier and demodulated at the far end. A frame that cannot be recovered does not continue.

## Corruption now has a location

The PRD says corrupted frames "collapse into dissonance and are naturally dissipated." Slice 1 could only show that a corrupted frame *fails to decode*. Transport shows it **dissipates at the hop where the fault occurred** — `Delivery::Dissipated` reports the cell, the hop index, and the net amplitude.

`corruption_dissipates_at_the_hop_where_it_occurs` injects a fault before hops 0, 1, and 2 in turn and asserts the packet dies at exactly that hop, at the cell the route says it should be at.

Corruption enters through a `medium` closure rather than being generated internally. That matters: `a_quiet_medium_delivers_intact` confirms the injector is the *only* source of damage, so the dissipation tests measure the medium rather than a gateway bug.

## The blind spot, now visible end-to-end

`correlated_corruption_survives_the_route_undetected` flips a symbol **and its complement partner** in flight. The frame cancels to zero dissonance, reports clean, traverses the whole route, and **arrives with a corrupted payload**.

Slice 1 asserted this limitation in a unit test. Asserting it end-to-end matters more: it is the difference between "the checker has a known blind spot" and "a corrupted message can reach a destination and be accepted."

## Two calibration errors of mine

Both new tests failed first on my own thresholds, not on the transport:

- `delivery_is_hop_optimal` sampled 29 pairs where I demanded >50
- `transduction_is_lossless_over_the_longest_route` found a 4-hop longest route where I demanded ≥5

A depth-4 patch is 166 cells and reaches only 4 hops from the origin. Fixed by using a depth-5 patch (441 cells, 5 hops, ~180 pairs) for those two — which makes them **stronger**, not weaker. A `deep_gateway()` helper documents why the deeper patch is needed.

## Doctrine checks — two performed

| Sabotage | Tests failed | What it showed |
|---|---|---|
| carry corruption to the destination instead of dissipating | 2 | dissipation-at-hop is load-bearing |
| sample every hop at a carrier zero crossing | **9 of 13** | the `substrate` hazard is real in transit |

The second is the more striking result. `substrate` found that both bit states read as exactly zero at a zero crossing; this shows what that means for a working system — **nine of thirteen end-to-end behaviours collapse**, including plain delivery of an uncorrupted payload. `every_hop_samples_at_a_safe_instant` survived, correctly: it checks the instants directly rather than delivery.

## Still not built

- **Session-gated delivery.** `Link` exists and `Gateway` exists, but delivery does not yet require an established link, nor does teardown interrupt a transfer.
- **Fragmentation, retransmission, socket I/O** — unchanged from slice 1, and retransmission remains absent *by design*.
- **Backpressure or queueing.** Delivery is synchronous and one packet at a time.

## Human check

Read `corruption_dissipates_at_the_hop_where_it_occurs` and `transduction_is_lossless_over_the_longest_route`. The first is the PRD's "naturally dissipated" claim made concrete and located; the second proves that five successive carrier transductions leave a 256-byte payload bit-identical.

---

# Slice 3 — session-gated delivery

Added `Gateway::deliver_over` / `deliver_over_with`, `Delivery::LinkLost`, `Link::drift_to`, and `neos/tests/ftg_session_transport.rs` (11 assertions). Workspace total: **193 passing**.

Closes the gap recorded at the end of slice 2: *"delivery does not require an established link, nor does teardown interrupt a transfer."* This is where §6 transport finally meets §7's connection lifecycle.

## Two failure modes, deliberately kept apart

Transport now distinguishes:

| Outcome | Meaning |
|---|---|
| `Dissipated` | the **frame** collapsed into dissonance |
| `LinkLost` | the **session** collapsed; the frame is intact, the carrier is gone |

Collapsing these into one "failed" would lose the distinction that matters: a dissipated frame was corrupted in transit, while a lost link means the connection ended underneath a healthy packet. `dissipation_and_link_loss_are_distinguishable` asserts they are different variants, and `a_stranded_frame_is_still_intact` proves the second claim by re-sending the *same* packet over a fresh session and having it arrive untouched.

`LinkLost` carries `carrier_amplitude`, measured at the moment of loss, so a caller can see the medium really did vanish rather than trusting the label. Asserted `< 1e-9`.

## The link is re-checked every hop, not at admission

This is the whole slice. §7 says a connection is a shared standing wave, not a state record — so a session can collapse *mid-transfer*, and a packet already in flight must stop where the carrier stopped.

`link_is_rechecked_at_every_hop` collapses the session on the **final** hop and requires the packet to strand anyway. An admission-only gate would deliver it.

## Doctrine check

| Sabotage | Tests failed |
|---|---|
| gate at admission only, drop the per-hop recheck | **5 of 11** |

The four admission tests survived, correctly — they only exercise the gate at entry. That the mid-transfer tests are the ones that fell is the evidence the recheck is load-bearing rather than defensive.

## Detection stays separate from resolution

`Link::drift_to` lets the far oscillator drift out of resonance, and **does not tear the link down on its own**. `still_locked()` reports the loss; the caller decides.

`equations.md` mandates automatic teardown once variance exceeds `π/4`, but that policy belongs to whatever drives the link, not to the measurement — the same separation `symphony-kernel` keeps between deadlock detection and resolution.

## The ungated path is unchanged

`deliver` still works without a session. Session gating is an added guarantee on a new method, not a change to slice 2's behaviour — `ungated_delivery_is_unaffected` pins that, and it survived the sabotage.

## Still not built

- **Automatic teardown on drift.** Detection exists; the policy that acts on it does not.
- **Retransmission after `LinkLost`.** The frame is provably intact and a fresh session delivers it, but nothing retries automatically — consistent with there being no retransmission by design.
- **Concurrent sessions.** One link, one packet, synchronously.

## Human check

Read `teardown_mid_transfer_strands_the_packet` and `a_stranded_frame_is_still_intact`. Together they make "a connection is a standing wave, not a record" operational: the packet stops when the wave stops, and the frame it was carrying is demonstrably undamaged.

---

# Slice 4 — Multi-Hop Frame Propagation at Scale

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 368 passed; 0 failed
```

Workspace total: **365 → 368**. The propagation pipeline (`Gateway::deliver_through`/`deliver_over_with`, walking `path.windows(2)` hop by hop) was already built in slices 2 and 3. This slice is what it had not yet been verified against: its own documented scale.

## The gap was between two already-true claims, not inside either one

`layers_3_4.rs`'s own module docs state *"measured 4000/4000 arrivals over a 441-cell patch, and BFS-optimal on every sampled route."* `greedy_descent_is_shortest_path` backs that at real scale — routing is genuinely well-tested. Separately, every wave-level transport test (`payload_survives_a_multi_hop_route`, the session-gated equivalents) used `min_hops` of 3 or 4.

Nobody had checked whether those two facts actually met — whether the **wave-level** pipeline, not just the router, survives the hop counts routing is proven to produce. Measured before writing anything: the true diameter of the documented 441-cell patch (`Router::new(5)`) is **10 hops**, not the 3-4 every transport test exercised. `diameter_of_the_documented_patch_is_measured` (`ftg.rs`) pins this so the number stays live rather than becoming another unverified doc comment.

## What closes it

- `farthest_pair_delivers_byte_exact_at_the_true_diameter` (`ftg_transport.rs`) — a real farthest-pair search on the 441-cell patch, delivered via `deliver_through` with a clean medium, asserting both `Delivery::Arrived` and byte-exact payload at 10 real hops.
- `undisturbed_session_survives_the_true_diameter` (`ftg_session_transport.rs`) — the session-gated counterpart. No existing session test went past 4 hops; this is the same route via `deliver_over`, confirming a link that starts locked and is never perturbed carries a packet exactly as far as the ungated path does.
- `delivery_is_hop_optimal` strengthened to assert `Delivery::arrived()` explicitly, not just `.hops()` equality — `.hops()` is defined on every `Delivery` variant including the two failure ones, so a coincidental dissipation at the optimal hop count would have passed the old assertion without the packet ever arriving. A defensive fix rather than one demonstrated by a live bug: I could not construct a mutation of the current, correct code that actually triggers this path, and say so rather than manufacture a sabotage that proves nothing.

## A hypothesis that turned out wrong, and why

Before measuring, the reasonable-sounding worry was that `safe_sample_instant(hop as u32)` might lose floating-point precision at high hop counts — `sin`/`cos` of a large argument is a real, well-known hazard once argument reduction dominates. Checked directly: `depth=7` (3046 cells) has a longest-from-origin path of only **7 hops**; the documented 441-cell patch's true diameter is 10. Ring `n` of a `{5,4}` tiling holds `5·Fib(2n)` cells, so cell count grows exponentially while hop count from the centre grows only logarithmically — reaching thousands of cells never requires more than a handful of hops. `omega_c · t` at hop 10 is nowhere near where trig precision degrades. The worry was reasonable; the geometry that makes hyperbolic routing worth having is exactly what makes it groundless here.

## A mutation that revealed the round-trip is a genuine no-op, not a gap

Sabotaged `deliver_through`'s hop loop to skip reassigning `frame` from the demodulated bits — a real code change, and the module's own docs call the demodulate/re-encode step the thing that makes a hop *"a transmission, not a pointer move."* **Zero of 368 tests failed, including the new diameter-scale ones.**

Traced rather than dismissed: `Frame::phases()` only ever holds values in `{PHASE_TRUE, PHASE_FALSE}` — `encode` produces only those, and `corrupt()` just negates within that same pair. At a safe sampling instant, demodulating a phase already in that two-element set and reconstructing the frame is provably the identity, for every value this system can construct. The fault-injection model tests use (`medium: FnMut(&mut Frame)`, mutating the abstract frame directly) never touches the carrier-sample level the round-trip actually operates on, so there is no way — within the current design — to make the round-trip produce anything other than what skipping it would. Reverted the sabotage; this is not a defect, and the honest record is that this step's necessity is asserted by design intent, not currently distinguishable from a no-op by any test that could be written against this domain.

## Doctrine checks — three performed

| Sabotage | Failures |
|---|---|
| skip frame regeneration after demodulation | **0 of 368** — genuine no-op on this domain, not a gap; see above |
| non-strict routing descent (`d <= here` instead of `d < here`) | **2 of 368** — both instances of `stranded_packet_reports_no_descent`, exactly the property this guards |

Only two sabotages are listed as informative; the third (frame regeneration) is recorded above as a traced non-finding rather than a doctrine-check pass, since "0 failures" here means something different from a weak test — it means the mutation doesn't change behaviour on this domain at all.

## Still not built

Retransmission after `LinkLost`, concurrent sessions. Automatic teardown on drift is now built — see Slice 5.

## Human check

Read `diameter_of_the_documented_patch_is_measured` and the hypothesis section above. The first is the number every other test in this slice depends on, kept as a live assertion rather than a doc comment. The second is a case where checking a plausible-sounding concern against real numbers found nothing wrong — worth recording precisely because the instinct to "harden" it would have added complexity against a problem the geometry already rules out.

---

# Slice 5 — Automatic Teardown on Drift

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 373 passed; 0 failed
```

Workspace total: **368 → 373**. Closes the last item slices 2-4 all listed under "still not built": `equations.md`'s Standing Wave Superposition execution rule — *"Any phase variance exceeding `±π/4` triggers automatic phase inversion and teardown"* — had detection (`still_locked`) but no policy that acted on it.

## The gap, precisely

`Link::drift_to` deliberately never touches `state`, by design — detection stays pure measurement so it can't silently double as resolution, the same separation `symphony-kernel` keeps between deadlock detection and resolution. But nothing had ever been added on the *other* side of that separation: a link that drifted past the lock bound was reported as lost (`still_locked() == false`) while remaining internally `Resonant { .. }` forever, unless something happened to call `teardown` on it directly. The measurement and the physics it was supposed to describe could disagree indefinitely.

## `Link::enforce_lock` — the resolution `equations.md` names

```rust
pub fn enforce_lock(&mut self, omega: f64, t: f64) -> bool {
    if self.still_locked() { return true; }
    let _ = self.teardown(omega, t);
    false
}
```

Three lines, because the two things it composes — `still_locked` (detection) and `teardown` (the physical collapse) — already existed and already worked. This is a small synthesis in the same sense the earlier gate/video work was: nothing new was derived, an existing measurement and an existing action were wired to the policy the law already states connects them.

Wired into `Gateway::deliver_over_with` at both points a link's status matters: admission (before any hop runs) and the per-hop re-check. In both places the pre-existing amplitude/variance measurement is taken **before** calling `enforce_lock`, not after — `teardown`'s forced `π` shift changes the link's phases as a side effect, so measuring afterward would report the shift instead of the drift that actually caused the loss. `Delivery::LinkLost`'s `carrier_amplitude` and `FtgError::NoLock`'s `variance` keep exactly their pre-existing meaning.

## Two sabotages, one real and one a repeat of an established pattern

| Sabotage | Failures |
|---|---|
| `enforce_lock` reports the loss but never calls `teardown` | **3 of 373** — all three new end-to-end tests |
| remove an explicit `state == Collapsed` short-circuit before the `still_locked` check | **0 of 373** |

The first is the real one: exactly the three tests built to catch it (`enforce_lock_collapses_a_drifted_link`, and the two `deliver_over_with`-level tests), nothing collateral, nothing missed.

The second is the third occurrence this workspace has now recorded of a mutation revealing genuinely redundant-but-correct code rather than a bug: `teardown` itself already refuses cleanly on an already-collapsed link — `if self.state == LinkState::Collapsed { return Err(FtgError::Collapsed) }`, before mutating anything — so a *second* guard for the same case ahead of it in `enforce_lock` was never load-bearing. Removed rather than kept "for safety"; the method is three lines instead of six, and the doc comment says why, the same way the `ξ` piecewise split and the `deliver_through` no-op finding are documented rather than silently reverted.

## What this does *not* do

`enforce_lock` is not called automatically by `still_locked` or by anything on a timer — there is still no periodic "heartbeat" driving link lifecycle outside of an actual delivery attempt. A link that drifts and is never used again stays `Resonant` in memory, technically incorrect but inert, until something calls `deliver_over_with` (or `enforce_lock` directly) on it. This matches the existing "no concurrent sessions" scope boundary: nothing in this workspace runs a link's lifecycle independent of a caller actively using it, and adding a background driver was not part of what `equations.md`'s rule asks for — it asks for the *policy to exist*, not for a scheduler to invoke it unprompted.

## Still not built

Retransmission after `LinkLost`, concurrent sessions, and (as above) any background/periodic enforcement independent of an active delivery attempt.

## Human check

Read `drift_discovered_mid_transfer_actually_collapses_the_link`. It is the test that would have failed silently forever under the old code — not because anything crashed, but because `link.state()` after a `LinkLost` delivery would have kept reading `Resonant`, a mismatch between what the system reported and what was physically true that nothing surfaced until this test asked the question directly.
