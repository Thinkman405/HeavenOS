//! Tetryen construction and geodesic edge sampling.
//!
//! Contract §2 and §3.
//!
//! ## Edges are geodesics, and that is not a quality setting
//!
//! A point `p` lies on the geodesic between `u` and `v` exactly when
//! `d(u,p) + d(p,v) = d(u,v)`; off it the triangle inequality is strict. A
//! Euclidean chord misses by `~3e-3`, five orders of magnitude above the
//! numerical floor, so the distinction is sharp and testable.
//!
//! There is deliberately no straight-line interpolator here. Lerping ball
//! coordinates is the defect, and it is the natural thing to write by accident.

use crate::ball::{BallIsometry, H4Point};
use crate::GuiError;
use lattice::PoincarePoint;

/// Four spatial directions forming a regular tetrahedron.
const TETRA_DIRS: [[f64; 4]; 4] = [
    [1.0, 1.0, 1.0, 0.0],
    [1.0, -1.0, -1.0, 0.0],
    [-1.0, 1.0, -1.0, 0.0],
    [-1.0, -1.0, 1.0, 0.0],
];

/// An edge rendered along the hyperbolic geodesic between two points.
#[derive(Debug, Clone, PartialEq)]
pub struct GeodesicEdge {
    from: PoincarePoint,
    to: PoincarePoint,
    length: f64,
}

impl GeodesicEdge {
    /// # Errors
    /// [`GuiError::DegenerateEdge`] if the endpoints coincide. The tangent
    /// construction divides by `sinh(d)`; without this guard that is a silent
    /// `NaN` propagating into the scene.
    pub fn new(from: PoincarePoint, to: PoincarePoint) -> Result<Self, GuiError> {
        let length = from.distance_to(&to);
        if length <= 0.0 || !length.is_finite() {
            return Err(GuiError::DegenerateEdge);
        }
        Ok(Self { from, to, length })
    }

    pub fn from_point(&self) -> &PoincarePoint {
        &self.from
    }

    pub fn to_point(&self) -> &PoincarePoint {
        &self.to
    }

    /// Geodesic length, from `lattice`'s metric.
    pub fn length(&self) -> f64 {
        self.length
    }

    /// The point a given fraction along the **geodesic**.
    ///
    /// Computed in the hyperboloid model as `gamma(t) = u cosh t + w sinh t`,
    /// with `w` the unit tangent at `u` toward `v`. This is not interpolation
    /// of ball coordinates - that would be the straight edge the contract
    /// forbids.
    pub fn point_at(&self, fraction: f64) -> Result<PoincarePoint, GuiError> {
        let u = H4Point::from_ball(&self.from);
        let v = H4Point::from_ball(&self.to);
        let d = u.distance_to(&v);
        if d <= 0.0 || !d.is_finite() {
            return Err(GuiError::DegenerateEdge);
        }
        let (cd, sd) = (d.cosh(), d.sinh());
        let mut w = [0.0; 5];
        for i in 0..5 {
            w[i] = (v.0[i] - u.0[i] * cd) / sd;
        }
        let t = d * fraction;
        let (ct, st) = (t.cosh(), t.sinh());
        let mut out = [0.0; 5];
        for i in 0..5 {
            out[i] = u.0[i] * ct + w[i] * st;
        }
        H4Point(out).normalise().to_ball()
    }

    /// Sample the edge, endpoints included.
    pub fn sample(&self, segments: usize) -> Result<Vec<PoincarePoint>, GuiError> {
        (0..=segments)
            .map(|i| self.point_at(i as f64 / segments as f64))
            .collect()
    }

    /// Geodesic-membership residual: `d(u,p) + d(p,v) - d(u,v)`.
    ///
    /// Zero on the geodesic, strictly positive off it. Exposed so callers and
    /// tests can assert membership directly instead of inspecting coordinates.
    pub fn deviation_at(&self, p: &PoincarePoint) -> f64 {
        self.from.distance_to(p) + p.distance_to(&self.to) - self.length
    }
}

