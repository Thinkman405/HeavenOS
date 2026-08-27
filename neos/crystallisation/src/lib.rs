//! # Crystallisation — application data translation (PRD §8)
//!
//! Flat 1D and 2D media converted into native 3D/4D resonant shapes: text into
//! harmonic node chains, images into frequency maps on Tetryen faces, audio
//! into localised oscillators.
//!
//! A **sibling** of `ftg`, not its child. The PRD frames §8 as the gateway's
//! Layer 7, which invites the assumption that this needs transport. It does
//! not: crystallisation is a representation transform, and whether the result
//! then travels a network is `ftg`'s concern. Dependencies are `lattice` and
//! `substrate` only.
//!
//! ## Three things worth knowing before reading further
//!
//! 1. **Documents are about four line breaks deep.** Each break is an A1
//!    bifurcation scaling extent by `(x)`, which leaves its domain quickly. The
//!    *same* ceiling appears in `lattice`'s curved addressing — the constraint
//!    is systemic to iterating `(x)`, not local to either subsystem.
//!
//! 2. **The frequency map is a representation, not a summary.** Parseval holds
//!    and the transform round-trips; a lossy map would not represent the image.
//!
//! 3. **Volumetric time-crystals now have an operational definition** — see
//!    [`timecrystal`]. Takens delay embedding into 4D phase space, Floquet
//!    quasi-energies quantised by the Howard Comma, and `SO(3,1)` modulation
//!    preserving phase-space volume. The gap `01_derive` recorded is closed.
//!
//!    Its energy bound needs **joint** quantisation: rounding each mode
//!    independently overshoots the half-quantum floor by up to 36x, so the
//!    fundamental absorbs the residual.

pub mod codec;
pub mod holographic;
pub mod linguistic;
pub mod parallel;
pub mod resonant;
pub mod timecrystal;

/// Constants generated from `_mkb/constants.json` at build time.
pub mod constants {
    include!(concat!(env!("OUT_DIR"), "/mkb_constants.rs"));
}

use std::fmt;

/// Named for what the data did, per `_mkb/test-doctrine.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrystalError {
    /// More bifurcations than `(x)`'s domain allows. The document is refused
    /// rather than truncated — losing content silently is worse.
    TooDeep { bifurcations: usize, limit: usize },
    /// Coefficients do not divide evenly across a Tetryen's four faces.
    UnevenProjection { coefficients: usize },
    /// An empty media stream has no frequency at all.
    EmptyMedia,
    /// Pixel count does not match the stated dimensions.
    MalformedGrid {
        height: usize,
        width: usize,
        pixels: usize,
    },
    /// A modulation that does not preserve phase-space volume. Liouville's
    /// theorem requires `det = 1`; anything else creates or destroys
    /// information content.
    NonUnitary { determinant: f64 },
    /// The signal needs more `C_H` quanta than `f64` counts exactly.
    ///
    /// Not a limitation of the implementation: the Howard Comma is `~2.6e-34`,
    /// so a unit-amplitude tone needs `~2.5e35` quanta. Past `2^53` an added
    /// quantum does not change the total, and the quantisation would be
    /// pretend.
    EnergyExceedsQuantisation { required: f64, max: f64 },
    /// Bytes claim a format this crate does not decode: not `P5`/`P6` for
    /// images, or not `RIFF`/`WAVE`/PCM-with-a-supported-bit-depth for audio.
    UnrecognisedFormat,
    /// A media file's declared size does not match the bytes actually
    /// present — a truncated or corrupt file, refused rather than read past
    /// its own boundary.
    TruncatedMedia { expected: usize, got: usize },
    /// A video frame's dimensions do not match the frames before it. Every
    /// frame in a sequence must agree, or "one video" is not what the caller
    /// actually has.
    FrameSizeMismatch {
        expected_height: usize,
        expected_width: usize,
        height: usize,
        width: usize,
    },
    /// A [`timecrystal::TetryenRecurrence`] step left its measured stability
    /// region and produced a non-finite amplitude.
    Diverged { amplitude: f64 },
}

impl fmt::Display for CrystalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooDeep {
                bifurcations,
                limit,
            } => write!(
                f,
                "document has {bifurcations} bifurcations; (x) permits {limit}"
            ),
            Self::UnevenProjection { coefficients } => write!(
                f,
                "{coefficients} coefficients do not divide across four Tetryen faces"
            ),
            Self::EmptyMedia => write!(f, "empty media stream has no frequency"),
            Self::MalformedGrid {
                height,
                width,
                pixels,
            } => write!(
                f,
                "grid claims {height}x{width} but carries {pixels} pixels"
            ),
            Self::NonUnitary { determinant } => write!(
                f,
                "non-unitary modulation: det = {determinant}, Liouville requires 1"
            ),
            Self::EnergyExceedsQuantisation { required, max } => write!(
                f,
                "signal needs {required:.4e} C_H quanta; only {max:.4e} are exactly countable"
            ),
            Self::UnrecognisedFormat => {
                write!(f, "bytes do not declare a format this crate decodes")
            }
            Self::TruncatedMedia { expected, got } => write!(
                f,
                "media declares {expected} bytes but only {got} are present"
            ),
            Self::FrameSizeMismatch {
                expected_height,
                expected_width,
                height,
                width,
            } => write!(
                f,
                "frame is {height}x{width}, expected {expected_height}x{expected_width} \
                 to match the rest of the sequence"
            ),
            Self::Diverged { amplitude } => write!(
                f,
                "tetryen recurrence diverged: amplitude {amplitude} is non-finite; \
                 dt/gamma left the measured stability region"
            ),
        }
    }
}

impl std::error::Error for CrystalError {}

pub use codec::{decode_ppm, decode_wav, AudioSamples};
pub use holographic::{Complex, FaceProjection, FrequencyMap, PixelGrid};
pub use linguistic::{Crystal, HarmonicNode};
pub use parallel::{crystallize_images, crystallize_videos, embed_audio};
pub use resonant::ResonantChamber;
pub use timecrystal::{
    takens_embed, LorentzTransform, Mode, PhaseSpaceVector, TetryenRecurrence,
    VolumetricTimeCrystal,
};
