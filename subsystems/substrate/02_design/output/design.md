---
type: design
subsystem: substrate
stage: 02_design
derived_from: ["../01_derive/output/math-contract.md"]
---

# Substrate — Design

Types and interfaces. Where the contract forbids a value, make it unrepresentable.

## Modules

| File | Job |
|---|---|
| `clock.rs` | `ω_c`, the frequency newtypes (shared downward) |
| `memory.rs` | non-Euclidean memory pool — **the flat/curved boundary** |
| `translation.rs` | binary ↔ wave pipelines |
| `main.rs` | hypervisor entry, VM bootstrap |

## The flat/curved boundary

Contract §2. This is the subsystem's reason to exist, so it gets the strongest available mechanism.

```rust
pub struct LatticeAddress { cell: CellId, offset: CellOffset }

struct FlatOffset(usize);   // PRIVATE — never leaves this module
```

- `LatticeAddress` is the **only** public address type.
- `FlatOffset` is private to `memory`. It has no public constructor, no accessor, and no `Deref`.
- `MemoryPool` exposes no `as_ptr`, no `as_slice`, no `usize` index.

A consumer literally cannot obtain a linear address — not by convention, by construction. `CellOffset` is a bounded intra-cell position, publicly constructible but meaningless outside its cell, so it cannot be used as a global linear index.

**Why this severity:** any consumer that can get a flat address will eventually do arithmetic on it, and at that moment it is working in Euclidean space no matter what the geometry layer claims. `ftg` must read a native non-Euclidean space. A convention would not survive a hot loop; a private type will.

## Memory

```rust
pub struct MemoryPool { /* cells -> slabs, backed by lattice::Tiling */ }

impl MemoryPool {
    pub fn new(cells: usize, cell_capacity: usize) -> Self;
    pub fn allocate(&mut self, bytes: usize) -> Result<Allocation, SubstrateError>;
    pub fn write(&mut self, at: LatticeAddress, data: &[u8]) -> Result<(), SubstrateError>;
    pub fn read(&self, at: LatticeAddress, len: usize) -> Result<Vec<u8>, SubstrateError>;
    pub fn distance(&self, a: LatticeAddress, b: LatticeAddress) -> f64;  // hyperbolic
    pub fn split(&mut self) -> Result<f64, SubstrateError>;               // A1
    pub fn free(&mut self, alloc: &Allocation);
}

pub struct Allocation { cells: Vec<CellId>, start: LatticeAddress, len: usize }
```

`Allocation` records **which cells** it occupies, not a base+length in bytes. `distance` returns hyperbolic distance between cell centres — the only meaningful notion of "how far apart" two addresses are here.

### Allocation locality

An allocation larger than one cell extends into **adjacent** cells, found by `Cell::neighbors()` via breadth-first growth from the seed cell. Never "the next index".

This is the property `ftg` depends on: addresses that are near in the metric are near in the allocation, so routing by hyperbolic distance is meaningful rather than decorative.

## Clock and frequency types

```rust
pub struct Frequency(f64);         // ordinary, Hz
pub struct AngularFrequency(f64);  // rad/s
pub const CARRIER: AngularFrequency;  // omega_c
```

**These move here from `symphony-kernel`.** Substrate is the lowest layer that uses them (the clock is `ω_c`), so this is their home; `symphony-kernel` gains a dependency on `substrate` and re-exports. Keeping two copies would be exactly the drift the MKB forbids.

Dependency direction becomes `lattice ← substrate ← symphony-kernel`, matching the PRD's tiering — Symphony runs *on* the Substrate.

No shared arithmetic, no `From`; conversion stays explicit.

## Translation

```rust
pub fn bits_to_phases(bytes: &[u8]) -> Vec<f64>;
pub fn phases_to_bits(phases: &[f64]) -> Result<Vec<u8>, SubstrateError>;
pub fn synthesize(phases: &[f64], t: f64, amplitude: f64) -> f64;   // W(t)
pub fn sample_at_quarter_period(k: u32) -> f64;                     // safe sampling instants
pub fn demodulate(phases: &[f64], t: f64) -> Result<Vec<u8>, SubstrateError>;
```

### The zero-crossing hazard

Contract §5.1. `cos(x ± π/2) = ∓sin(x)`, so the two bit states differ only in the sign of the sine component — and at `t = 0` (or any half period) **both read as exactly zero**. Demodulating there recovers nothing.

`demodulate` therefore **returns an error** at a zero crossing rather than silently returning garbage bits. `sample_at_quarter_period` gives the instants where separation is maximal (2.0). A caller that wants a specific `t` must handle the error; a caller that just wants correct bits uses the helper.

This is the kind of defect that would otherwise present as intermittent corruption at higher layers.

## Errors

```rust
pub enum SubstrateError {
    Exhausted { requested: usize, available: usize },
    Unmapped { cell: CellId },
    OffsetOutOfCell { offset: usize, capacity: usize },
    ZeroCrossing { t: f64 },        // no information recoverable here
    IndeterminatePhase { phi: f64 },// not one of the two orientations
    SplitDomain { unit: f64 },      // outside (x)'s domain
}
```

Named for the physical failure, per doctrine.

## Hypervisor bootstrap

```rust
pub struct Hypervisor { pool: MemoryPool, carrier: AngularFrequency, ticks: u64 }
impl Hypervisor {
    pub fn boot(cells: usize, cell_capacity: usize) -> Self;
    pub fn tick(&mut self) -> f64;      // advances by one quarter period
    pub fn uptime_seconds(&self) -> f64;
}
```

`tick` advances by a quarter period deliberately — that is the sampling cadence §5.1 requires, so the clock and the demodulator agree by construction rather than by the caller remembering.

`main.rs` is a thin binary over this; the library is where everything testable lives.

## Deliberately not built

- **Virtualisation proper** — trapping, guest isolation, privilege levels. "Virtual Machine" in the PRD names the translation layer; real guest execution needs the Symphony instruction model.
- **Concurrent allocation.** `MemoryPool` is single-threaded. Making it shared needs a decision about whether locks live here or in `symphony-kernel`, and that is a scheduler question.
- **Fractal area preservation on resize.** PRD §5 assigns it to `lattice`; substrate would consume it. `lattice` has not built it.

## Human check

For each type, could it hold a value the axioms forbid?

- `LatticeAddress` — cannot express a flat address; `FlatOffset` is private.
- `MemoryPool` — exposes no pointer, slice, or `usize` index.
- `Frequency` / `AngularFrequency` — cannot be confused; one home now.
- `demodulate` — cannot silently return garbage at a zero crossing.