/// The core rendering primitive: four nodes, six geodesic edges.
///
/// # `E[Gamma]` is not minimised
///
/// `_mkb/tetryen.md` characterises the Tetryen as the minimiser of
/// `E[Gamma] = integral(K(s) + H(s)^2) ds`. This type **constructs** a shape
/// satisfying that characterisation - regular, with geodesic edges - it does
/// not solve the variational problem. Computing that minimum over a curved
/// surface is a research problem, not a rendering slice.
#[derive(Debug, Clone, PartialEq)]
pub struct Tetryen {
    nodes: [PoincarePoint; 4],
    circumradius: f64,
}

impl Tetryen {
    /// A Tetryen centred at the origin.
    ///
    /// # Errors
    /// [`GuiError::InvalidRadius`] for a non-positive or non-finite radius.
    pub fn new(circumradius: f64) -> Result<Self, GuiError> {
        if !(circumradius > 0.0) || !circumradius.is_finite() {
            return Err(GuiError::InvalidRadius { r: circumradius });
        }
        let (s, c) = (circumradius.sinh(), circumradius.cosh());
        let mut nodes = Vec::with_capacity(4);
        for dir in TETRA_DIRS {
            let n: f64 = dir.iter().map(|x| x * x).sum::<f64>().sqrt();
            let h = H4Point([
                dir[0] / n * s,
                dir[1] / n * s,
                dir[2] / n * s,
                dir[3] / n * s,
                c,
            ]);
            nodes.push(h.to_ball()?);
        }
        Ok(Self {
            nodes: [nodes[0], nodes[1], nodes[2], nodes[3]],
            circumradius,
        })
    }

    /// A Tetryen centred elsewhere, by translating one built at the origin.
    ///
    /// Regularity is a property of the shape, not of its position - the
    /// translation is an isometry, so it survives.
    pub fn at(centre: &PoincarePoint, circumradius: f64) -> Result<Self, GuiError> {
        let base = Self::new(circumradius)?;
        let c = centre.coords();
        let norm: f64 = c.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm == 0.0 {
            return Ok(base);
        }
        let distance = PoincarePoint::origin().distance_to(centre);
        let iso = BallIsometry::translation(*c, distance)?;
        let moved: Result<Vec<_>, _> = base.nodes.iter().map(|n| iso.apply_ball(n)).collect();
        let moved = moved?;
        Ok(Self {
            nodes: [moved[0], moved[1], moved[2], moved[3]],
            circumradius,
        })
    }

    /// Exactly four. The array type carries the count, because a Tetryen with
    /// three or five nodes is not a degenerate Tetryen - it is not one.
    pub fn nodes(&self) -> &[PoincarePoint; 4] {
        &self.nodes
    }

    pub fn circumradius(&self) -> f64 {
        self.circumradius
    }

    /// The six edges, every one a geodesic.
    pub fn edges(&self) -> Result<[GeodesicEdge; 6], GuiError> {
        let pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        let mut out = Vec::with_capacity(6);
        for (i, j) in pairs {
            out.push(GeodesicEdge::new(self.nodes[i], self.nodes[j])?);
        }
        Ok([
            out[0].clone(),
            out[1].clone(),
            out[2].clone(),
            out[3].clone(),
            out[4].clone(),
            out[5].clone(),
        ])
    }

    /// Common edge length. Equal for all six by construction.
    pub fn edge_length(&self) -> f64 {
        self.nodes[0].distance_to(&self.nodes[1])
    }

    /// Largest deviation among the six edge lengths. Zero for a regular shape.
    pub fn edge_spread(&self) -> f64 {
        let pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        let ds: Vec<f64> = pairs
            .iter()
            .map(|(i, j)| self.nodes[*i].distance_to(&self.nodes[*j]))
            .collect();
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for d in ds {
            lo = lo.min(d);
            hi = hi.max(d);
        }
        hi - lo
    }

    pub fn is_regular(&self, tol: f64) -> bool {
        self.edge_spread() <= tol
    }

    /// Node standing wave `psi(r) = A sinh(r/R) e^(-r/R)`, at `A = 1`.
    ///
    /// Zero at `r = 0`: a node at the centre has no amplitude.
    ///
    /// Delegates to `lattice::tetryen_node_envelope` — the formula's one
    /// home, since `crystallisation` needs it too and cannot depend on
    /// `gui` (the dependency runs the other way).
    pub fn node_amplitude(&self, r: f64) -> f64 {
        lattice::tetryen_node_envelope(r)
    }
}
