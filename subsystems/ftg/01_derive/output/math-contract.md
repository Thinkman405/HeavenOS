---
type: math-contract
subsystem: ftg
stage: 01_derive
derived_from: ["_mkb/axioms.md", "_mkb/equations.md", "_mkb/constants.json", "_spec/prd.md"]
consumes: [lattice, substrate]
---

# FTG — Math Contract

The complete law binding `neos/ftg`. Formulas copied, not paraphrased. Scope is **§6 + §7**: everything that moves a packet or manages a connection. §8 belongs to [[crystallisation]].

## 1. Binding axioms

**A2 — Logic Gate Override.** Binary states map to phase orientations `{−π/2, +π/2}`. Frame validation is interference, not comparison.

**A3 — Spatial Addressing Override.** Linear IP addresses map to hyperbolic lattice coordinates. Routing is metric descent, not table lookup.

A1 does not bind this subsystem.

## 2. Layer 1/2 — framing and geometric error checking

### 2.1 Transduction

$$W(t) = \sum_{k=0}^{N-1} \left[ A \cos(\omega_c t + \phi_k) \right], \qquad \phi_k \in \{-\pi/2, +\pi/2\}$$

Bit→phase mapping and carrier synthesis are **consumed from `substrate::translation`**, not reimplemented.

**Inherited constraint:** never demodulate at a carrier zero crossing. Both bit states read exactly zero at `t = 0` and every half period. Use `safe_sample_instant`.

### 2.2 Geometric error checking — no CRC

The PRD forbids CRC: "corrupted frames collapse into dissonance and are naturally dissipated."

A frame is its payload phases **followed by their complements**. Define frame dissonance as

$$D = \left|\sum_k \sin(\phi_k)\right|$$

Since `sin(±π/2) = ±1`, every payload symbol contributes `+1` or `−1` and its complement contributes the opposite. A clean frame therefore cancels **exactly**:

| Frame state | `D` |
|---|---|
| clean | **0.000** (exact, verified) |
| any single symbol flipped | **2.000** (exact, verified for every position) |

**Execution rule:** a frame with `D > 0` is dissonant and must be **dissipated, not repaired**. There is no correction path — the geometry rejects it.

### 2.3 Detection limit — stated, not hidden

A correlated flip of **both** a payload symbol and its complement partner returns `D = 0` and is **undetected**. Verified.

This is the same class of blind spot as parity, and it is a real limitation of interference checking: the mechanism sees net amplitude, so it cannot distinguish "no error" from "errors that cancel". It detects any odd number of flips within a payload/complement pair.

**Do not describe this scheme as detecting all corruption.** Any claim about frame integrity must carry this caveat.

## 3. Layer 3/4 — routing and multiplexing

### 3.1 Address mapping

IPv4/IPv6 linear addresses map to `{5,4}` cell coordinates. The mapping must be **deterministic and total** — the same address always names the same cell.

### 3.2 Routing by metric descent

$$d_{\mathbb{H}}(\mathbf{u}, \mathbf{v}) = \operatorname{arcosh}\left(1 + \frac{2\Vert{}\mathbf{u} - \mathbf{v}\Vert{}^2}{(1 - \Vert{}\mathbf{u}\Vert{}^2)(1 - \Vert{}\mathbf{v}\Vert{}^2)}\right)$$

Consumed from `lattice`, not reimplemented.

**Execution rule:** at each hop, forward to the neighbour minimising hyperbolic distance to the destination. No routing table, no flooding, no path state.

### 3.3 Greedy routing is complete *and* optimal here — verified

Greedy geometric routing gets stuck at local minima on general graphs. On this tiling it does not. Measured over a 441-cell patch:

| Property | Result |
|---|---|
| success rate | **4000 / 4000 pairs** |
| stuck at local minimum | **0** |
| routes longer than BFS-optimal | **0 of 400 sampled** |
| hop count | mean 7.76, max 10 |

Greedy descent finds a **shortest** path, not merely some path. This is the concrete payoff of the hyperbolic embedding and must be asserted in tests — if a change makes routing merely "successful", the property has silently degraded.

**Caveat:** verified on a connected patch grown from the origin. A patch with holes could strand a packet; `03_tests` must cover the stuck case and the implementation must report it rather than loop.

### 3.4 Harmonic multiplexing

Ports become overtones on the fundamental set by the IP coordinate:

$$\omega_{\text{port } n} = (n+1)\,\omega_c$$

**Verified orthogonal:** inner products between distinct port channels over one fundamental period are `~1e-17` — numerically zero. Self-overlap is exactly `0.5` (the mean of `cos²`). Distinct ports therefore do not interfere, which is what makes them independent channels rather than a shared medium.

## 4. §7 — connection lifecycle

### 4.1 Resonant Handshake (replaces SYN/ACK)

$$f(t) = 2A \sin(kx)\cos(\omega_{sync} t)$$

Two independent oscillators synchronise into a shared standing wave.

**Execution rule:** a link is established when phase variance between the two oscillators is **strictly below** `π/4` (`thresholds.link_stability_phase_variance`). At or above, no lock. Verified exclusive at the boundary.

Established links must be re-checked; variance drifting to `≥ π/4` triggers automatic teardown per `equations.md`.

### 4.2 Phase Inversion Teardown (replaces FIN/ACK) — Test Case 1

$$f_{total} = f_{node A} + f_{node B} = 0$$

The node shifts transmission phase by **exactly π**, forcing combined amplitude to zero.

**Execution rule:** teardown is not a message. It is the amplitude reaching zero. Resource reclamation follows from `E = C_H·ν → 0`.

**Verified:** with A at `φ = 0` and B at `φ = π`, the sum is `≤ 1.11e-16` at every sampled `t` — cancellation is continuous, not sampled. `03_tests` must implement **Test Case 1** from `_mkb/test-doctrine.md` directly, and state its epsilon against this measured floor.

## 5. Consumed, never reimplemented

| From | What |
|---|---|
| `lattice` | hyperbolic metric, `{5,4}` tiling, `CellId`, `Cell::neighbors()` |
| `substrate` | bit↔phase, carrier synthesis, `safe_sample_instant`, `AngularFrequency` |

One home per fact applies to code. Neither the distance function nor the bit/phase mapping may appear here.

## 6. Forbidden constructs

| Forbidden | Because |
|---|---|
| CRC or checksum for frame validation | §2.2 — the geometry does it |
| repairing a dissonant frame | §2.2 — dissipate, do not correct |
| claiming full corruption detection | §2.3 — correlated flips are invisible |
| routing tables or path state | §3.2 — descent is stateless |
| demodulating at a carrier zero crossing | inherited from `substrate` |
| `ω_c` where `ν` is required | inherited; the newtypes make it a compile error |
| reimplementing the metric or bit/phase map | §5 |
| hardcoding any constant from `constants.json` | one home per fact |

## 7. Constants consumed

| JSON path | Use |
|---|---|
| `constants.baseline_carrier_frequency.value` | `ω_c`, and the overtone fundamental |
| `logic_phases.*` | bit ↔ phase |
| `thresholds.link_stability_phase_variance` | handshake lock bound (`π/4`) |
| `thresholds.teardown_phase_shift` | the exact `π` inversion |

## 8. Open questions

**None blocking.** The address→cell mapping (§3.1) is unspecified by the corpus; `02_design` chooses one and records it as a stated assumption rather than a derivation.
