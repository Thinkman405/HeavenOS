//! Text and code into sequential harmonic nodes.
//!
//! PRD §8: "Linear character strings are converted into sequential harmonic
//! nodes. Line breaks or code structures trigger bifurcation events, rendering
//! text documents as navigable 3D polymer-like fractals."
//!
//! ## Documents are shallow, for the same reason addresses are
//!
//! Each line break is an A1 bifurcation, scaling extent by **self**-`(x)`
//! (`extent <- extent (x) extent`, not a unit step) — squaring the product
//! each time rather than incrementing it, so this leaves `(x)`'s domain one
//! step sooner than the unit-step iteration `lattice`'s curved addressing
//! uses. Real values, confirmed directly rather than estimated:
//!
//! ```text
//! break 1 -> 2.0
//! break 2 -> 20.970562748477143
//! break 3 -> ~1.07e177
//! break 4 -> REFUSED
//! ```
//!
//! **Exactly three line breaks**, not four — [`Crystal::max_bifurcations`] is
//! computed from the real operator rather than hardcoded, precisely so this
//! number tracks the law instead of a stale guess about it. Same systemic
//! cause as the unit-step ceiling elsewhere in this workspace (`(x)` grows
//! super-exponentially against a fixed domain, so **every** subsystem that
//! iterates it inherits some limit), but a different arity gives a different
//! number — the two ceilings must not be conflated.
//!
//! An over-deep document is **refused**, never truncated - losing content
//! silently would be worse than declining to crystallise it.

use crate::CrystalError;
use lattice::LatticeScalar;
use substrate::constants::{PHASE_FALSE, PHASE_TRUE};

/// One character, placed as a harmonic node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarmonicNode {
    pub codepoint: char,
    pub index: usize,
    /// Phase orientation, from the character's low bit (axiom A2's pair).
    pub phase: f64,
}

/// A crystallised document: a chain of harmonic nodes, branched at line breaks.
#[derive(Debug, Clone, PartialEq)]
pub struct Crystal {
    nodes: Vec<HarmonicNode>,
    bifurcations: usize,
    extent: f64,
}

impl Crystal {
    /// Crystallise text into harmonic nodes.
    ///
    /// Every non-newline character becomes one node, in order. Every newline is
    /// a bifurcation event scaling extent by `(x)`.
    ///
    /// # Errors
    /// [`CrystalError::TooDeep`] when the document has more line breaks than
    /// `(x)`'s domain allows. There is deliberately **no truncating variant**:
    /// the contract says refuse, not lose content.
    pub fn crystallise(text: &str) -> Result<Self, CrystalError> {
        let limit = Self::max_bifurcations();
        let breaks = text.matches('\n').count();
        if breaks > limit {
            return Err(CrystalError::TooDeep {
                bifurcations: breaks,
                limit,
            });
        }

        let mut nodes = Vec::new();
        let mut extent = LatticeScalar::new(1.0);
        let mut bifurcations = 0;

        for ch in text.chars() {
            if ch == '\n' {
                // A1: a bifurcation is a structural split, not a copy.
                extent = extent
                    .otimes(extent)
                    .map_err(|_| CrystalError::TooDeep {
                        bifurcations: bifurcations + 1,
                        limit,
                    })?;
                bifurcations += 1;
                continue;
            }
            nodes.push(HarmonicNode {
                codepoint: ch,
                index: nodes.len(),
                phase: if (ch as u32) & 1 == 1 {
                    PHASE_TRUE
                } else {
                    PHASE_FALSE
                },
            });
        }

        Ok(Self {
            nodes,
            bifurcations,
            extent: extent.get(),
        })
    }

    pub fn nodes(&self) -> &[HarmonicNode] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn bifurcations(&self) -> usize {
        self.bifurcations
    }

    /// Structural extent after all bifurcations. `1.0` for a single-line
    /// document, exactly `2.0` after one line break.
    pub fn extent(&self) -> f64 {
        self.extent
    }

    /// Recover the characters, in order. Line breaks are structure, not
    /// content, so they do not reappear.
    pub fn text(&self) -> String {
        self.nodes.iter().map(|n| n.codepoint).collect()
    }

    /// How many bifurcations `(x)` permits before leaving its domain.
    ///
    /// **Computed, not hardcoded** - iterating the real operator until it
    /// refuses. If the domain ever changes, this follows it.
    pub fn max_bifurcations() -> usize {
        let mut extent = LatticeScalar::new(1.0);
        for depth in 0..64 {
            match extent.otimes(extent) {
                Ok(next) => extent = next,
                Err(_) => return depth,
            }
        }
        64
    }
}
