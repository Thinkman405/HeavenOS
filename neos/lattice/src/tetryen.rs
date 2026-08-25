//! The Tetryen's node standing-wave envelope.
//!
//! Law: `_mkb/tetryen.md`'s "Node dynamics" — `ψ(r) = A·sinh(r/R)·e^(−r/R)`,
//! reducing to `A·sinh(r)·e^(−r)` in lattice-native units (`R = 1`).
//!
//! Previously implemented only once, inside `gui::renderer::Tetryen`, which
//! also owns the *geometric* construction of a Tetryen (four embedded
//! nodes, H4 isometries). This is the formula on its own, with no geometry
//! attached, so a crate that needs the envelope but not the geometry — and
//! critically, cannot depend on `gui` without creating a cycle, since `gui`
//! depends on it — has one real home to call rather than a second
//! transcription of the same law. `gui::renderer::Tetryen::node_amplitude`
//! now delegates here.

use crate::constants::LATTICE_SCALE_R;

/// `ψ(r) = A·sinh(r/R)·e^(−r/R)`, at `A = 1`.
///
/// Zero at `r = 0`: a node at its own centre has no amplitude. Positive and
/// finite for every finite `r >= 0`; the caller is responsible for `r`
/// being a real, non-negative separation (a geodesic distance, typically).
pub fn tetryen_node_envelope(r: f64) -> f64 {
    let x = r / LATTICE_SCALE_R;
    x.sinh() * (-x).exp()
}
