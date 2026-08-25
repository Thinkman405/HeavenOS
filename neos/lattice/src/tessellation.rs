//! The {5,4} pentagonal tessellation of the hyperbolic plane.
//!
//! Schlafli {p,q} means *q* p-gons meet at each vertex — so {5,4} is pentagons,
//! **four** per vertex. `vACUUM_FLUX.pdf` says "five" in prose while writing
//! {5,4} in the same sentence; the notation won. See `_mkb/reconciliation.md` R3.
//!
//! That decision is now independently confirmed by the geometry itself:
//! [`Tiling::cells_at_vertex`] counts four, built from the group action alone
//! with no appeal to the constant.
//!
//! ## Where cell naming comes from
//!
//! Each cell is the image of the fundamental pentagon under an isometry, so a
//! cell is named by a **word in the five edge-reflection generators**. Two
//! words name the same cell exactly when their isometries agree on the origin
//! — this is the word problem for the tiling, and it is decided here by
//! geometric realisation rather than by combinatorial rewriting.
//!
//! That is sound because distinct cell centres are **provably separated** by
//! `2 × inradius ≈ 1.2537` in the hyperbolic metric (see
//! [`CENTRE_SEPARATION`]), which is enormous next to the accumulated rounding
//! error. See [`CellId`] for the quantisation and its safety margin.
//!
//! ## Dimension note
//!
//! {5,4} tessellates H², while the lattice is H⁴. The tiling therefore occupies
//! a 2-plane of the 4-ball, embedded with the two remaining coordinates zero.
//! A genuine 4D honeycomb would need a rank-4 Schlafli symbol, which the source
//! corpus does not supply.

use crate::constants::{EDGES_PER_CELL, SCHLAFLI, VERTEX_DEGREE};
use crate::isometry::{HyperboloidPoint, Isometry};
use crate::metric::{LatticeError, PoincarePoint};
use std::collections::HashMap;
use std::f64::consts::PI;

pub use crate::constants::{EDGES_PER_CELL as EDGES, SCHLAFLI as SYMBOL, VERTEX_DEGREE as DEGREE};

/// `(p-2)(q-2) > 4` is the hyperbolicity condition. Euclidean tilings give
/// exactly 4; spherical ones give less. {5,4} gives 6.
pub fn is_hyperbolic() -> bool {
    let (p, q) = SCHLAFLI;
    (p - 2) * (q - 2) > 4
}

/// Interior angle at each vertex: `2*pi/q`. For {5,4} this is exactly `pi/2`.
pub fn interior_angle() -> f64 {
    2.0 * PI / f64::from(SCHLAFLI.1)
}

/// Area of one cell by Gauss-Bonnet at `K = -1`: `(p-2)*pi - p*(interior angle)`.
///
/// For {5,4} this is exactly `pi/2`. A hyperbolic cell's area is fixed by its
/// angles alone, with no free scale — there is no Euclidean analogue.
pub fn cell_area() -> f64 {
    let (p, _) = SCHLAFLI;
    f64::from(p - 2) * PI - f64::from(p) * interior_angle()
}

/// Distance from cell centre to a **vertex**: `acosh(cot(pi/p) * cot(pi/q))`.
///
/// This is the hypotenuse of the fundamental right triangle, whose angles are
/// `pi/p` at the centre, `pi/q` at the vertex, and `pi/2` at the edge midpoint.
///
/// Necessarily **greater** than [`inradius`] — see [`half_edge_length`] for the
/// quantity this was previously confused with.
pub fn circumradius() -> f64 {
    let (p, q) = SCHLAFLI;
    ((PI / f64::from(p)).tan().recip() * (PI / f64::from(q)).tan().recip()).acosh()
}

/// Distance from cell centre to an **edge midpoint**: `acosh(cos(pi/q)/sin(pi/p))`.
///
/// The leg of the fundamental triangle opposite the vertex angle.
pub fn inradius() -> f64 {
    let (p, q) = SCHLAFLI;
    ((PI / f64::from(q)).cos() / (PI / f64::from(p)).sin()).acosh()
}

/// Half an edge, i.e. vertex to edge midpoint: `acosh(cos(pi/p)/sin(pi/q))`.
///
/// The leg opposite the centre angle. Related to the other two by the
/// hyperbolic Pythagorean theorem, `cosh(c) = cosh(a)cosh(b)`.
pub fn half_edge_length() -> f64 {
    let (p, q) = SCHLAFLI;
    ((PI / f64::from(p)).cos() / (PI / f64::from(q)).sin()).acosh()
}

