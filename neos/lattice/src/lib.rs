//! # Lattice — hyperbolic 4D geometry and storage engine
//!
//! Implements NEOS axiom A3: Cartesian coordinate arrays are replaced by
//! non-Euclidean hyperbolic metric spaces.
//!
//! The governing law lives in `_mkb/`; this crate is downstream of it. In
//! particular see:
//!
//! - `_mkb/axioms.md` — A3, the spatial addressing override
//! - `_mkb/operators.md` — the (x) operator, its domain, its non-associativity
//! - `_mkb/reconciliation.md` — why each contested definition was chosen
//!
//! ## Two things that will surprise a reader
//!
//! 1. **(x) is not associative.** `(a (x) b) (x) c` and `a (x) (b (x) c)` do not
//!    merely round differently — they diverge completely. Never reorder a chain.
//!
//! 2. **Edge crossings are reflections, not translations.** A rotate-then-
//!    translate step is not an involution, so stepping across an edge and back
//!    does not return — and the tiling unfolds into a free tree of `5^n` cells
//!    instead of closing up. See [`tessellation::generators`].
//!
//! All values are in lattice-native units (`R = 1`, `K = -1`).

pub mod addressing;
pub mod isometry;
pub mod metric;
pub mod pathfinding;
pub mod tessellation;
pub mod tetryen;

/// Constants generated from `_mkb/constants.json` at build time.
///
/// Never edit these by hand and never retype a value from them into source —
/// the JSON is the single home for every number here.
pub mod constants {
    include!(concat!(env!("OUT_DIR"), "/mkb_constants.rs"));
}

pub use addressing::{AddressPath, LogicalArea};
pub use isometry::{HyperboloidPoint, Isometry};
pub use metric::{LatticeError, LatticeScalar, PoincarePoint};
pub use pathfinding::{shortest_distance, shortest_path};
pub use tessellation::{Cell, CellId, Tiling};
pub use tetryen::tetryen_node_envelope;
