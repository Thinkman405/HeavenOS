//! Raw binary <-> wave translation.
//!
//! Contract §5. Bits map to phase orientations (axiom A2) and synthesize onto
//! the carrier:
//!
//! ```text
//! W(t) = sum_k [ A cos(omega_c t + phi_k) ],  phi_k in {-pi/2, +pi/2}
//! ```
//!
//! ## The zero-crossing hazard
//!
//! `cos(x + pi/2) = -sin(x)` and `cos(x - pi/2) = +sin(x)`, so the two bit
//! states differ **only in the sign of the sine component**. At `t = 0` - and
//! at every half period - both evaluate to exactly zero and carry no
//! information at all.
//!
//! Demodulating there recovers nothing. [`demodulate`] therefore returns
//! [`SubstrateError::ZeroCrossing`] rather than silently producing garbage
//! bits, which is how this would otherwise present: intermittent corruption at
//! every layer above, with no local cause.
//!
//! Use [`safe_sample_instant`] for the quarter periods where bit separation is
//! maximal (exactly 2.0).

use crate::clock::CARRIER;
use crate::constants::{PHASE_FALSE, PHASE_TRUE};
use crate::SubstrateError;

/// Tolerance for classifying a phase as one of the two permitted orientations.
pub const PHASE_TOLERANCE: f64 = 1e-9;

/// How close to a zero crossing counts as one.
const CROSSING_TOLERANCE: f64 = 1e-9;

/// Bits, most significant first, to phase orientations.
pub fn bits_to_phases(bytes: &[u8]) -> Vec<f64> {
    let mut out = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for bit in (0..8).rev() {
            out.push(if (byte >> bit) & 1 == 1 {
                PHASE_TRUE
            } else {
                PHASE_FALSE
            });
        }
    }
    out
}

/// Phase orientations back to bits.
///
/// # Errors
/// [`SubstrateError::IndeterminatePhase`] if a phase is neither orientation.
/// A2 admits exactly two; anything else is not a logic state and must not be
/// rounded into one.
pub fn phases_to_bits(phases: &[f64]) -> Result<Vec<u8>, SubstrateError> {
    if phases.len() % 8 != 0 {
        return Err(SubstrateError::IndeterminatePhase { phi: f64::NAN });
    }
    let mut out = Vec::with_capacity(phases.len() / 8);
    for chunk in phases.chunks(8) {
        let mut byte = 0u8;
        for &phi in chunk {
            let bit = if (phi - PHASE_TRUE).abs() <= PHASE_TOLERANCE {
                1
            } else if (phi - PHASE_FALSE).abs() <= PHASE_TOLERANCE {
                0
            } else {
                return Err(SubstrateError::IndeterminatePhase { phi });
            };
            byte = (byte << 1) | bit;
        }
        out.push(byte);
    }
    Ok(out)
}

/// The superposed carrier `W(t)` for a run of phases.
pub fn synthesize(phases: &[f64], t: f64, amplitude: f64) -> f64 {
    let w = CARRIER.get();
    phases.iter().map(|phi| amplitude * (w * t + phi).cos()).sum()
}

/// The carrier contribution of a single phase.
pub fn carrier_at(phi: f64, t: f64, amplitude: f64) -> f64 {
    amplitude * (CARRIER.get() * t + phi).cos()
}

/// The `k`-th instant where bit separation is maximal.
///
/// Odd quarter periods: `omega_c t = pi/2 + k*pi`. Separation there is exactly
/// 2.0, versus exactly 0.0 at the zero crossings between them.
pub fn safe_sample_instant(k: u32) -> f64 {
    let w = CARRIER.get();
    (std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * f64::from(k)) / w
}

/// Whether `t` is a carrier zero crossing, where no information is recoverable.
pub fn is_zero_crossing(t: f64) -> bool {
    let x = CARRIER.get() * t;
    x.sin().abs() <= CROSSING_TOLERANCE
}

/// Recover bits from phases carried at instant `t`.
///
/// # Errors
/// [`SubstrateError::ZeroCrossing`] if `t` is a zero crossing. Both bit states
/// read as zero there; returning bits would be fabricating them.
pub fn demodulate(phases: &[f64], t: f64) -> Result<Vec<u8>, SubstrateError> {
    if is_zero_crossing(t) {
        return Err(SubstrateError::ZeroCrossing { t });
    }
    let sign = (CARRIER.get() * t).sin().signum();
    let recovered: Vec<f64> = phases
        .iter()
        .map(|&phi| {
            // c = -sin(x) for phi = +pi/2, +sin(x) for phi = -pi/2.
            let c = carrier_at(phi, t, 1.0);
            if c * sign < 0.0 {
                PHASE_TRUE
            } else {
                PHASE_FALSE
            }
        })
        .collect();
    phases_to_bits(&recovered)
}

/// Superposition of two phases on the carrier at `t`.
///
/// Exactly zero for opposed phases, at **every** `t` - not merely at sample
/// points. That continuity is what lets phase teardown work without an
/// acknowledgement message.
pub fn superpose(a: f64, b: f64, t: f64, amplitude: f64) -> f64 {
    carrier_at(a, t, amplitude) + carrier_at(b, t, amplitude)
}
