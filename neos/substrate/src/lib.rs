//! # Substrate - NEOS Tier 1
//!
//! The hypervisor floor: where a wave-based geometric OS meets flat silicon.
//!
//! Law lives in `_mkb/`; this crate is downstream of it. Contract:
//! `subsystems/substrate/01_derive/output/math-contract.md`.
//!
//! ## The one thing this subsystem exists to decide
//!
//! Hardware is flat and byte-addressed. Axiom A3 says addressable space is
//! hyperbolic. **The boundary is the public API of [`memory`]**: flat offsets
//! exist only inside that module, and every public address is a lattice
//! coordinate.
//!
//! This is structural rather than conventional. A consumer able to obtain a
//! flat address would eventually compute with it, and would then be working in
//! Euclidean space no matter what the geometry layer claims. `ftg` Layer 3/4
//! routing must read a native non-Euclidean space.
//!
//! ## Two hazards worth knowing before reading further
//!
//! 1. **The carrier is information-free at zero crossings.** Both bit states
//!    evaluate to exactly zero at `t = 0` and every half period. See
//!    [`translation`].
//!
//! 2. **`omega_c` is angular.** It must never reach `E = C_H * nu`, which takes
//!    ordinary frequency. The newtypes in [`clock`] make that a compile error.
//!
//! ## Layering
//!
//! `lattice <- substrate <- symphony-kernel`, matching the PRD's tiering:
//! Symphony runs *on* the Substrate. The frequency newtypes live here because
//! this is the lowest layer that uses them.

pub mod clock;
pub mod memory;
pub mod translation;

/// Constants generated from `_mkb/constants.json` at build time.
pub mod constants {
    include!(concat!(env!("OUT_DIR"), "/mkb_constants.rs"));
}

use lattice::tessellation::CellId;
use std::fmt;

/// Named for the physical failure, per `_mkb/test-doctrine.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SubstrateError {
    /// Not enough space across the pool's cells.
    Exhausted { requested: usize, available: usize },
    /// A cell that is not part of this pool.
    Unmapped { cell: CellId },
    /// A position past the end of its cell.
    OffsetOutOfCell { offset: usize, capacity: usize },
    /// The carrier carries no information at this instant - both bit states
    /// evaluate to zero. Returning bits here would be fabricating them.
    ZeroCrossing { t: f64 },
    /// A phase that is neither permitted orientation. A2 admits exactly two.
    IndeterminatePhase { phi: f64 },
    /// Pool split would leave the `(x)` operator's domain.
    SplitDomain { unit: f64 },
    /// A curved address's `(x)`-fold left the operator's domain part-way
    /// through resolving, so no cell can be named. Carries the underlying
    /// `lattice` failure rather than re-deriving a description of it - the
    /// failure already has a name, one home for it.
    AddressUnresolvable(lattice::LatticeError),
}

impl fmt::Display for SubstrateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted { requested, available } => write!(
                f,
                "pool exhausted: requested {requested} bytes, {available} available"
            ),
            Self::Unmapped { cell } => write!(f, "cell {cell:?} is not mapped in this pool"),
            Self::OffsetOutOfCell { offset, capacity } => write!(
                f,
                "offset {offset} exceeds cell capacity {capacity}"
            ),
            Self::ZeroCrossing { t } => write!(
                f,
                "t = {t} is a carrier zero crossing; no information is recoverable there"
            ),
            Self::IndeterminatePhase { phi } => write!(
                f,
                "phase {phi} is neither -pi/2 nor +pi/2 and is not a logic state"
            ),
            Self::SplitDomain { unit } => {
                write!(f, "splitting extent {unit} leaves the (x) operator domain")
            }
            Self::AddressUnresolvable(source) => {
                write!(f, "curved address unresolvable: {source}")
            }
        }
    }
}

impl std::error::Error for SubstrateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AddressUnresolvable(source) => Some(source),
            _ => None,
        }
    }
}

/// What a trap handler decided to do about a fault, per [`Hypervisor::allocate_trapped`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapAction {
    /// The handler took corrective action; retry the faulting operation.
    Retry,
    /// The handler declined, or couldn't help; propagate the fault.
    Propagate,
}

/// The hypervisor: memory, clock, and the translation pipeline.
pub struct Hypervisor {
    pool: memory::MemoryPool,
    carrier: clock::AngularFrequency,
    ticks: u64,
}

impl Hypervisor {
    pub fn boot(cells: usize, cell_capacity: usize) -> Self {
        Self {
            pool: memory::MemoryPool::new(cells, cell_capacity),
            carrier: clock::CARRIER,
            ticks: 0,
        }
    }

    pub fn pool(&self) -> &memory::MemoryPool {
        &self.pool
    }

    pub fn pool_mut(&mut self) -> &mut memory::MemoryPool {
        &mut self.pool
    }

    pub fn carrier(&self) -> clock::AngularFrequency {
        self.carrier
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Advance one quarter period and return the new time.
    ///
    /// A quarter period deliberately: that is the cadence at which the carrier
    /// is demodulable, so the clock and [`translation::demodulate`] agree by
    /// construction rather than by the caller remembering.
    pub fn tick(&mut self) -> f64 {
        self.ticks += 1;
        self.uptime_seconds()
    }

    pub fn uptime_seconds(&self) -> f64 {
        self.ticks as f64 * self.carrier.quarter_period()
    }

    /// Allocate `bytes`, routing any fault through a real trap handler
    /// before deciding whether to retry or propagate — see this crate's
    /// implementation log for why this exists and, just as importantly,
    /// what it deliberately is *not*.
    ///
    /// Scoped to `allocate` specifically, not `read`/`write` too: an
    /// `Exhausted` fault has a genuine corrective action a handler can take
    /// (free something, then retry) — the same shape as a page fault a real
    /// OS resolves by evicting a page. `Unmapped`/`OffsetOutOfCell` faults
    /// from `read`/`write` are wrong-argument errors from the caller's own
    /// logic; no handler can fix those by acting on the pool, so trapping
    /// them would add an interface with nothing real behind it.
    ///
    /// The handler is called on **every** fault, unconditionally — a
    /// genuine transfer of control, not a conditional notification — and
    /// receives `&mut MemoryPool` directly, the same pool the failing
    /// allocation is against, so a real recovery action (freeing an
    /// allocation it knows about) is actually possible, not merely
    /// simulated by a closure that captured unrelated state.
    ///
    /// What's bounded is how many times the operation gets *retried*
    /// afterward: `max_retries`, a second guard so a handler that never
    /// actually resolves the fault cannot hang the caller in an infinite
    /// loop, the same shape as `ftg::Router::route`'s own `max_hops`.
    pub fn allocate_trapped(
        &mut self,
        bytes: usize,
        max_retries: usize,
        mut handler: impl FnMut(SubstrateError, &mut memory::MemoryPool) -> TrapAction,
    ) -> Result<Allocation, SubstrateError> {
        let mut attempts = 0;
        loop {
            match self.pool.allocate(bytes) {
                Ok(alloc) => return Ok(alloc),
                Err(e) => {
                    let action = handler(e, &mut self.pool);
                    if action == TrapAction::Propagate || attempts >= max_retries {
                        return Err(e);
                    }
                    attempts += 1;
                }
            }
        }
    }
}

pub use clock::{AngularFrequency, Frequency, CARRIER};
pub use memory::{Allocation, CellOffset, LatticeAddress, MemoryPool};
