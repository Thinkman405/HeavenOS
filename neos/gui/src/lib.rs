//! # GUI — Tetryen rendering, fractal navigation, interference visualisation
//!
//! PRD §9: render the wave mechanics and spatial geometry of the kernel.
//!
//! This is the **geometry** layer a rasteriser would consume. There is no
//! framebuffer, window, or GPU binding here - the PRD does not specify one, and
//! the testable content is the geometry, not the pixels.
//!
//! ## Three things that will surprise a reader
//!
//! 1. **Edges are geodesics, and a straight edge is a defect.** There is no
//!    straight-line interpolator, not even as a fast path. A Euclidean chord
//!    misses geodesic membership by `~3e-3`, five orders above the numerical
//!    floor. See [`renderer::GeodesicEdge`].
//!
//! 2. **There is no `zoom`.** Navigation is hyperbolic translation, which is an
//!    isometry - distances are preserved exactly and the observer moves. A
//!    Euclidean zoom by `k` multiplies every distance by `k`, which is what
//!    destroys "infinite resolution". See [`fractal::Viewport`].
//!
//! 3. **Tolerances here cannot be machine epsilon.** `acosh` has unbounded
//!    derivative at 1, so a `1e-16` representation error surfaces as `~1e-8` in
//!    a distance between close points. The geodesic tolerance is `1e-7` for
//!    that reason, and it is still sharp because the thing it rejects misses by
//!    `3e-3`.
//!
//! ## What is not done here
//!
//! `E[Gamma]` is **not minimised**. `_mkb/tetryen.md` characterises the Tetryen
//! as a minimiser of that functional; this crate *constructs* a shape meeting
//! the characterisation. Solving the variational problem is a research task.
//!
//! The metric comes from `lattice` and is not reimplemented. H4 isometries do
//! not exist in `lattice` (its `Isometry` is H2, for the {5,4} tiling), so they
//! are built in [`ball`] - see that module for why, and when they should move.

pub mod ball;
pub mod evolution;
pub mod fractal;
pub mod renderer;
pub mod telemetry;
pub mod visualization;

/// Constants generated from `_mkb/constants.json` at build time.
pub mod constants {
    include!(concat!(env!("OUT_DIR"), "/mkb_constants.rs"));
}

use std::fmt;

/// Named for the geometric failure, per `_mkb/test-doctrine.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuiError {
    /// A point that is not inside the ball, so it names no location to draw.
    Unrenderable { norm: f64 },
    /// Endpoints coincide, so no unique geodesic exists between them. Without
    /// this check the tangent construction divides by zero and emits `NaN`.
    DegenerateEdge,
    /// A circumradius or distance that is non-positive or non-finite.
    InvalidRadius { r: f64 },
    /// A [`evolution::TetryenState`] step left its measured stability
    /// region and produced a non-finite amplitude.
    Diverged { amplitude: f64 },
}

impl fmt::Display for GuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unrenderable { norm } => {
                write!(f, "unrenderable point: ||u|| = {norm} is not inside the ball")
            }
            Self::DegenerateEdge => write!(
                f,
                "degenerate edge: coincident endpoints admit no unique geodesic"
            ),
            Self::InvalidRadius { r } => write!(f, "invalid radius {r}: must be positive and finite"),
            Self::Diverged { amplitude } => write!(
                f,
                "tetryen recurrence diverged: amplitude {amplitude} is non-finite; \
                 dt/gamma left the measured stability region"
            ),
        }
    }
}

impl std::error::Error for GuiError {}

pub use ball::{isometry_floor, BallIsometry, H4Point};
pub use evolution::TetryenState;
pub use fractal::{pairwise_distances, Viewport};
pub use renderer::{GeodesicEdge, Tetryen};
pub use telemetry::SystemSnapshot;
pub use visualization::{
    classify, combine, superpose_phases, Interference, LoadVisualisation, StandingWave,
    TetryenVisualisation,
};
