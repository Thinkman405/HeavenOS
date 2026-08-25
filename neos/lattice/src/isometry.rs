//! Isometries of the hyperbolic plane, in the hyperboloid (Minkowski) model.
//!
//! A point is `(x, y, t)` with `x² + y² − t² = −1`, `t > 0`. Isometries are
//! 3×3 matrices preserving that form — the group `O(2,1)` — so composition is
//! plain matrix multiplication and inversion is well conditioned.
//!
//! ## Why not Möbius transformations on the disk
//!
//! The Poincaré disk is the natural place to *look* at the tiling, but a poor
//! place to *compute* it: cells crowd exponentially toward `‖u‖ = 1`, so
//! coordinates lose absolute precision exactly where the tiling gets
//! interesting. In the hyperboloid model the coordinates grow instead of
//! crowding, and relative precision is preserved. We compute here and project
//! to the disk only for identity comparison, where the separation bound in
//! [`crate::tessellation`] makes the loss harmless.

/// An isometry of H², as a 3×3 matrix acting on the hyperboloid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Isometry([[f64; 3]; 3]);

/// A point on the hyperboloid: `x² + y² − t² = −1`, `t > 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HyperboloidPoint(pub [f64; 3]);

impl HyperboloidPoint {
    pub const ORIGIN: Self = Self([0.0, 0.0, 1.0]);

    /// Minkowski inner product `⟨u,v⟩ = uₓvₓ + u_yv_y − u_tv_t`.
    pub fn minkowski(&self, other: &Self) -> f64 {
        self.0[0] * other.0[0] + self.0[1] * other.0[1] - self.0[2] * other.0[2]
    }

    /// Geodesic distance: `cosh d = −⟨u,v⟩`.
    ///
    /// The argument is clamped at 1.0 from below — it is `≥ 1` analytically,
    /// but rounding can push it a few ulp under, and `acosh` of that is NaN.
    pub fn distance_to(&self, other: &Self) -> f64 {
        (-self.minkowski(other)).max(1.0).acosh()
    }

    /// Project to the Poincaré disk: `(x, y, t) → (x/(1+t), y/(1+t))`.
    ///
    /// Always lands strictly inside the unit disk, since `t ≥ 1` and
    /// `x² + y² = t² − 1 < (1 + t)²`.
    pub fn to_disk(&self) -> [f64; 2] {
        let [x, y, t] = self.0;
        [x / (1.0 + t), y / (1.0 + t)]
    }
}

impl Isometry {
    pub const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ]);

    /// Rotation about the origin by `theta`.
    pub fn rotation(theta: f64) -> Self {
        let (s, c) = theta.sin_cos();
        Self([[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]])
    }

    /// Translation along the x-axis by hyperbolic distance `d`.
    pub fn translation(d: f64) -> Self {
        let (c, s) = (d.cosh(), d.sinh());
        Self([[c, 0.0, s], [0.0, 1.0, 0.0], [s, 0.0, c]])
    }

    /// Reflection across the y-axis geodesic (`x → −x`).
    pub fn flip_x() -> Self {
        Self([[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    }

    /// Reflection across the geodesic perpendicular to the x-axis at distance
    /// `d` from the origin: `T(d) ∘ flip ∘ T(−d)`.
    ///
    /// Maps the origin to the point at distance `2d` along the x-axis, and is
    /// an involution — which is exactly what an edge crossing must be, so that
    /// stepping across an edge and back returns to where you started.
    pub fn reflection_at(d: f64) -> Self {
        Self::translation(d)
            .compose(&Self::flip_x())
            .compose(&Self::translation(-d))
    }

    /// Matrix product `self · rhs`. Applied to a point, `rhs` acts first.
    pub fn compose(&self, rhs: &Self) -> Self {
        let mut m = [[0.0; 3]; 3];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.0[i][k] * rhs.0[k][j]).sum();
            }
        }
        Self(m)
    }

    /// Conjugation `self · inner · self⁻¹` — re-expresses `inner` in the frame
    /// `self` maps to. Used to build a reflection across a rotated edge.
    pub fn conjugate(&self, inner: &Self) -> Self {
        self.compose(inner).compose(&self.inverse())
    }

    /// Inverse. For `O(2,1)` this is `J Mᵀ J` with `J = diag(1, 1, −1)`, which
    /// is exact up to rounding and needs no general matrix inversion.
    pub fn inverse(&self) -> Self {
        let m = &self.0;
        Self([
            [m[0][0], m[1][0], -m[2][0]],
            [m[0][1], m[1][1], -m[2][1]],
            [-m[0][2], -m[1][2], m[2][2]],
        ])
    }

    pub fn apply(&self, p: &HyperboloidPoint) -> HyperboloidPoint {
        let mut out = [0.0; 3];
        for (i, o) in out.iter_mut().enumerate() {
            *o = (0..3).map(|k| self.0[i][k] * p.0[k]).sum();
        }
        HyperboloidPoint(out)
    }

    /// Where this isometry sends the origin — the centre of the cell it names.
    pub fn origin_image(&self) -> HyperboloidPoint {
        self.apply(&HyperboloidPoint::ORIGIN)
    }

    pub fn as_matrix(&self) -> &[[f64; 3]; 3] {
        &self.0
    }
}
