//! # Symphony-kernel — scheduler, quantization, equilibrium
//!
//! Runs processes as energy states on a self-stabilising harmonic field,
//! rather than as time-sliced threads on a priority queue.
//!
//! Law lives in `_mkb/`; this crate is downstream of it:
//!
//! - `_mkb/resonance.md` — the three roles of the Howard Comma, and the field
//!   equation as load balancing
//! - `_mkb/reconciliation.md` R5a/R5b — why `C_H` and `xi(r)` are what they are
//!
//! ## Three things that will surprise a reader
//!
//! 1. **`C_H` is neither `h` nor `hbar`.** It is `h/sqrt(2*pi)`, and
//!    `E = C_H*nu` gives `0.3989x` the Planck energy. Deliberate, not an
//!    approximation error.
//!
//! 2. **`nu` and `omega` are different types on purpose.** They differ by
//!    `2*pi` and the units do not distinguish them, so the compiler does.
//!
//! 3. **Load equilibrium does not prevent deadlock.** It eliminates thrashing
//!    and bottlenecks. Circular waits on resource acquisition are orthogonal,
//!    so [`deadlock`] detects them separately — and the test suite asserts that
//!    a perfectly balanced field can still deadlock.
//!
//! ## What this crate does not do
//!
//! Tiling geometry and neighbour naming come from the `lattice` crate. Nothing
//! about {5,4} is recomputed here.
//!
//! The runtime task model — what a task *is*, what a DSL condition *is* — comes
//! from `symphony-lang`, which is deferred. [`bifurcation::TaskModel`] fixes
//! the shape of that seam. The *arithmetic* of axioms A1 and A2 is settled law
//! and is fully implemented, not stubbed.

pub mod bifurcation;
pub mod concurrent_resources;
pub mod deadlock;
pub mod equilibrium;
pub mod memory;
pub mod quantization;
pub mod resonance;
pub mod resources;
pub mod scheduler;

/// Constants generated from `_mkb/constants.json` at build time.
pub mod constants {
    include!(concat!(env!("OUT_DIR"), "/mkb_constants.rs"));
}

use std::fmt;

/// Named for the physical failure, per `_mkb/test-doctrine.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KernelError {
    /// Diffusion coupling at or beyond `2/lambda_max(L)` — the balancer would
    /// oscillate rather than converge.
    Unstable { alpha: f64, bound: f64 },
    /// A scale outside the domain of `xi`. Note `r == 0` is *valid* and
    /// returns the limit; this is for `r < 0` and NaN.
    UndefinedScale { r: f64 },
    /// The phase-error integral is growing instead of tending to zero: the
    /// clock domain has diverged.
    Diverged { residual: f64 },
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unstable { alpha, bound } => write!(
                f,
                "unstable coupling: alpha = {alpha} must be below the topology bound {bound}"
            ),
            Self::UndefinedScale { r } => {
                write!(f, "undefined scale r = {r}: xi requires r >= 0")
            }
            Self::Diverged { residual } => write!(
                f,
                "clock domain diverged: H(kappa) = {residual} is not tending to zero"
            ),
        }
    }
}

impl std::error::Error for KernelError {}

pub use bifurcation::{
    detuning, evaluate_branch, fork, fork_unit, resonates, Bifurcation, Interference, Phase,
};
pub use concurrent_resources::ConcurrentTracker;
pub use deadlock::WaitForGraph;
pub use equilibrium::{CoreTopology, LoadField};
pub use memory::ConcurrentPool;
pub use quantization::{energy, is_reclaimable, AngularFrequency, Frequency, Joules};
pub use resonance::{xi, DriftIntegrator};
pub use resources::{Acquired, ResourceError, ResourceId, ResourceTracker};
pub use scheduler::{SchedulePass, Scheduler, Task, TaskId};
