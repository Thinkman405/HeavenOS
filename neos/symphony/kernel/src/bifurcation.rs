//! Axiom A1 (bifurcation) and A2 (phase logic).
//!
//! ## What is implemented, and what is a stub
//!
//! The law already determines the **arithmetic** of both axioms, so that is
//! implemented and tested. What awaits `symphony-lang` is only the *binding*:
//! what a runtime task is, and what a DSL condition is. Stubbing the arithmetic
//! too would hide work the MKB has already settled.
//!
//! - **A1** — fork multiplicity is `1 (x) 1 = 2`, computed with `lattice`'s
//!   operator rather than a local reimplementation. Fully implemented.
//! - **A2** — branch evaluation is phase alignment against `{-pi/2, +pi/2}`.
//!   Fully implemented.
//! - **Stubbed** — [`TaskModel`], the trait `symphony-lang` will implement to
//!   supply real tasks and conditions.

use crate::constants::{PHASE_FALSE, PHASE_TRUE, RESONANCE_BAND};
use crate::KernelError;
use lattice::LatticeScalar;

// ------------------------------------------------------------------ A1

/// The result of a fork under Lynchpin bifurcation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bifurcation {
    /// Number of child execution units.
    pub children: f64,
    /// Address-space scale factor.
    pub address_scale: f64,
}

/// Fork a unit of execution.
///
/// Axiom A1: `1 x 1 = 2`. Forking is a **structural geometric split**, not
/// scalar duplication — both the child count and the address space scale by
/// the modified product, not by copying.
///
/// Computed with [`LatticeScalar::otimes`] from the `lattice` crate. The
/// operator has exactly one home, and this is not it.
///
/// For `unit = 1` the result is exactly `2.0` — bit-exact, because
/// `sinh(arcsinh(1)) = 1` identically.
///
/// # Errors
/// [`KernelError::UndefinedScale`] if the unit is outside `(x)`'s domain.
pub fn fork(unit: f64) -> Result<Bifurcation, KernelError> {
    let u = LatticeScalar::new(unit);
    let split = u
        .otimes(u)
        .map_err(|_| KernelError::UndefinedScale { r: unit })?;
    Ok(Bifurcation {
        children: split.get(),
        address_scale: split.get(),
    })
}

/// The canonical unit fork: `1 (x) 1`, yielding exactly 2.
pub fn fork_unit() -> Bifurcation {
    fork(1.0).expect("the unit fork is always inside the domain")
}

// ------------------------------------------------------------------ A2

/// A logic state, as phase orientation rather than a boolean.
///
/// Axiom A2 deprecates `true`/`false`. There is deliberately no `From<bool>`
/// and no `Into<bool>`: a conversion would reintroduce the classical logic the
/// axiom removes, at whichever call site used it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// `-pi/2`
    Negative,
    /// `+pi/2`
    Positive,
}

impl Phase {
    pub fn radians(self) -> f64 {
        match self {
            Self::Negative => PHASE_FALSE,
            Self::Positive => PHASE_TRUE,
        }
    }

    /// Phase inversion: the exact `pi` shift. **Gate 2 of PRD section 3.**
    ///
    /// Law: `_mkb/gates.md` section 2. A2's two orientations are separated by
    /// exactly `pi`, which is exactly the shift Phase Inversion Teardown
    /// prescribes. The shift therefore maps the permitted set onto itself, and
    /// that closure is what makes it a gate rather than an escape from the
    /// axiom.
    ///
    /// **Total by construction** — it cannot fail and takes no domain check.
    ///
    /// Deliberately not written as `-phi`. For this set the two coincide
    /// numerically, but only because the orientations happen to be symmetric
    /// about zero; the law is the `pi` shift, and `-phi` is derived from
    /// nothing. `invert` is its own inverse.
    pub fn invert(self) -> Self {
        match self {
            Self::Negative => Self::Positive,
            Self::Positive => Self::Negative,
        }
    }

    /// Classify a phase angle.
    ///
    /// # Errors
    /// [`KernelError::UndefinedScale`] if the angle is not near either
    /// permitted orientation. A2 admits exactly two; anything else is not a
    /// logic state and must not be silently rounded into one.
    pub fn from_radians(phi: f64, tolerance: f64) -> Result<Self, KernelError> {
        if (phi - PHASE_TRUE).abs() <= tolerance {
            Ok(Self::Positive)
        } else if (phi - PHASE_FALSE).abs() <= tolerance {
            Ok(Self::Negative)
        } else {
            Err(KernelError::UndefinedScale { r: phi })
        }
    }
}

