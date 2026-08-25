//! Energy quantization: `E = C_H * nu`.
//!
//! Law: `_mkb/equations.md`, `_mkb/resonance.md` Part 1.
//! Contract: `subsystems/symphony-kernel/01_derive/output/math-contract.md` §2.

use crate::constants::HOWARD_COMMA;

/// Frequency types come from `substrate`, which is the lowest layer that uses
/// them (its clock is `omega_c`). Re-exported rather than redefined: two
/// copies of a type this load-bearing would drift, and the whole point of the
/// separation is that the compiler catches `nu`/`omega` confusion.
pub use substrate::{AngularFrequency, Frequency};

/// Energy, in joules.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Joules(pub f64);

/// `E = C_H * nu`.
///
/// Accepts ordinary frequency only. `C_H = h/sqrt(2*pi)` is neither `h` nor
/// `hbar`, and this yields `0.3989x` the Planck energy `h*nu` — a deliberate
/// departure, not an approximation. Do not "correct" it toward Planck.
pub fn energy(nu: Frequency) -> Joules {
    Joules(HOWARD_COMMA * nu.get())
}

/// Frequency below which a process holds no energy and its memory is unmapped.
///
/// Garbage collection is a consequence of `E = C_H*nu`, not a separate policy:
/// as `nu -> 0`, `E -> 0`, and the vector is reclaimable.
pub const RECLAMATION_THRESHOLD: Frequency = Frequency::hertz(1e-30);

pub fn is_reclaimable(nu: Frequency) -> bool {
    nu.get().abs() <= RECLAMATION_THRESHOLD.get()
}
