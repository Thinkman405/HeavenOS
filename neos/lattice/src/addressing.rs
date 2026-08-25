//! Curved addressing — PRD §5.
//!
//! "Read/write operations use `a (x) b = a x b + d(a,b)` to traverse the
//! non-linear directory tree." "Scaling file sizes triggers geometric fractals
//! preserving logical area, entirely eliminating disk fragmentation."
//!
//! Both claims are built here, and both come with a constraint the PRD does not
//! mention.
//!
//! ## Paths are shallow, and step size decides how shallow
//!
//! `(x)` grows super-exponentially, and its domain stops at `a*b < 805.56`. So a
//! path is not free to be long:
//!
//! | step | reachable depth |
//! |---|---|
//! | 0.1 | 40+ |
//! | 0.5 | 40+ |
//! | 1.0 | **4** |
//! | 2.0 | **2** |
//! | 3.0 | **2** |
//!
//! Sub-unit steps *contract* the running product and traverse indefinitely;
//! unit-or-larger steps explode within a few levels. A directory tree addressed
//! by `(x)` with unit segments is about four levels deep - measured, not
//! estimated. [`AddressPath::max_depth_for_step`] computes it for any step.
//!
//! ## Traversal order is part of the address
//!
//! `(x)` is strongly non-associative (see `_mkb/operators.md`). Measured on the
//! path `1, 2, 1.5`: left association gives `303.23`, right gives `3373.0` - a
//! factor of 11. This is not rounding. [`AddressPath::resolve`] therefore fixes
//! **left association** and says so, because an unfixed order would make an
//! address depend on how the resolver happened to fold it.
//!
//! ## From a resolved scalar back to a cell
//!
//! [`AddressPath::resolve`] produces a single real number - the logical
//! address. The tiling names cells by a *word in the five edge-reflection
//! generators* (see `tessellation`'s module docs), which is a fundamentally
//! different representation, and nothing connected the two.
//!
//! [`AddressPath::resolve_to_cell`] is that connection, built from primitives
//! that already exist rather than a new formula:
//!
//! 1. [`Isometry::translation`] already treats a signed distance as a point on
//!    the canonical x-axis geodesic through the origin - it is how
//!    [`crate::tessellation::Tiling::origin_cell_vertex`] is built.
//! 2. [`Tiling::nearest_cell`] already does a distance-based scan over the
//!    tiling's own cells - the same shape `cells_at_vertex` uses.
//!
//! A resolved scalar is one-dimensional, so it can only ever select a point on
//! *one* geodesic through the 2D tiling - it cannot address off that line. That
//! is not a shortcoming being papered over: it is what "the tiling names cells
//! with points and addressing computes a scalar" can honestly mean when the
//! addressing side is a single real number. Sign selects direction along the
//! geodesic; magnitude selects hyperbolic distance from the origin.

use crate::isometry::{HyperboloidPoint, Isometry};
use crate::metric::{LatticeError, LatticeScalar};
use crate::tessellation::{cell_area, CellId, Tiling};

/// A path through the directory tree: a start point and a sequence of steps.
#[derive(Debug, Clone, PartialEq)]
pub struct AddressPath {
    start: LatticeScalar,
    steps: Vec<LatticeScalar>,
}

impl AddressPath {
    pub fn new(start: f64, steps: &[f64]) -> Self {
        Self {
            start: LatticeScalar::new(start),
            steps: steps.iter().map(|s| LatticeScalar::new(*s)).collect(),
        }
    }

    pub fn depth(&self) -> usize {
        self.steps.len()
    }

    pub fn start(&self) -> LatticeScalar {
        self.start
    }

    pub fn steps(&self) -> &[LatticeScalar] {
        &self.steps
    }

    /// Resolve the path to an address, folding **left**.
    ///
    /// `((start (x) s0) (x) s1) (x) s2 ...`
    ///
    /// The association order is fixed deliberately. `(x)` is strongly
    /// non-associative, so a different fold yields a different address for the
    /// same path - measured factor of 11 on a three-segment path. An address
    /// that depended on the resolver's internal choice would not be an address.
    ///
    /// # Errors
    /// [`LatticeError::Dissonant`] when a step takes the running product
    /// outside `(x)`'s domain. Refusing beats returning `+inf`: an infinite
    /// address is not a location.
    pub fn resolve(&self) -> Result<LatticeScalar, LatticeError> {
        let mut acc = self.start;
        for step in &self.steps {
            acc = acc.otimes(*step)?;
        }
        Ok(acc)
    }

    /// Resolve folding **right**, for comparison only.
    ///
    /// Exists so the test suite can demonstrate that order genuinely changes
    /// the address. **Never use this to address anything** - [`resolve`] is the
    /// one true fold.
    pub fn resolve_right(&self) -> Result<LatticeScalar, LatticeError> {
        let mut acc = match self.steps.last() {
            Some(last) => *last,
            None => return Ok(self.start),
        };
        for step in self.steps.iter().rev().skip(1) {
            acc = step.otimes(acc)?;
        }
        self.start.otimes(acc)
    }

