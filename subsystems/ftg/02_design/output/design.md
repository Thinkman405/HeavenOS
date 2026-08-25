---
type: design
subsystem: ftg
stage: 02_design
derived_from: ["../01_derive/output/math-contract.md"]
---

# FTG — Design

Types and interfaces. Where the contract forbids a value, make it unrepresentable.

## Modules

| File | PRD | Job |
|---|---|---|
| `layers_1_2.rs` | §6 | framing, dissonance validation |
| `layers_3_4.rs` | §6 | address→cell mapping, metric-descent routing, port overtones |
| `session.rs` | §7 | handshake, teardown, link state |

## Layer 1/2 — framing

```rust
pub struct Frame { phases: Vec<f64> }        // payload followed by its complement

impl Frame {
    pub fn encode(payload: &[u8]) -> Self;
    pub fn dissonance(&self) -> f64;         // |sum sin(phi)|
    pub fn is_clean(&self) -> bool;          // dissonance below the floor
    pub fn decode(&self) -> Result<Vec<u8>, FtgError>;
    pub fn payload_len(&self) -> usize;
}
```

`decode` returns `Err(Dissonant { amplitude })` when the frame is corrupt. **There is no repair path and no `decode_lossy`** — the contract says dissipate, not correct, and offering a lossy variant would invite callers to bypass the check.

Bit↔phase comes from `substrate::translation`. This module adds the complement structure and the amplitude test, nothing more.

### The detection limit, in the type

`is_clean()` is deliberately **not** named `is_valid()`. A correlated flip of a symbol and its complement partner returns dissonance zero and passes — so the method reports what it measures (absence of net dissonance), not a guarantee it cannot make. The doc comment states the blind spot.

## Layer 3/4 — routing

```rust
pub struct NetAddress(pub u128);             // IPv4 or IPv6, widened

pub struct Router { tiling: Tiling, patch: HashSet<CellId> }

impl Router {
    pub fn new(depth: usize) -> Self;
    pub fn cell_for(&self, addr: NetAddress) -> CellId;         // deterministic, total
    pub fn next_hop(&self, at: CellId, dst: CellId) -> Result<CellId, FtgError>;
    pub fn route(&self, src: CellId, dst: CellId) -> Result<Vec<CellId>, FtgError>;
}
```

**No routing table exists as a field.** `next_hop` computes from the metric alone; the only state is the tiling itself. That is what makes the routing stateless in the contract's sense.

### Address→cell mapping — a stated assumption

The corpus does not derive one. Chosen: hash the address and index the patch in BFS ring order.

Properties that matter and hold: **deterministic** (same address → same cell always) and **total** (every address maps somewhere). Properties deliberately *not* claimed: locality-preservation (numerically adjacent IPs are not spatially adjacent) and uniformity beyond what the hash gives.

Recorded as an assumption in the log, not presented as derived law.

### Stuck detection

`next_hop` returns `Err(NoDescent { at, dst })` when no in-patch neighbour is strictly closer. Measured success on a connected patch is 4000/4000, but a patch with holes could strand a packet, and the implementation must report that rather than loop forever.

`route` carries a hop limit as a second guard, so a bug cannot hang the caller.

## Port multiplexing

```rust
pub struct Port(pub u16);
impl Port {
    pub fn overtone(self) -> AngularFrequency;   // (n+1) * omega_c
}
pub fn channels_interfere(a: Port, b: Port, samples: usize) -> f64;
```

`overtone` returns `substrate`'s `AngularFrequency` — the carrier is angular, and reusing the type keeps it from reaching `E = C_H·ν`.

## Session — §7

```rust
pub struct Oscillator { pub phase: f64, pub amplitude: f64 }

pub enum LinkState { Idle, Resonant { sync_phase: f64 }, Collapsed }

pub struct Link { a: Oscillator, b: Oscillator, state: LinkState }

impl Link {
    pub fn attempt_handshake(a: Oscillator, b: Oscillator) -> Result<Self, FtgError>;
    pub fn phase_variance(&self) -> f64;
    pub fn standing_wave(&self, k: f64, x: f64, t: f64) -> f64;   // 2A sin(kx) cos(wt)
    pub fn is_resonant(&self) -> bool;
    pub fn teardown(&mut self) -> f64;             // returns residual amplitude
    pub fn superposition(&self, t: f64) -> f64;
}
```

`attempt_handshake` returns `Err(NoLock { variance })` when variance is at or above `π/4`. The bound is **strict** — verified exclusive at the boundary.

`teardown` shifts by exactly `thresholds.teardown_phase_shift` and returns the residual so a caller can assert it, rather than trusting that it worked.

`LinkState::Collapsed` is terminal: a torn-down link cannot be reused. Reuse would mean a connection surviving amplitude zero, which contradicts the physics the design rests on.

## Errors

```rust
pub enum FtgError {
    Dissonant { amplitude: f64 },      // frame carries net interference
    NoDescent { at: CellId, dst: CellId },
    HopLimit { limit: usize },
    NoLock { variance: f64 },          // oscillators too far apart to resonate
    Collapsed,                         // operation on a torn-down link
    ZeroCrossing { t: f64 },           // propagated from substrate
}
```

Named for the physical failure, per doctrine.

## Float tolerances required

| Site | Nature |
|---|---|
| clean frame dissonance | measured **exactly 0.0** — assert equality |
| single-flip dissonance | measured **exactly 2.0** — assert equality |
| teardown residual | measured `≤ 1.11e-16`; set ε just above |
| channel orthogonality | `~1e-17` numeric; ε at `1e-12` for sampling error |
| handshake bound | exact comparison against `π/4`, strict `<` |
| routing hop counts | integers, no tolerance |

## Deliberately not built

- **Real socket I/O.** This is the translation and routing layer, not a driver. Nothing binds a NIC.
- **Fragmentation and reassembly.** A frame is one unit; MTU handling needs a transport policy the PRD does not specify.
- **Retransmission.** There is none by design — a dissonant frame dissipates. Whether a higher layer re-sends is that layer's concern.
- **§8 crystallisation.** Spun off to [[crystallisation]].

## Human check

For each type, could it hold a value the axioms forbid?

- `Frame` — cannot be decoded while dissonant; no lossy path exists.
- `Router` — holds no routing table; `next_hop` is pure metric descent.
- `Link` — cannot resonate at variance `≥ π/4`; cannot be reused once `Collapsed`.
- `Port::overtone` — returns `AngularFrequency`, so it cannot reach `E = C_H·ν`.
