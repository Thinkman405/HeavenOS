//! # FTG — the Fourier Transform Gateway
//!
//! The bridge between NEOS and the standard OSI world. Everything that moves a
//! packet or manages a connection: PRD §6 and §7.
//!
//! Application-data crystallisation (§8) is a separate record,
//! `subsystems/crystallisation` - it is a representation transform, not a
//! transport concern.
//!
//! ## Three things that will surprise a reader
//!
//! 1. **There is no CRC, and there must not be.** Frame validation is
//!    destructive interference: a clean frame cancels to exactly zero, a single
//!    flipped symbol gives exactly 2.0. See [`layers_1_2`].
//!
//! 2. **There is no routing table.** Forwarding is metric descent, and on this
//!    tiling greedy descent is not merely complete but **BFS-optimal**. See
//!    [`layers_3_4`].
//!
//! 3. **Teardown is not a message.** It is the amplitude reaching zero. A
//!    collapsed link is terminal. See [`session`].
//!
//! 4. **A corrupted frame dissipates where it was corrupted**, not at the
//!    destination. Every hop is a real transduction through the carrier, so a
//!    frame that cannot be recovered simply does not continue. See
//!    [`transport`].
//!
//! ## What this crate does not do
//!
//! The metric comes from `lattice`; bit-to-phase and carrier synthesis come
//! from `substrate`. Neither is reimplemented here.
//!
//! No socket I/O, no fragmentation, no retransmission - a dissonant frame
//! dissipates, and whether anything re-sends is a higher layer's concern.

pub mod layers_1_2;
pub mod layers_3_4;
pub mod session;
pub mod transport;

/// Constants generated from `_mkb/constants.json` at build time.
pub mod constants {
    include!(concat!(env!("OUT_DIR"), "/mkb_constants.rs"));
}

use lattice::tessellation::CellId;
use std::fmt;

/// Named for the physical failure, per `_mkb/test-doctrine.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FtgError {
    /// The frame carries net interference. It must be dissipated, never
    /// repaired - there is no correction path by design.
    Dissonant { amplitude: f64 },
    /// No neighbour is closer to the destination. The packet is stranded, which
    /// is reported rather than looped on.
    NoDescent { at: CellId, dst: CellId },
    /// Path exceeded its bound. A second guard so a bug cannot hang a caller.
    HopLimit { limit: usize },
    /// Oscillators too far apart in phase to resonate.
    NoLock { variance: f64 },
    /// Operation on a link that has already reached amplitude zero.
    Collapsed,
}

impl fmt::Display for FtgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dissonant { amplitude } => write!(
                f,
                "dissonant frame: net amplitude {amplitude}; dissipate, do not repair"
            ),
            Self::NoDescent { at, dst } => write!(
                f,
                "no descending neighbour from {at:?} toward {dst:?}; packet stranded"
            ),
            Self::HopLimit { limit } => write!(f, "path exceeded {limit} hops"),
            Self::NoLock { variance } => write!(
                f,
                "phase variance {variance} is at or above the lock bound; no resonance"
            ),
            Self::Collapsed => write!(f, "link has collapsed; amplitude zero is terminal"),
        }
    }
}

impl std::error::Error for FtgError {}

pub use layers_1_2::Frame;
pub use layers_3_4::{channel_overlap, NetAddress, Port, Router};
pub use session::{cancellation_floor, superpose, Link, LinkState, Oscillator};
pub use transport::{Delivery, Gateway, Packet};