/// How two phases combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interference {
    /// Aligned phases reinforce — the branch is taken.
    Constructive,
    /// Opposed phases cancel — the branch is not taken.
    Destructive,
}

/// Evaluate a branch by phase alignment, not boolean comparison.
///
/// Aligned phases interfere constructively and the branch is taken; opposed
/// phases cancel and it is not. This is A2's replacement for `if (x == true)`.
pub fn evaluate_branch(condition: Phase, reference: Phase) -> Interference {
    if condition == reference {
        Interference::Constructive
    } else {
        Interference::Destructive
    }
}

/// Superposition amplitude of two unit waves at the given phases.
///
/// Exactly `0.0` for opposed phases — destructive cancellation is total, which
/// is what makes phase teardown work without an acknowledgement message.
pub fn superpose(a: Phase, b: Phase) -> f64 {
    a.radians().sin() + b.radians().sin()
}

// ------------------------------------------------------------------ gate 3

/// Whether two oscillators sustain a standing wave. **Gate 3 of PRD section 3.**
///
/// Law: `_mkb/gates.md` section 3, a synthesis of `xi(r)` (which multiplies
/// nominal frequency at observation scale) with the standing-wave `+-pi/4`
/// stability variance.
///
/// Each oscillator's **effective** frequency is `nu * xi(r)`. Their relative
/// phase drift over one period of the pair's mean effective frequency is
/// `2*pi*delta_nu/nu_bar`; holding that inside `pi/4` gives
///
/// ```text
/// |nu_A - nu_B| / mean(nu_A, nu_B) <= 1/8
/// ```
///
/// The band is [`RESONANCE_BAND`](crate::constants::RESONANCE_BAND), derived
/// rather than tuned, and **relative** because the derivation produces a
/// dimensionless ratio — not because a relative form was preferred.
///
/// This is not a comparison of scales. A2 admits no relational operator, so the
/// gate asks a physical question — would a standing wave between these two
/// survive? — and answers it two-valued.
///
/// # Errors
/// [`KernelError::UndefinedScale`] if either scale is outside `xi`'s domain, or
/// if the mean effective frequency is not positive and finite. `xi(r) -> 0` as
/// `r -> inf`, so a large enough scale drives the ratio to `0/0`; that is
/// refused rather than answered, exactly as `(x)` refuses its domain limit.
pub fn resonates(
    nu_a: f64,
    scale_a: f64,
    nu_b: f64,
    scale_b: f64,
) -> Result<Interference, KernelError> {
    let eff_a = nu_a * crate::resonance::xi(scale_a)?;
    let eff_b = nu_b * crate::resonance::xi(scale_b)?;
    let mean = (eff_a + eff_b) / 2.0;

    if !(mean > 0.0) || !mean.is_finite() {
        return Err(KernelError::UndefinedScale { r: mean });
    }

    let detuning = (eff_a - eff_b).abs() / mean;
    if detuning <= RESONANCE_BAND {
        Ok(Interference::Constructive)
    } else {
        Ok(Interference::Destructive)
    }
}

/// The detuning ratio itself, for callers that need the magnitude rather than
/// the gate outcome. Same refusals as [`resonates`].
pub fn detuning(nu_a: f64, scale_a: f64, nu_b: f64, scale_b: f64) -> Result<f64, KernelError> {
    let eff_a = nu_a * crate::resonance::xi(scale_a)?;
    let eff_b = nu_b * crate::resonance::xi(scale_b)?;
    let mean = (eff_a + eff_b) / 2.0;
    if !(mean > 0.0) || !mean.is_finite() {
        return Err(KernelError::UndefinedScale { r: mean });
    }
    Ok((eff_a - eff_b).abs() / mean)
}

// ------------------------------------------------------------------ stub

/// The runtime task model `symphony-lang` will supply.
///
/// **This is the genuine stub in this module.** The arithmetic of A1 and A2 is
/// settled and implemented above; what is not settled is what a task *is* and
/// what a condition *is* in the DSL. Those definitions unfreeze when
/// `symphony-lang` resumes.
///
/// Implementors will supply a task's frequency, its fork behaviour, and the
/// phase its conditions evaluate to. Nothing in the kernel depends on this
/// trait yet — it exists to fix the shape of the seam, not to be used.
pub trait TaskModel {
    /// Ordinary frequency of this task, driving `E = C_H * nu`.
    fn frequency(&self) -> f64;

    /// Phase this task's guard condition currently evaluates to.
    fn guard_phase(&self) -> Phase;

    /// Unit parameter for a fork. Defaults to the canonical `1`.
    fn fork_unit(&self) -> f64 {
        1.0
    }
}