/// Hyperbolic distance between the centres of two edge-adjacent cells:
/// `2 x inradius`.
///
/// This is also the **minimum** separation between any two distinct cell
/// centres in the tiling, which is what makes identity-by-centre sound.
pub fn centre_separation() -> f64 {
    2.0 * inradius()
}

/// Alias for documentation links.
pub use centre_separation as CENTRE_SEPARATION;

/// Quantisation scale for [`CellId`]: Poincare coordinates times 1e9, truncated.
///
/// Poincare coordinates live in `(-1, 1)`, so this keeps nine decimal places —
/// far finer than needed to separate cells (`2 x inradius ~ 1.2537` in the
/// hyperbolic metric) and far coarser than accumulated rounding error at any
/// depth this enumerator is used for.
const ID_SCALE: f64 = 1e9;

/// A deterministic name for a cell.
///
/// Derived from the cell centre projected to the Poincare disk and quantised.
/// Two words in the generators produce equal `CellId`s exactly when they name
/// the same cell — this is the decision procedure for the word problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellId(i64, i64);

impl CellId {
    fn from_point(p: &HyperboloidPoint) -> Self {
        let [x, y] = p.to_disk();
        Self((x * ID_SCALE).round() as i64, (y * ID_SCALE).round() as i64)
    }

    /// The cell containing the origin.
    pub const ORIGIN: Self = Self(0, 0);

    pub fn disk_coords(&self) -> [f64; 2] {
        [self.0 as f64 / ID_SCALE, self.1 as f64 / ID_SCALE]
    }
}

/// A pentagon of the tiling, carrying the isometry that produced it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    transform: Isometry,
}

impl Cell {
    /// The fundamental cell, centred at the origin.
    pub const fn at_origin() -> Self {
        Self {
            transform: Isometry::IDENTITY,
        }
    }

    pub const fn from_transform(transform: Isometry) -> Self {
        Self { transform }
    }

    pub fn transform(&self) -> &Isometry {
        &self.transform
    }

    pub fn centre(&self) -> HyperboloidPoint {
        self.transform.origin_image()
    }

    pub fn id(&self) -> CellId {
        CellId::from_point(&self.centre())
    }

    /// The centre as a point of the 4-ball, with the two unused coordinates
    /// zero. See the module-level dimension note.
    pub fn centre_in_ball(&self) -> Result<PoincarePoint, LatticeError> {
        let [x, y] = self.centre().to_disk();
        PoincarePoint::new([x, y, 0.0, 0.0])
    }

    /// Face-neighbours: one per edge, so `p` of them.
    ///
    /// Distinct from [`VERTEX_DEGREE`], which counts cells meeting at a vertex.
    pub const fn neighbor_count(&self) -> u32 {
        EDGES_PER_CELL
    }

    pub const fn cells_per_vertex(&self) -> u32 {
        VERTEX_DEGREE
    }

    /// The five neighbours, in edge order.
    ///
    /// Neighbour `k` is `self.transform · gen_k`, where `gen_k` is the
    /// reflection across edge `k` of the fundamental cell. Because each
    /// generator is an involution, `self.neighbors()[k].neighbors()[k]` is
    /// `self` again.
    pub fn neighbors(&self) -> [Cell; EDGES_PER_CELL as usize] {
        let gens = generators();
        std::array::from_fn(|k| Cell::from_transform(self.transform.compose(&gens[k])))
    }

    /// The single neighbour across edge `k`.
    pub fn neighbor(&self, edge: usize) -> Option<Cell> {
        (edge < EDGES_PER_CELL as usize)
            .then(|| Cell::from_transform(self.transform.compose(&generators()[edge])))
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::at_origin()
    }
}

/// The five edge-reflection generators of the fundamental cell.
///
/// `gen_k = R(2*pi*k/p) · reflect(inradius) · R(-2*pi*k/p)` — the reflection
/// across edge `k`, obtained by conjugating the x-axis edge reflection into
/// edge `k`'s frame.
///
/// **These must be reflections, not rotate-then-translate.** A translation
/// composed with a rotation about the *origin* is not an involution, so
/// stepping across an edge and back does not return; the enumeration then
/// unfolds into a free tree of 5^n cells instead of closing into a tiling.
pub fn generators() -> [Isometry; EDGES_PER_CELL as usize] {
    let (p, _) = SCHLAFLI;
    let reflect = Isometry::reflection_at(inradius());
    std::array::from_fn(|k| {
        let theta = 2.0 * PI * (k as f64) / f64::from(p);
        Isometry::rotation(theta).conjugate(&reflect)
    })
}

