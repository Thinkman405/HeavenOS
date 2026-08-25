//! Fractal navigation — moving the observer, never scaling the scene.
//!
//! Contract §4. "Infinite resolution scaling: zoom into localized data nodes
//! without pixelation."
//!
//! ## What that actually means
//!
//! Navigation is a **hyperbolic translation**, which is an isometry: every
//! distance in the scene is preserved exactly. Detail does not degrade because
//! nothing is being magnified - the observer is moving through the space.
//!
//! A Euclidean zoom by factor `k` multiplies every distance by `k`. That is a
//! different operation, not a cheaper one, and it is why [`Viewport`] has no
//! `zoom` method: offering one would invite exactly the thing that destroys the
//! property.

use crate::ball::{isometry_floor, BallIsometry};
use crate::GuiError;
use lattice::PoincarePoint;

/// The observer's position and orientation in the scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    transform: BallIsometry,
    travelled: f64,
}

impl Viewport {
    pub fn identity() -> Self {
        Self {
            transform: BallIsometry::IDENTITY,
            travelled: 0.0,
        }
    }

    /// Move the observer by `distance` along `direction`.
    ///
    /// # Errors
    /// [`GuiError::InvalidRadius`] for a degenerate direction or non-finite
    /// distance.
    pub fn translate(&mut self, direction: [f64; 4], distance: f64) -> Result<(), GuiError> {
        let step = BallIsometry::translation(direction, distance)?;
        self.transform = step.compose(&self.transform);
        self.travelled += distance.abs();
        Ok(())
    }

    /// Rotate the view in a coordinate plane.
    pub fn rotate(&mut self, axis_a: usize, axis_b: usize, theta: f64) {
        let step = BallIsometry::rotation(axis_a, axis_b, theta);
        self.transform = step.compose(&self.transform);
    }

    /// Where a scene point appears from here.
    pub fn project(&self, p: &PoincarePoint) -> Result<PoincarePoint, GuiError> {
        self.transform.apply_ball(p)
    }

    /// Total distance travelled, which sets the isometry tolerance.
    pub fn travelled(&self) -> f64 {
        self.travelled
    }

    /// Tolerance for asserting this viewport preserved a distance.
    ///
    /// Grows with distance travelled - see [`isometry_floor`]. A fixed bound
    /// taken from a short move fails on a long one.
    pub fn isometry_floor(&self) -> f64 {
        isometry_floor(self.travelled)
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::identity()
    }
}

/// Distances between every pair, for checking that a view change preserved them.
pub fn pairwise_distances(points: &[PoincarePoint]) -> Vec<f64> {
    let mut out = Vec::new();
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            out.push(points[i].distance_to(&points[j]));
        }
    }
    out
}

/// What a **Euclidean** zoom would do, for contrast only.
///
/// Provided so the test suite can demonstrate that scaling is a genuinely
/// different operation rather than a cheaper approximation of navigation.
/// **Never call this from rendering** - it is not a navigation primitive.
pub fn euclidean_scale_distances(distances: &[f64], factor: f64) -> Vec<f64> {
    distances.iter().map(|d| d * factor).collect()
}
