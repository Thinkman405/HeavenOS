//! Media as localised oscillators.
//!
//! PRD §8: "Media files act as localized oscillators **or** volumetric
//! time-crystals, driving physical vibrations and 4D spatial rotation."
//!
//! # Half of this sentence is built, and that is deliberate
//!
//! The **localised oscillator** reading has law: a sample stream carries a
//! dominant frequency, and `substrate` supplies the frequency types.
//!
//! The **volumetric time-crystal** reading has none. Unlike the Tetryen — which
//! was undefined until `Mathematical_Fra.pdf` supplied its energy functional —
//! no paper in the corpus defines a time-crystal. There is nothing to distil.
//!
//! So there is **no time-crystal type here**, not even a stub: a stub would
//! imply a semantics nobody has specified. See
//! `subsystems/crystallisation/01_derive/output/math-contract.md` §1.1, and
//! CLAUDE.md's prohibition on speculative math.
//!
//! "Driving physical vibrations" and "4D spatial rotation" rest on the
//! undefined half and are likewise absent.

use crate::CrystalError;
use substrate::Frequency;

/// A media stream as a localised oscillator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResonantChamber {
    frequency: Frequency,
    samples: usize,
}

impl ResonantChamber {
    /// Derive an oscillator from a sample stream.
    ///
    /// The dominant frequency is estimated by zero-crossing rate, which is
    /// cheap and adequate for a single tone. It is **not** a spectral estimate:
    /// a polyphonic stream has no single dominant frequency, and this reports
    /// the crossing rate rather than pretending otherwise.
    ///
    /// # Errors
    /// [`CrystalError::EmptyMedia`] for an empty stream, which has no frequency
    /// at all.
    pub fn from_samples(samples: &[f64], sample_rate: f64) -> Result<Self, CrystalError> {
        if samples.is_empty() || !(sample_rate > 0.0) {
            return Err(CrystalError::EmptyMedia);
        }
        let crossings = samples
            .windows(2)
            .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
            .count();
        // Two crossings per cycle.
        let hz = crossings as f64 * sample_rate / (2.0 * samples.len() as f64);
        Ok(Self {
            frequency: Frequency::hertz(hz),
            samples: samples.len(),
        })
    }

    /// Ordinary frequency, in hertz.
    ///
    /// Returns `substrate`'s [`Frequency`], never `AngularFrequency`, so a
    /// media rate cannot reach the wave-synthesis carrier path.
    pub fn frequency(&self) -> Frequency {
        self.frequency
    }

    pub fn sample_count(&self) -> usize {
        self.samples
    }

    /// Whether the chamber is silent — no zero crossings, so no oscillation.
    pub fn is_silent(&self) -> bool {
        self.frequency.get() == 0.0
    }
}
