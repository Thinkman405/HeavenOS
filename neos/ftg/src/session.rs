//! §7 — connection lifecycle as physical resonance.
//!
//! Contract §4. A connection is not a state record; it is two oscillators
//! locked into a standing wave. Teardown is not a message; it is the amplitude
//! reaching zero.

use crate::constants::{LINK_LOCK_BOUND, TEARDOWN_PHASE_SHIFT};
use crate::FtgError;

/// One end of a link.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oscillator {
    pub phase: f64,
    pub amplitude: f64,
}

impl Oscillator {
    pub const fn new(phase: f64, amplitude: f64) -> Self {
        Self { phase, amplitude }
    }
}

/// Where a link is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkState {
    /// Not yet synchronised.
    Idle,
    /// Locked into a shared standing wave.
    Resonant { sync_phase: f64 },
    /// Torn down. **Terminal** - a link that has reached amplitude zero cannot
    /// resonate again, and reusing one would contradict the physics the design
    /// rests on.
    Collapsed,
}

/// Two oscillators and the state between them.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    a: Oscillator,
    b: Oscillator,
    state: LinkState,
}

impl Link {
    /// Attempt the Resonant Handshake, replacing SYN/ACK.
    ///
    /// Locks when phase variance is **strictly below** `pi/4`. The bound is
    /// exclusive - verified at the boundary.
    ///
    /// # Errors
    /// [`FtgError::NoLock`] when the oscillators are too far apart to resonate.
    pub fn attempt_handshake(a: Oscillator, b: Oscillator) -> Result<Self, FtgError> {
        let variance = (a.phase - b.phase).abs();
        if variance >= LINK_LOCK_BOUND {
            return Err(FtgError::NoLock { variance });
        }
        Ok(Self {
            a,
            b,
            state: LinkState::Resonant {
                sync_phase: (a.phase + b.phase) / 2.0,
            },
        })
    }

    pub fn state(&self) -> LinkState {
        self.state
    }

    pub fn is_resonant(&self) -> bool {
        matches!(self.state, LinkState::Resonant { .. })
    }

    pub fn phase_variance(&self) -> f64 {
        (self.a.phase - self.b.phase).abs()
    }

    /// The established session waveform: `f(t) = 2A sin(kx) cos(w_sync t)`.
    pub fn standing_wave(&self, k: f64, x: f64, omega_sync: f64, t: f64) -> f64 {
        let amp = (self.a.amplitude + self.b.amplitude) / 2.0;
        2.0 * amp * (k * x).sin() * (omega_sync * t).cos()
    }

    /// Combined amplitude of the two ends at instant `t`, on a carrier of
    /// angular frequency `omega`.
    pub fn superposition(&self, omega: f64, t: f64) -> f64 {
        self.a.amplitude * (omega * t + self.a.phase).cos()
            + self.b.amplitude * (omega * t + self.b.phase).cos()
    }

    /// Phase Inversion Teardown, replacing FIN/ACK.
    ///
    /// Shifts one end by exactly `pi`, forcing combined amplitude to zero.
    /// Returns the residual so a caller can assert on the real value rather
    /// than trusting the call happened.
    ///
    /// # Errors
    /// [`FtgError::Collapsed`] if the link is already torn down.
    pub fn teardown(&mut self, omega: f64, t: f64) -> Result<f64, FtgError> {
        if self.state == LinkState::Collapsed {
            return Err(FtgError::Collapsed);
        }
        self.b.phase = self.a.phase + TEARDOWN_PHASE_SHIFT;
        self.b.amplitude = self.a.amplitude;
        let residual = self.superposition(omega, t).abs();
        self.state = LinkState::Collapsed;
        Ok(residual)
    }

    /// Re-check an established link. Variance drifting to `>= pi/4` means the
    /// link has lost resonance and must be torn down.
    pub fn still_locked(&self) -> bool {
        self.is_resonant() && self.phase_variance() < LINK_LOCK_BOUND
    }