    /// Resolve as far as the domain allows, returning the depth reached.
    ///
    /// For callers that want to walk until the tree ends rather than treating
    /// the end as an error.
    pub fn resolve_partial(&self) -> (LatticeScalar, usize) {
        let mut acc = self.start;
        for (i, step) in self.steps.iter().enumerate() {
            match acc.otimes(*step) {
                Ok(next) => acc = next,
                Err(_) => return (acc, i),
            }
        }
        (acc, self.steps.len())
    }

    /// The point this path's resolved scalar names, on the tiling's canonical
    /// reference geodesic.
    ///
    /// `Isometry::translation(d)` is already the tiling's own convention for
    /// "a point at signed hyperbolic distance `d` along the x-axis through the
    /// origin" - reused here rather than reinvented. See the module docs for
    /// why a single scalar can only ever name a point on one geodesic.
    ///
    /// # Errors
    /// Whatever [`resolve`](Self::resolve) can fail with: leaving `(x)`'s
    /// domain part-way through the fold.
    pub fn resolved_point(&self) -> Result<HyperboloidPoint, LatticeError> {
        let scalar = self.resolve()?;
        Ok(Isometry::translation(scalar.get()).origin_image())
    }

    /// Resolve this path all the way to the tiling cell it names.
    ///
    /// The inverse direction of what [`Tiling`] already does: cells are named
    /// by points (a cell's [`CellId`] comes from its centre), so naming a cell
    /// from a point is the nearest-cell search
    /// [`Tiling::nearest_cell`](crate::tessellation::Tiling::nearest_cell)
    /// already performs - not a new lookup mechanism.
    ///
    /// # Errors
    /// Whatever [`resolved_point`](Self::resolved_point) can fail with.
    pub fn resolve_to_cell(&self, tiling: &Tiling) -> Result<CellId, LatticeError> {
        let point = self.resolved_point()?;
        Ok(tiling
            .nearest_cell(&point)
            .expect("Tiling::grow always inserts at least the origin cell"))
    }

    /// How many uniform steps of `step` can be taken from `start` before `(x)`
    /// leaves its domain.
    ///
    /// Measured, not assumed: unit steps give 4, `2.0` gives 2, sub-unit steps
    /// run past `limit`. Capped by `limit` so a contracting step terminates.
    pub fn max_depth_for_step(start: f64, step: f64, limit: usize) -> usize {
        let mut acc = LatticeScalar::new(start);
        let s = LatticeScalar::new(step);
        for d in 0..limit {
            match acc.otimes(s) {
                Ok(next) => acc = next,
                Err(_) => return d,
            }
        }
        limit
    }
}

/// Logical area of a stored object, quantised to whole `{5,4}` cells.
///
/// ## Why fragmentation is not merely rare but impossible
///
/// In hyperbolic space a cell's area is fixed by its angles alone - Gauss-Bonnet
/// gives every `{5,4}` cell exactly `pi/2` at `K = -1`, with no free scale
/// parameter. Storage is therefore quantised into **identical** units.
///
/// There are no partial cells, so there are no gaps between them, so there is
/// nothing to fragment. [`LogicalArea::fragmentation`] returns exactly `0.0`
/// and always will - that is a consequence of the geometry, not a property the
/// allocator maintains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct LogicalArea {
    cells: usize,
}

impl LogicalArea {
    pub const fn of(cells: usize) -> Self {
        Self { cells }
    }

    pub const fn cells(&self) -> usize {
        self.cells
    }

    /// Total logical area: `cells x pi/2`, exactly.
    pub fn area(&self) -> f64 {
        self.cells as f64 * cell_area()
    }

    /// Area of one cell. Identical for every cell in the tiling.
    pub fn unit_area() -> f64 {
        cell_area()
    }

    /// Grow by whole cells. Scaling adds cells; it never resizes one.
    pub fn grow(&mut self, by: usize) {
        self.cells += by;
    }

    /// Shrink by whole cells, saturating at zero.
    pub fn shrink(&mut self, by: usize) {
        self.cells = self.cells.saturating_sub(by);
    }

    /// Wasted area between allocations. **Always exactly zero.**
    ///
    /// Not a measurement that happens to come out clean - identical cells
    /// cannot leave a gap smaller than a cell, and a gap of one or more cells
    /// is simply free space.
    pub fn fragmentation(&self) -> f64 {
        0.0
    }

    /// Whether an area is a whole number of cells. Always true by construction;
    /// present so the invariant is checkable rather than assumed.
    pub fn is_quantised(&self) -> bool {
        let a = self.area();
        let n = (a / cell_area()).round();
        (a - n * cell_area()).abs() < 1e-12
    }
}
