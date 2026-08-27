---
type: subsystem
subsystem: ftg
tier: bridge
language: Rust
stage: 04_implement
status: complete
result: "61 tests passing. Framing, dissonance validation, hyperbolic routing, port overtones, handshake, teardown, end-to-end delivery, session-gated delivery, multi-hop propagation verified at the documented patch's true diameter (10 hops), and automatic teardown-on-drift."
slices: ["framing + routing + session", "end-to-end transport", "session-gated delivery", "multi-hop propagation at scale", "automatic teardown on drift"]
prd_sections: ["6", "7"]
binds_axioms: ["A2", "A3"]
consumes: [lattice, substrate]
spun_off: crystallisation
---

# FTG — the Fourier Transform Gateway

One job: move packets and manage connections between NEOS and the standard OSI world — binary converted to continuous wave phenomena, routed through hyperbolic space, with connections forged and torn down as physical resonance.

Everything that **moves a packet or manages a connection**. Application-data crystallisation (§8) was spun off to [[crystallisation]].

## The build loop

| Stage | Job | Output |
|---|---|---|
| `01_derive` | pull the exact law that binds this subsystem | `math-contract.md` |
| `02_design` | types and interfaces against that contract | `design.md` |
| `03_tests` | physics assertions, written before code | `test-plan.md` |
| `04_implement` | write the Rust into `neos/ftg/` | `implementation-log.md` |

## Scope

**Owns:** `neos/ftg/**` — `layers_1_2.rs`, `layers_3_4.rs`, `session.rs`
**PRD sections:** §6 (Networking), §7 (Network State Management)
**Axioms that bind it:** A2 (bit → phase), A3 (address → hyperbolic coordinate)
**Constants read:** `baseline_carrier_frequency`, `logic_phases`, `thresholds.*`

### The three concerns

| Concern | PRD | Job |
|---|---|---|
| **Layer 1/2** | §6 | wave framing; **geometric error checking by destructive interference** — corrupted frames collapse into dissonance and dissipate. **Do not implement CRC**; the geometry does the checking. |
| **Layer 3/4** | §6 | Poincaré-disk hyperbolic routing over lattice cell patches; TCP/UDP ports as harmonic overtones on the fundamental wave |
| **Session** | §7 | Resonant Handshake (SYN/ACK replacement) — two oscillators synchronising into a shared standing wave; Phase Inversion Teardown (FIN/ACK replacement) — a π shift forcing amplitude to zero |

## Multi-hop propagation, verified at the patch's true diameter

Routing (path-finding) and transport (wave-level delivery) were each tested — but at different scales. Routing was proven BFS-optimal across the documented 441-cell patch; transport was only ever exercised at 3-4 hops, on the same patch whose measured diameter is **10 hops**. Neither claim was wrong, but nothing had checked they actually met.

Closed by measuring the real diameter (`diameter_of_the_documented_patch_is_measured`) and delivering, byte-exact, at it — both ungated (`farthest_pair_delivers_byte_exact_at_the_true_diameter`) and session-gated (`undisturbed_session_survives_the_true_diameter`).

A hypothesis worth recording because it was checked rather than assumed either way: high hop counts could plausibly threaten `safe_sample_instant`'s trig precision. They do not, and the reason is the geometry itself — ring `n` of `{5,4}` holds `5·Fib(2n)` cells, so cell count grows exponentially while hop count from a centre grows only logarithmically. A 3046-cell patch has a longest-from-origin path of 7 hops.

## Automatic teardown on drift

`equations.md`'s Standing Wave Superposition rule — *"any phase variance exceeding `±π/4` triggers automatic phase inversion and teardown"* — had detection (`Link::still_locked`) but no policy that acted on it. A link that drifted past the lock bound was reported lost while remaining internally `Resonant { .. }` indefinitely, since `Link::drift_to` deliberately never touches `state` — detection stays pure measurement by design, the same separation [[symphony-kernel]] keeps between deadlock detection and resolution.

`Link::enforce_lock` is the resolution the law names: check `still_locked`, and if it fails, actually call `teardown`. Three lines, because both halves already existed — this composes them rather than adding new physics. Wired into `Gateway::deliver_over_with` at admission and at every hop, always measuring amplitude/variance **before** calling it, since `teardown`'s forced `π` shift would otherwise make a later read describe the shift rather than the drift that caused the loss.

## Carries Test Case 1

Test Case 1 in [`_mkb/test-doctrine.md`](../../_mkb/test-doctrine.md) — Destructive Interference Teardown — proves **Phase Inversion Teardown**, which is §7. It belongs to the session concern, not to routing. `03_tests` must implement the assertion, not merely cite it.

This is why the record keeps §7: a routing-only `ftg` could not carry its own canonical doctrine test.

## Depends on — both built

- **[[lattice]]** — the hyperbolic metric and `{5,4}` cell naming for Layer 3/4 routing. Do not implement the distance function twice.
- **[[substrate]]** — the address space and the binary↔wave primitives. Its public API yields **only** `LatticeAddress`; no pointer or byte index exists to obtain, so routing reads a native non-Euclidean space. Allocation locality follows lattice adjacency, so "near in the metric" and "near in memory" mean the same thing.

Two inherited constraints, non-negotiable:

- **Never demodulate at a carrier zero crossing.** Both bit states read exactly zero at `t = 0` and every half period. Use `translation::safe_sample_instant`. Layer 1/2 depends on this directly.
- **`ω_c` is angular.** It must never reach `E = C_H·ν`. The newtypes make that a compile error; do not add a conversion to work around it.

## Scale note

Three concerns, two PRD sections. If `02_design` sprawls, the natural next cut is **transport (§6) vs. session (§7)** — but they share most of their law (carrier synthesis, phase orientation, the standing-wave equations), so splitting early would duplicate contracts rather than separate them.

## A real coverage gap, found from outside this record

`neos/tests/geometric_testbed.rs` (a cross-cutting harness owned by neither `ftg` nor `symphony-kernel` — see root `CONTEXT.md`'s cross-cutting slices) sabotaged `Router::adjacent`'s equality check (`==` flipped to `!=`) and found this crate's own 62-test suite did not notice: the check's one existing caller, `every_hop_crosses_an_edge` in `neos/tests/ftg.rs`, only ever asserted the positive case (a real route hop is adjacent). Under the inversion, `adjacent(a, b)` returns true for nearly any `b` — a cell's five neighbours are almost never all equal to one fixed `b` — so a positive-only check cannot distinguish "correct" from "always true." Fixed in place, in this record's own test, not worked around from outside: `every_hop_crosses_an_edge` now also confirms a same-route pair two hops apart (verified via `bfs_hops`, not assumed) is correctly refused as non-adjacent. Sabotage re-run after the fix: caught immediately.

## Do not

Load other subsystems' records. They don't share state; they share the factory (`_mkb/`, `_spec/`).