    /// Enforce the automatic-teardown policy `equations.md`'s Standing Wave
    /// Superposition execution rule mandates: *"Any phase variance exceeding
    /// `+-pi/4` triggers automatic phase inversion and teardown."*
    ///
    /// This is the piece [`still_locked`](Self::still_locked) and
    /// [`drift_to`](Self::drift_to) deliberately do not do on their own —
    /// both stay pure measurement, so detection cannot silently double as
    /// resolution. This method **is** the resolution: "whatever drives the
    /// link" (the sole caller today is [`crate::transport::Gateway`]'s
    /// session-gated delivery) calls this explicitly, and only here does a
    /// drifted link actually collapse.
    ///
    /// Before this existed, a link that drifted past the bound was reported
    /// as lost (`still_locked` returning `false`) while remaining internally
    /// `Resonant` — the caller's own `Link` value never reflected what had
    /// happened to it. Calling this closes that gap: after it returns
    /// `false`, [`state`](Self::state) is genuinely
    /// [`LinkState::Collapsed`], not merely "reported as unusable."
    ///
    /// Idempotent on an already-collapsed link — not by a separate check
    /// here, but because [`teardown`](Self::teardown) itself already refuses
    /// cleanly on one, returning `Err` before mutating anything. Verified by
    /// sabotage rather than assumed: removing an earlier explicit
    /// short-circuit in this method changed no test's outcome, which is what
    /// "already handled one level down" looks like when it's true.
    ///
    /// Returns `true` if the link is resonant and locked after this call —
    /// nothing needed to happen. Returns `false` if it is now collapsed,
    /// whether that is because it arrived collapsed or because drift forced
    /// a teardown here.
    pub fn enforce_lock(&mut self, omega: f64, t: f64) -> bool {
        if self.still_locked() {
            return true;
        }
        // Resonant in name only: variance has reached the bound. Tear down
        // for real rather than leaving the mismatch between reported and
        // actual state. A no-op `Err` if this link was already collapsed -
        // `teardown` guards that case itself.
        let _ = self.teardown(omega, t);
        false
    }

    /// Let the far oscillator drift to a given phase variance.
    ///
    /// Independent oscillators drift; a link that locked at admission does not
    /// stay locked for free. This does **not** tear the link down on its own -
    /// [`still_locked`](Self::still_locked) reports the loss and the caller
    /// decides, keeping detection separate from resolution. The resolution
    /// itself is [`enforce_lock`](Self::enforce_lock).
    pub fn drift_to(&mut self, variance: f64) {
        self.b.phase = self.a.phase + variance;
    }
}

/// Superposition of two unit oscillators at phases `a` and `b`, on carrier
/// `omega`, at instant `t`.
///
/// This is the raw form behind **Test Case 1**: at `a = 0`, `b = pi`, the sum
/// is zero for all `t`, not merely at sampled instants.
pub fn superpose(a: f64, b: f64, omega: f64, t: f64) -> f64 {
    (omega * t + a).cos() + (omega * t + b).cos()
}

/// The amplitude below which pi-opposed oscillators count as fully cancelled.
///
/// # This is not a constant, and assuming it is will produce a flaky test
///
/// Analytically `cos(x)` and `cos(x + pi)` are exact negatives, so the sum is
/// zero. In IEEE-754 it is not: **`x + pi` rounds**, and the absolute rounding
/// error grows with `|x|`. Since `|d(cos)/dx| <= 1`, that error transfers to
/// the result roughly one-for-one, giving
///
/// ```text
/// residual ~ eps * |omega * t|
/// ```
///
/// Measured: `0.0` at `omega*t ~ 6`, but `5.55e-16` at `omega*t ~ 12`, and a
/// peak of `1.08e-14` sweeping 40 periods. A fixed floor picked from one sample
/// passes there and fails elsewhere - which is exactly what happened while
/// building this module.
///
/// The factor of 4 is headroom over the observed peak. `1.0 +` keeps the bound
/// meaningful as `t -> 0`, where the residual is genuinely zero.
///
/// This satisfies the doctrine's requirement to justify a tolerance against the
/// scale in play rather than reaching for `f64::EPSILON` unexamined.
pub fn cancellation_floor(omega: f64, t: f64) -> f64 {
    4.0 * f64::EPSILON * (1.0 + (omega * t).abs())
}
