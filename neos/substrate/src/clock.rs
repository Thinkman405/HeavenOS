//! The carrier clock, and the frequency newtypes shared by every layer above.
//!
//! These types live here because substrate is the **lowest** layer that uses
//! them: `omega_c` is the substrate's own clock. `symphony-kernel` re-exports
//! rather than redefining, so there is one home per fact and the two cannot
//! drift apart.

use crate::constants::CARRIER_RAD_PER_SEC;
use std::f64::consts::TAU;

/// Ordinary frequency, in hertz.
///
/// This is what the Howard Comma pairs with: `E = C_H * nu`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Frequency(f64);

/// Angular frequency, in radians per second.
///
/// Shares no arithmetic with [`Frequency`] and has no `From` impl. `omega_c`
/// is this type, so feeding the carrier to `E = C_H * nu` is a compile error
/// rather than a silent factor of `2*pi` - the units cannot tell them apart,
/// so the compiler must.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AngularFrequency(f64);

impl Frequency {
    pub const fn hertz(v: f64) -> Self {
        Self(v)
    }
    pub const fn get(self) -> f64 {
        self.0
    }
    /// Explicit, named conversion. Never implicit.
    pub fn to_angular(self) -> AngularFrequency {
        AngularFrequency(self.0 * TAU)
    }
}

impl AngularFrequency {
    pub const fn rad_per_sec(v: f64) -> Self {
        Self(v)
    }
    pub const fn get(self) -> f64 {
        self.0
    }
    /// Explicit, named conversion. Never implicit.
    pub fn to_ordinary(self) -> Frequency {
        Frequency(self.0 / TAU)
    }

    /// Full period, in seconds.
    pub fn period(self) -> f64 {
        TAU / self.0
    }

    /// Quarter period - the interval between valid demodulation instants.
    ///
    /// See `translation`: the carrier carries no information at zero
    /// crossings, and quarter periods are where bit separation is maximal.
    pub fn quarter_period(self) -> f64 {
        self.period() / 4.0
    }
}

/// The resting synchronisation parameter, `omega_c`.
pub const CARRIER: AngularFrequency = AngularFrequency::rad_per_sec(CARRIER_RAD_PER_SEC);
