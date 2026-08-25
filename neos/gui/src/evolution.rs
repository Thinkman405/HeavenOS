//! Discrete time evolution of a Tetryen's four node amplitudes.
//!
//! Law: `_mkb/tetryen_recurrence.md`. Closes the undistilled corpus's
//! `ψ_{n+1} = f(ψ_n, ψ_{n-1})` placeholder — a synthesis, not a
//! distillation. See that file for the full derivation: why the uncoupled
//! step is an exact trig identity, why the coupling weight reuses
//! `Tetryen::node_amplitude` rather than inventing a new attenuation
//! function, and the measured (not proven in closed form) stability
//! region for `Δt`/`γ`.
//!
//! **What this deliberately does not do**: declare "emergence." No paper
//! and no `_mkb/` file defines an operational criterion for it — searched
//! directly before writing this module, not assumed absent. This type
//! only advances the state; interpreting it is left undone rather than
//! invented.

use crate::renderer::Tetryen;
use crate::GuiError;

/// A Tetryen's four node amplitudes, evolving in discrete time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TetryenState {
    current: [f64; 4],
    previous: [f64; 4],
}

impl TetryenState {
    /// Seed both time slices explicitly — a caller resuming from saved
    /// state, or one that wants non-zero initial discrete "velocity"
    /// (`current != previous`).
    pub const fn seeded(current: [f64; 4], previous: [f64; 4]) -> Self {
        Self { current, previous }
    }

    /// Seed both time slices with the same initial amplitudes — the
    /// natural "at rest" condition: `ψ₀ = ψ₋₁` means zero initial velocity
    /// in the discrete sense the recurrence uses.
    pub const fn at_rest(initial: [f64; 4]) -> Self {
        Self::seeded(initial, initial)
    }

    pub const fn amplitudes(&self) -> [f64; 4] {
        self.current
    }

    /// Advance one step of `_mkb/tetryen_recurrence.md`'s recurrence.
    ///
    /// # Errors
    /// [`GuiError::Diverged`] if this step left the recurrence's measured
    /// stability region (that file's own `Δt`/`γ` execution rule) and
    /// produced a non-finite amplitude. Refused rather than propagated —
    /// the state is left unchanged so a caller can inspect it or retry
    /// with different parameters.
    pub fn step(
        &mut self,
        tetryen: &Tetryen,
        omega: f64,
        dt: f64,
        gamma: f64,
    ) -> Result<[f64; 4], GuiError> {
        let nodes = tetryen.nodes();
        let mut next = [0.0; 4];

        for i in 0..4 {
            let mut coupling = 0.0;
            for j in 0..4 {
                if j == i {
                    continue;
                }
                let d = nodes[i].distance_to(&nodes[j]);
                let weight = tetryen.node_amplitude(d);
                coupling += weight * (self.current[j] - self.current[i]);
            }

            let uncoupled = 2.0 * (omega * dt).cos() * self.current[i] - self.previous[i];
            let value = uncoupled + gamma * dt * dt * coupling;

            if !value.is_finite() {
                return Err(GuiError::Diverged { amplitude: value });
            }
            next[i] = value;
        }

        self.previous = self.current;
        self.current = next;
        Ok(next)
    }
}