/// A breadth-first enumeration of the tiling out to a bounded depth.
///
/// Cells are deduplicated by [`CellId`], which decides word equality.
#[derive(Debug, Clone)]
pub struct Tiling {
    cells: HashMap<CellId, Cell>,
    layers: Vec<Vec<CellId>>,
}

impl Tiling {
    /// Grow the tiling out to `depth` rings around the origin cell.
    ///
    /// Cell count grows like `phi^2 ~ 2.618` per ring, so depth is the knob
    /// that keeps this bounded. Depth 7 is about 3,000 cells.
    pub fn grow(depth: usize) -> Self {
        let gens = generators();
        let origin = Cell::at_origin();
        let mut cells = HashMap::new();
        cells.insert(origin.id(), origin);

        let mut layers = vec![vec![origin.id()]];
        let mut frontier = vec![origin];

        for _ in 0..depth {
            let mut next = Vec::new();
            let mut next_ids = Vec::new();
            for cell in &frontier {
                for gen in &gens {
                    let child = Cell::from_transform(cell.transform.compose(gen));
                    let id = child.id();
                    if let std::collections::hash_map::Entry::Vacant(e) = cells.entry(id) {
                        e.insert(child);
                        next_ids.push(id);
                        next.push(child);
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            layers.push(next_ids);
            frontier = next;
        }

        Self { cells, layers }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn get(&self, id: &CellId) -> Option<&Cell> {
        self.cells.get(id)
    }

    pub fn contains(&self, id: &CellId) -> bool {
        self.cells.contains_key(id)
    }

    /// Cell counts per ring: `[1, 5, 15, 40, 105, ...]`.
    ///
    /// Ring `n` holds exactly `5 * Fib(2n)` cells, equivalently
    /// `a(n) = 3a(n-1) - a(n-2)`.
    pub fn layer_sizes(&self) -> Vec<usize> {
        self.layers.iter().map(|l| l.len()).collect()
    }

    pub fn layer(&self, n: usize) -> Option<&[CellId]> {
        self.layers.get(n).map(|v| v.as_slice())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.cells.values()
    }

    /// Cells meeting the given vertex, found by distance from their centres.
    ///
    /// For {5,4} this returns four — the tiling's own confirmation of
    /// [`VERTEX_DEGREE`], derived from the group action rather than read from
    /// the constant.
    pub fn cells_at_vertex(&self, vertex: &HyperboloidPoint, tol: f64) -> Vec<CellId> {
        let r = circumradius();
        let mut found: Vec<CellId> = self
            .cells
            .iter()
            .filter(|(_, c)| (c.centre().distance_to(vertex) - r).abs() < tol)
            .map(|(id, _)| *id)
            .collect();
        found.sort();
        found
    }

    /// The tiling cell whose centre is nearest a target point.
    ///
    /// Linear over the cells this tiling has grown — the same shape as
    /// [`cells_at_vertex`](Self::cells_at_vertex), which already does a
    /// distance-based scan rather than a generator-word walk. That is
    /// deliberate here too: `target` is an arbitrary point (see
    /// [`crate::addressing::AddressPath::resolved_point`]), not necessarily
    /// one reachable by any exact word in the generators, so "nearest" is the
    /// honest question — a word walk would have to first decide *which* word,
    /// which is exactly the thing not being invented.
    ///
    /// Cannot return `None` for any `Tiling` built through [`grow`](Self::grow):
    /// that always inserts at least the origin cell.
    pub fn nearest_cell(&self, target: &HyperboloidPoint) -> Option<CellId> {
        self.cells
            .iter()
            .min_by(|(_, a), (_, b)| {
                a.centre()
                    .distance_to(target)
                    .partial_cmp(&b.centre().distance_to(target))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id)
    }

    /// A vertex of the origin cell — the corner between edges 0 and 1.
    pub fn origin_cell_vertex() -> HyperboloidPoint {
        let (p, _) = SCHLAFLI;
        Isometry::rotation(PI / f64::from(p))
            .compose(&Isometry::translation(circumradius()))
            .origin_image()
    }
}
