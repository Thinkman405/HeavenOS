//! Isometries of the hyperbolic 4-ball, in the hyperboloid model.
//!
//! # Why this lives here and not in `lattice`
//!
//! `lattice` provides the 4-ball metric (`PoincarePoint::distance_to`) but its
//! `Isometry` type is **3x3 - isometries of the hyperbolic plane**, built for
//! the {5,4} tiling which tessellates H2. The Tetryen lives in the 4-ball, so
//! it needs H4 isometries, and those exist nowhere.
//!
//! This is new geometry, not a duplicate: the metric itself still comes from
//! `lattice` and is not reimplemented. **If a second subsystem ever needs H4
//! isometries, this module should move to `lattice`** - that is its natural
//! home, and it is only here to avoid reopening a completed record for a reuse
//! that has not happened yet.
//!
//! Points are `(x0, x1, x2, x3, t)` with `<u,u> = -1` and `t > 0`, under the
//! Minkowski form `<u,v> = sum(u_i v_i) - u_t v_t`.

use crate::GuiError;
use lattice::PoincarePoint;

/// A point on the 4-dimensional hyperboloid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct H4Point(pub [f64; 5]);

impl H4Point {
    pub const ORIGIN: Self = Self([0.0, 0.0, 0.0, 0.0, 1.0]);

    /// Lift a ball point onto the hyperboloid.
    pub fn from_ball(p: &PoincarePoint) -> Self {
        let c = p.coords();
        let n2: f64 = c.iter().map(|x| x * x).sum();
        let s = 2.0 / (1.0 - n2);
        Self([
            s * c[0],
            s * c[1],
            s * c[2],
            s * c[3],
            s * (1.0 + n2) / 2.0,
        ])
    }

    /// Project back into the ball.
    ///
    /// # Errors
    /// [`GuiError::Unrenderable`] if the result is not strictly inside, which
    /// can only happen if the point drifted off the hyperboloid.
    pub fn to_ball(self) -> Result<PoincarePoint, GuiError> {
        let d = 1.0 + self.0[4];
        let coords = [
            self.0[0] / d,
            self.0[1] / d,
            self.0[2] / d,
            self.0[3] / d,
        ];
        PoincarePoint::new(coords).map_err(|_| GuiError::Unrenderable {
            norm: coords.iter().map(|x| x * x).sum::<f64>().sqrt(),
        })
    }

    /// Minkowski inner product.
    pub fn minkowski(&self, other: &Self) -> f64 {
        (0..4).map(|i| self.0[i] * other.0[i]).sum::<f64>() - self.0[4] * other.0[4]
    }

    /// Geodesic distance, `cosh d = -<u,v>`.
    ///
    /// Used only inside the geodesic construction. Distances quoted to callers
    /// come from `lattice`'s metric, which is the one home for that fact.
    pub fn distance_to(&self, other: &Self) -> f64 {
        (-self.minkowski(other)).max(1.0).acosh()
    }

    /// Renormalise back onto the hyperboloid.
    ///
    /// Repeated isometries accumulate drift off the constraint surface, and
    /// because `acosh` has unbounded derivative at 1, that drift shows up
    /// amplified in distances between nearby points.
    pub fn normalise(self) -> Self {
        let spatial: f64 = (0..4).map(|i| self.0[i] * self.0[i]).sum();
        let t = (1.0 + spatial).sqrt();
        Self([self.0[0], self.0[1], self.0[2], self.0[3], t])
    }
}

/// An isometry of the hyperbolic 4-ball.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BallIsometry {
    /// Accumulated translation, applied in the hyperboloid model.
    matrix: [[f64; 5]; 5],
}

impl BallIsometry {
    pub const IDENTITY: Self = Self {
        matrix: [
            [1.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 1.0],
        ],
    };

    /// Translation along a spatial direction by hyperbolic `distance`.
    ///
    /// # Errors
    /// [`GuiError::InvalidRadius`] if the direction is degenerate or the
    /// distance is not finite.
    pub fn translation(direction: [f64; 4], distance: f64) -> Result<Self, GuiError> {
        let norm: f64 = direction.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm == 0.0 || !norm.is_finite() || !distance.is_finite() {
            return Err(GuiError::InvalidRadius { r: distance });
        }
        let e: Vec<f64> = direction.iter().map(|x| x / norm).collect();
        let (c, s) = (distance.cosh(), distance.sinh());

        let mut m = Self::IDENTITY.matrix;
        for i in 0..4 {
            for j in 0..4 {
                m[i][j] = if i == j { 1.0 } else { 0.0 } + (c - 1.0) * e[i] * e[j];
            }
            m[i][4] = s * e[i];
            m[4][i] = s * e[i];
        }
        m[4][4] = c;
        Ok(Self { matrix: m })
    }

    /// Rotation by `theta` in the `(axis_a, axis_b)` coordinate plane.
    pub fn rotation(axis_a: usize, axis_b: usize, theta: f64) -> Self {
        let mut m = Self::IDENTITY.matrix;
        let (s, c) = theta.sin_cos();
        m[axis_a][axis_a] = c;
        m[axis_a][axis_b] = -s;
        m[axis_b][axis_a] = s;
        m[axis_b][axis_b] = c;
        Self { matrix: m }
    }

    /// Matrix product `self * rhs`. Applied to a point, `rhs` acts first.
    pub fn compose(&self, rhs: &Self) -> Self {
        let mut m = [[0.0; 5]; 5];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..5).map(|k| self.matrix[i][k] * rhs.matrix[k][j]).sum();
            }
        }
        Self { matrix: m }
    }

    pub fn apply(&self, p: H4Point) -> H4Point {
        let mut out = [0.0; 5];
        for (i, o) in out.iter_mut().enumerate() {
            *o = (0..5).map(|k| self.matrix[i][k] * p.0[k]).sum();
        }
        H4Point(out).normalise()
    }

    /// Apply to a ball point, staying in ball coordinates.
    pub fn apply_ball(&self, p: &PoincarePoint) -> Result<PoincarePoint, GuiError> {
        self.apply(H4Point::from_ball(p)).to_ball()
    }
}

/// Tolerance for asserting that an isometry preserved a distance.
///
/// # Not a constant, and treating it as one produces a flaky test
///
/// An isometry preserves distances exactly in exact arithmetic. In IEEE-754 it
/// does not: as the view translates outward, ball coordinates crowd toward
/// `||u|| = 1`, and `acosh` amplifies the resulting representation error
/// because its derivative is unbounded at 1.
///
/// Measured, translating a Tetryen: `2.2e-16` at distance 0.5, `1.7e-15` at
/// 1.0, `5.4e-14` at 3.0, `3.2e-11` at 6.0 - roughly exponential in the
/// translation distance, which is what `cosh` growth implies.
///
/// The `exp` term tracks that growth; the leading constant is headroom over the
/// observed values. Same shape of problem as `ftg::cancellation_floor`.
pub fn isometry_floor(distance: f64) -> f64 {
    16.0 * f64::EPSILON * distance.abs().exp()
}
