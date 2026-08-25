//! Layer 1/2 — wave framing and geometric error checking.
//!
//! Contract §2. Bit-to-phase and carrier synthesis come from
//! `substrate::translation`; this module adds the complement structure and the
//! amplitude test, nothing more.
//!
//! ## Validation is interference, not a checksum
//!
//! A frame is its payload phases followed by their complements. Since
//! `sin(+-pi/2) = +-1`, every payload symbol contributes `+1` or `-1` and its
//! complement contributes the opposite, so a clean frame cancels **exactly**.
//!
//! | frame state | dissonance |
//! |---|---|
//! | clean | 0.0 exactly |
//! | any single symbol flipped | 2.0 exactly |
//!
//! There is no CRC here and there must not be: the PRD is explicit that
//! corrupted frames "collapse into dissonance and are naturally dissipated".

use crate::constants::{PHASE_FALSE, PHASE_TRUE};
use crate::FtgError;
use substrate::translation;

/// Amplitude below which a frame counts as fully cancelled.
///
/// Public so [`crate::transport`] can apply the same threshold per hop that
/// [`Frame::decode`] applies at the destination - one home for the criterion.
///
/// Cancellation is exact for a clean frame, so this is not a fudge for expected
/// error - it is a floor for accumulated summation noise on long frames.
pub const DISSONANCE_FLOOR: f64 = 1e-9;

/// A wave frame: payload phases followed by their complements.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    phases: Vec<f64>,
    payload_bits: usize,
}

impl Frame {
    /// Encode a payload, appending the complement that makes a clean frame
    /// cancel to zero.
    pub fn encode(payload: &[u8]) -> Self {
        let base = translation::bits_to_phases(payload);
        let mut phases = base.clone();
        phases.extend(base.iter().map(|p| -p));
        Self {
            payload_bits: base.len(),
            phases,
        }
    }

    /// Build from raw phases, e.g. as received off the wire.
    pub fn from_phases(phases: Vec<f64>) -> Self {
        let payload_bits = phases.len() / 2;
        Self {
            phases,
            payload_bits,
        }
    }

    pub fn phases(&self) -> &[f64] {
        &self.phases
    }

    pub fn payload_bits(&self) -> usize {
        self.payload_bits
    }

    /// Net interference across the frame: `|sum sin(phi)|`.
    ///
    /// Exactly zero for a clean frame; exactly 2.0 for a single flipped symbol.
    pub fn dissonance(&self) -> f64 {
        self.phases.iter().map(|p| p.sin()).sum::<f64>().abs()
    }

    /// Whether the frame carries no net dissonance.
    ///
    /// # This is not a validity guarantee
    ///
    /// A correlated flip of a symbol **and its complement partner** cancels
    /// and reports clean. The mechanism measures net amplitude, so it cannot
    /// distinguish "no error" from "errors that cancel" - the same blind spot
    /// parity has. It detects any odd number of flips within a pair.
    ///
    /// Named `is_clean` rather than `is_valid` for exactly this reason.
    pub fn is_clean(&self) -> bool {
        self.dissonance() <= DISSONANCE_FLOOR
    }

    /// Recover the payload.
    ///
    /// # Errors
    /// [`FtgError::Dissonant`] if the frame carries net interference. There is
    /// deliberately **no lossy variant**: the contract says dissipate, never
    /// repair, and offering one would invite callers around the check.
    pub fn decode(&self) -> Result<Vec<u8>, FtgError> {
        let d = self.dissonance();
        if d > DISSONANCE_FLOOR {
            return Err(FtgError::Dissonant { amplitude: d });
        }
        translation::phases_to_bits(&self.phases[..self.payload_bits])
            .map_err(|_| FtgError::Dissonant { amplitude: d })
    }

    /// Flip one symbol. Test support for corruption scenarios.
    pub fn corrupt(&mut self, index: usize) {
        if let Some(p) = self.phases.get_mut(index) {
            *p = -*p;
        }
    }

    /// The phase a bit maps to, for callers building frames by hand.
    pub fn phase_for_bit(bit: bool) -> f64 {
        if bit {
            PHASE_TRUE
        } else {
            PHASE_FALSE
        }
    }
}
