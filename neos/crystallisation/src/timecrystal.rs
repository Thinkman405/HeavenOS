//! Volumetric Time Crystals — the operational definition of PRD §8's second
//! reading of media.
//!
//! A VTC is a discrete spatiotemporal structure exhibiting periodic, non-thermal
//! motion in time while bound in a non-Euclidean 4D lattice. Three ingredients,
//! each with law behind it:
//!
//! 1. **Takens delay embedding** places a time series into 4D phase space:
//!    `X(t) = [s(t), s(t-tau), s(t-2tau), s(t-3tau)]`, one component per
//!    Tetryen vertex.
//! 2. **Howard Comma quantisation** discretises Floquet quasi-energies, with
//!    `C_H` standing where `hbar` normally would.
//! 3. **SO(3,1) pseudo-rotation** models modulation, preserving phase-space
//!    volume (Liouville) and the Minkowski form.
//!
//! ## The energy bound needs joint quantisation, and this is not optional
//!
//! The conservation law is
//!
//! ```text
//! | E_crystal - sum_k n_k (C_H * nu_k) |  <=  0.5 * C_H * nu_0
//! ```
//!
//! It does **not** say how the `n_k` are chosen, and the obvious choice —
//! rounding each mode independently — *violates it*. Measured on an
//! eight-harmonic signal: independent rounding leaves a residual of `1.04e-31`
//! against a floor of `1.32e-32`, exceeding it eightfold, with a worst case
//! 36x over.
//!
//! Independent errors accumulate; the bound is a *single* half-quantum. So the
//! harmonics are quantised freely and **the fundamental absorbs the residual**,
//! which brings it inside the floor by construction. Verified at `2.11e-33`.

use crate::constants::HOWARD_COMMA;
use crate::holographic::{FrequencyMap, PixelGrid};
use crate::CrystalError;
use substrate::Frequency;

/// A point in 4D phase space, one component per Tetryen vertex.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PhaseSpaceVector(pub [f64; 4]);

impl PhaseSpaceVector {
    /// The `(3,1)` Minkowski form: `x0^2 + x1^2 + x2^2 - x3^2`.
    ///
    /// Preserved by every [`LorentzTransform`], which is what makes those
    /// transforms the right model for modulation.
    pub fn minkowski_norm(&self) -> f64 {
        let v = self.0;
        v[0] * v[0] + v[1] * v[1] + v[2] * v[2] - v[3] * v[3]
    }

    pub fn components(&self) -> &[f64; 4] {
        &self.0
    }
}

/// Takens delay embedding of a time series into 4D phase space.
///
/// # Errors
/// [`CrystalError::EmptyMedia`] if the signal is too short for the delay, or
/// `tau` is zero — a zero delay collapses all four components onto the same
/// sample and embeds nothing.
pub fn takens_embed(signal: &[f64], tau: usize) -> Result<Vec<PhaseSpaceVector>, CrystalError> {
    if tau == 0 || signal.len() <= 3 * tau {
        return Err(CrystalError::EmptyMedia);
    }
    Ok((3 * tau..signal.len())
        .map(|i| {
            PhaseSpaceVector([
                signal[i],
                signal[i - tau],
                signal[i - 2 * tau],
                signal[i - 3 * tau],
            ])
        })
        .collect())
}

/// A proper Lorentz transform in `SO(3,1)`, modelling media modulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LorentzTransform([[f64; 4]; 4]);

impl LorentzTransform {
    pub const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// A boost along a spatial axis — time dilation of the media stream.
    pub fn boost(rapidity: f64, axis: usize) -> Self {
        let mut m = Self::IDENTITY.0;
        let (c, s) = (rapidity.cosh(), rapidity.sinh());
        m[axis][axis] = c;
        m[3][3] = c;
        m[axis][3] = s;
        m[3][axis] = s;
        Self(m)
    }

    /// A spatial rotation — pitch shift or filtering.
    pub fn rotation(theta: f64, a: usize, b: usize) -> Self {
        let mut m = Self::IDENTITY.0;
        let (s, c) = theta.sin_cos();
        m[a][a] = c;
        m[a][b] = -s;
        m[b][a] = s;
        m[b][b] = c;
        Self(m)
    }

    pub fn compose(&self, rhs: &Self) -> Self {
        let mut m = [[0.0; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..4).map(|k| self.0[i][k] * rhs.0[k][j]).sum();
            }
        }
        Self(m)
    }

    pub fn apply(&self, v: PhaseSpaceVector) -> PhaseSpaceVector {
        let mut out = [0.0; 4];
        for (i, o) in out.iter_mut().enumerate() {
            *o = (0..4).map(|k| self.0[i][k] * v.0[k]).sum();
        }
        PhaseSpaceVector(out)
    }

    /// Determinant. **Must be 1** — that is Liouville's theorem, and it is why
    /// a modulation cannot create or destroy information content.
    pub fn determinant(&self) -> f64 {
        let m = &self.0;
        let mut det = 0.0;
        for (j, sign) in [(0usize, 1.0), (1, -1.0), (2, 1.0), (3, -1.0)] {
            let mut minor = [[0.0; 3]; 3];
            for r in 1..4 {
                let mut cc = 0;
                for c in 0..4 {
                    if c == j {
                        continue;
                    }
                    minor[r - 1][cc] = m[r][c];
                    cc += 1;
                }
            }
            let d3 = minor[0][0] * (minor[1][1] * minor[2][2] - minor[1][2] * minor[2][1])
                - minor[0][1] * (minor[1][0] * minor[2][2] - minor[1][2] * minor[2][0])
                + minor[0][2] * (minor[1][0] * minor[2][1] - minor[1][1] * minor[2][0]);
            det += sign * m[0][j] * d3;
        }
        det
    }

    /// Whether this transform preserves phase-space volume.
    pub fn is_volume_preserving(&self, tol: f64) -> bool {
        (self.determinant() - 1.0).abs() <= tol
    }
}

/// Largest occupation number an `f64` represents exactly: `2^53`.
///
/// Beyond this, adding one quantum does not change the value, so the
/// quantisation stops being a quantisation. See
/// [`VolumetricTimeCrystal::max_quantisable_energy`].
pub const MAX_EXACT_OCCUPATION: f64 = 9_007_199_254_740_992.0;

/// One quantised Floquet mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mode {
    pub frequency: Frequency,
    /// Occupation number — how many `C_H * nu` quanta this mode holds.
    ///
    /// An `f64` rather than an integer type, deliberately: at realistic
    /// energies this exceeds `i64` by many orders (a unit-amplitude tone needs
    /// `2.5e35` quanta against an `i64` ceiling of `9.2e18`). It is still a
    /// whole number — [`VolumetricTimeCrystal::crystallise`] refuses any signal
    /// whose occupation would exceed [`MAX_EXACT_OCCUPATION`], so within the
    /// accepted range the integrality is exact.
    pub occupation: f64,
}

impl Mode {
    /// `E_k = n_k * C_H * nu_k`.
    pub fn energy(&self) -> f64 {
        self.occupation * HOWARD_COMMA * self.frequency.get()
    }
}

/// A media stream crystallised into a quantised 4D spatiotemporal structure.
#[derive(Debug, Clone, PartialEq)]
pub struct VolumetricTimeCrystal {
    nodes: Vec<PhaseSpaceVector>,
    fundamental: Frequency,
    modes: Vec<Mode>,
    input_energy: f64,
}

impl VolumetricTimeCrystal {
    /// Crystallise a time series.
    ///
    /// The spectrum comes from the **same DFT** the holographic pipeline uses —
    /// the signal is transformed as a `1 x N` grid, so there is one home for
    /// the transform rather than a second copy for audio.
    ///
    /// # Errors
    /// [`CrystalError::EmptyMedia`] for a signal too short to embed, or a
    /// non-positive sample rate.
    pub fn crystallise(
        signal: &[f64],
        sample_rate: f64,
        tau: usize,
    ) -> Result<Self, CrystalError> {
        if !(sample_rate > 0.0) {
            return Err(CrystalError::EmptyMedia);
        }
        let nodes = takens_embed(signal, tau)?;

        let grid = PixelGrid::new(1, signal.len(), signal.to_vec())?;
        let spectrum = FrequencyMap::transform(&grid);
        let n = signal.len();

        // Fundamental = the lowest non-DC bin.
        let nu0 = sample_rate / n as f64;
        let input_energy = grid.energy();

        // Mode energies from the spectrum, harmonics first.
        // The quantum is ~1e-33 J, so a macroscopic signal needs more quanta
        // than f64 counts exactly. Refuse rather than silently returning a
        // number that adding a quantum would not change.
        let fundamental_quantum = HOWARD_COMMA * nu0;
        let required = input_energy / fundamental_quantum;
        if required > MAX_EXACT_OCCUPATION {
            return Err(CrystalError::EnergyExceedsQuantisation {
                required,
                max: MAX_EXACT_OCCUPATION,
            });
        }

        let half = n / 2;
        let mut modes: Vec<Mode> = Vec::new();
        let mut harmonic_energy = 0.0;
        for k in 2..half.max(2) {
            let nu_k = nu0 * k as f64;
            let e_k = spectrum.coefficients()[k].magnitude_squared() / n as f64;
            let quantum = HOWARD_COMMA * nu_k;
            let occupation = (e_k / quantum).round();
            harmonic_energy += occupation * quantum;
            modes.push(Mode {
                frequency: Frequency::hertz(nu_k),
                occupation,
            });
        }

        // The fundamental absorbs the residual. Rounding every mode
        // independently accumulates error past the half-quantum bound; letting
        // mode 0 take up the remainder keeps it inside by construction.
        let n0 = ((input_energy - harmonic_energy) / fundamental_quantum).round();
        modes.insert(
            0,
            Mode {
                frequency: Frequency::hertz(nu0),
                occupation: n0,
            },
        );

        Ok(Self {
            nodes,
            fundamental: Frequency::hertz(nu0),
            modes,
            input_energy,
        })
    }

    /// Crystallise a **video**: a temporal sequence of spatial frames.
    ///
    /// Law: `_mkb/timecrystal.md` §5, a synthesis closing the video half of
    /// PRD §8's audio/video reading. `§§1-4` only ever embed a scalar time
    /// series; a frame sequence is not one. The composition:
    ///
    /// ```text
    /// s_k = PixelGrid::energy(frame_k)   -- already law, already
    ///                                        Parseval-verified
    /// ```
    ///
    /// reduces each frame to the one number `§§1-4`'s machinery already
    /// expects, and everything downstream of that reduction is
    /// [`crystallise`](Self::crystallise), **unmodified** — no new Takens
    /// embedding, no new Floquet formula, no new quantisation rule.
    ///
    /// # Why an iterator rather than `&[PixelGrid]`
    ///
    /// A frame is large; its energy is one `f64`. Taking `frames` by iterator
    /// means only the reduced per-frame scalar has to outlive the frame it
    /// came from — a caller streaming frames from disk one at a time never
    /// holds more than one decoded `PixelGrid` at once. Widening this to
    /// `&[PixelGrid]` would force materialising the whole sequence for no
    /// reason `§5` needs.
    ///
    /// # This is not a rescaling pipeline
    ///
    /// `_mkb/timecrystal.md` §5.3 is explicit: a realistic frame's raw pixel
    /// energy is roughly fifty orders of magnitude past `C_H`'s quantisable
    /// ceiling (`§2.4`), the same ceiling the audio-driven tests already work
    /// around by construction. This function does not invent a rescaling —
    /// there is no such mapping in the corpus. A caller wanting quantised
    /// video supplies frames already scaled the way the audio tests scale
    /// their signal; an unscaled sequence is refused via
    /// [`CrystalError::EnergyExceedsQuantisation`], not truncated or
    /// silently rescaled.
    ///
    /// # Errors
    /// [`CrystalError::FrameSizeMismatch`] if a frame's dimensions differ from
    /// the first frame's — "one video" implies every frame agrees. Otherwise
    /// whatever [`crystallise`](Self::crystallise) can fail with, applied to
    /// the reduced per-frame signal.
    pub fn crystallise_video<I>(frames: I, frame_rate: f64, tau: usize) -> Result<Self, CrystalError>
    where
        I: IntoIterator<Item = PixelGrid>,
    {
        let mut signal = Vec::new();
        let mut dims: Option<(usize, usize)> = None;
        for frame in frames {
            let (h, w) = (frame.height(), frame.width());
            match dims {
                None => dims = Some((h, w)),
                Some((eh, ew)) if eh == h && ew == w => {}
                Some((eh, ew)) => {
                    return Err(CrystalError::FrameSizeMismatch {
                        expected_height: eh,
                        expected_width: ew,
                        height: h,
                        width: w,
                    })
                }
            }
            signal.push(frame.energy());
        }
        Self::crystallise(&signal, frame_rate, tau)
    }

    /// The largest signal energy that can be exactly quantised at `nu0`.
    ///
    /// `2^53 * C_H * nu_0` — about `1.9e-17` J at 7.8 Hz. Above this the
    /// quantum falls below `f64` resolution and quantisation is a no-op, so
    /// [`crystallise`](Self::crystallise) refuses.
    pub fn max_quantisable_energy(fundamental: Frequency) -> f64 {
        MAX_EXACT_OCCUPATION * HOWARD_COMMA * fundamental.get()
    }

    /// What **independent** per-mode rounding would have left as a residual.
    ///
    /// Provided so the joint scheme can be compared against the naive one
    /// rather than asserted to be better. Independent errors accumulate; the
    /// bound is a single half-quantum.
    pub fn independent_rounding_residual(&self) -> f64 {
        self.modes
            .iter()
            .map(|m| 0.5 * HOWARD_COMMA * m.frequency.get())
            .sum()
    }

    pub fn nodes(&self) -> &[PhaseSpaceVector] {
        &self.nodes
    }

    pub fn modes(&self) -> &[Mode] {
        &self.modes
    }

    pub fn fundamental(&self) -> Frequency {
        self.fundamental
    }

    pub fn input_energy(&self) -> f64 {
        self.input_energy
    }

    /// Total quantised energy: `sum_k n_k * C_H * nu_k`.
    pub fn quantised_energy(&self) -> f64 {
        self.modes.iter().map(Mode::energy).sum()
    }

    /// How far the quantised total sits from the input energy.
    pub fn energy_residual(&self) -> f64 {
        (self.input_energy - self.quantised_energy()).abs()
    }

    /// The half-quantum floor, `0.5 * C_H * nu_0`.
    ///
    /// A transformation that moves total energy beyond this is **non-unitary**
    /// and fails the doctrine check.
    pub fn half_quantum_floor(&self) -> f64 {
        0.5 * HOWARD_COMMA * self.fundamental.get()
    }

    /// Whether quantisation stayed inside the half-quantum floor.
    pub fn is_energy_conserving(&self) -> bool {
        self.energy_residual() <= self.half_quantum_floor()
    }

    /// Apply a modulation.
    ///
    /// # Errors
    /// [`CrystalError::NonUnitary`] if the transform does not preserve
    /// phase-space volume. A modulation that changed the volume would be
    /// creating or destroying information content.
    pub fn modulate(&self, transform: &LorentzTransform) -> Result<Self, CrystalError> {
        if !transform.is_volume_preserving(1e-9) {
            return Err(CrystalError::NonUnitary {
                determinant: transform.determinant(),
            });
        }
        Ok(Self {
            nodes: self.nodes.iter().map(|v| transform.apply(*v)).collect(),
            fundamental: self.fundamental,
            modes: self.modes.clone(),
            input_energy: self.input_energy,
        })
    }

    /// Whether the embedded trajectory repeats with the given period, in
    /// samples — the Floquet condition `u(t + T) = u(t)`.
    ///
    /// `relative_tol` is a fraction of the trajectory's own amplitude, **not an
    /// absolute displacement**. Signals that quantise against `C_H` have
    /// amplitudes around `1e-15`, so an absolute tolerance of even `1e-9` would
    /// declare *every* period a match — the test would pass on noise.
    ///
    /// Same shape of trap as `ftg::cancellation_floor` and `gui`'s scale-free
    /// check: a threshold has to be expressed in the units of the thing it
    /// bounds.
    pub fn is_floquet_periodic(&self, period_samples: usize, relative_tol: f64) -> bool {
        if period_samples == 0 || self.nodes.len() <= period_samples {
            return false;
        }
        let scale = self
            .nodes
            .iter()
            .flat_map(|v| v.0.iter())
            .fold(0.0_f64, |m, c| m.max(c.abs()));
        if scale == 0.0 {
            return true; // a silent trajectory is trivially periodic
        }
        let tol = relative_tol * scale;
        self.nodes
            .iter()
            .zip(self.nodes.iter().skip(period_samples))
            .all(|(a, b)| (0..4).all(|k| (a.0[k] - b.0[k]).abs() <= tol))
    }
}

/// Discrete time evolution of a Tetryen-mapped phase-space state.
///
/// Law: `_mkb/tetryen_recurrence.md` — the same synthesis
/// `gui::TetryenState` implements. The one real difference between the two,
/// stated plainly rather than papered over: this crate has no Tetryen
/// *geometry* to compute a coupling weight from (`gui` owns that, and
/// `crystallisation` cannot depend on `gui` — the dependency runs the other
/// way). The coupling weight is therefore supplied by the caller, computed
/// from whichever real geometry it has on hand — typically
/// `lattice::tetryen_node_envelope` evaluated at a real geodesic distance,
/// the same function `gui::Tetryen::node_amplitude` now delegates to.
/// `_mkb/tetryen_recurrence.md` itself proves every regular Tetryen gives a
/// *uniform* weight across all 6 edges regardless of which specific
/// instance is used, which is what makes accepting one scalar weight
/// (rather than a full geometry) an honest simplification here rather than
/// a loss of generality.
///
/// Driven by a [`VolumetricTimeCrystal`]'s own real, Howard-Comma-derived
/// `fundamental` frequency — a better-grounded driving frequency than an
/// arbitrary caller-supplied one, since it comes from the crystal's own
/// quantised spectrum rather than being invented for this purpose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TetryenRecurrence {
    current: PhaseSpaceVector,
    previous: PhaseSpaceVector,
}

impl TetryenRecurrence {
    /// Seed both time slices explicitly — a caller resuming from saved
    /// state, or one that wants non-zero initial discrete "velocity"
    /// (`current != previous`).
    pub const fn seeded(current: PhaseSpaceVector, previous: PhaseSpaceVector) -> Self {
        Self { current, previous }
    }

    /// Seed both time slices with the same initial state — zero initial
    /// discrete velocity.
    pub const fn at_rest(initial: PhaseSpaceVector) -> Self {
        Self::seeded(initial, initial)
    }

    pub const fn state(&self) -> PhaseSpaceVector {
        self.current
    }

    /// Advance one step, driven by `crystal`'s own real fundamental
    /// frequency (`omega = 2*pi*fundamental`).
    ///
    /// # Errors
    /// [`CrystalError::Diverged`] if this step left the recurrence's
    /// measured stability region (`_mkb/tetryen_recurrence.md` §3) and
    /// produced a non-finite amplitude. Refused rather than propagated —
    /// the state is left unchanged so a caller can inspect it or retry
    /// with different parameters.
    pub fn step(
        &mut self,
        crystal: &VolumetricTimeCrystal,
        dt: f64,
        gamma: f64,
        coupling_weight: f64,
    ) -> Result<PhaseSpaceVector, CrystalError> {
        let omega = std::f64::consts::TAU * crystal.fundamental().get();
        let cur = self.current.0;
        let prev = self.previous.0;
        let mut next = [0.0; 4];

        for i in 0..4 {
            let mut coupling = 0.0;
            for j in 0..4 {
                if j == i {
                    continue;
                }
                coupling += coupling_weight * (cur[j] - cur[i]);
            }

            let uncoupled = 2.0 * (omega * dt).cos() * cur[i] - prev[i];
            let value = uncoupled + gamma * dt * dt * coupling;

            if !value.is_finite() {
                return Err(CrystalError::Diverged { amplitude: value });
            }
            next[i] = value;
        }

        self.previous = self.current;
        self.current = PhaseSpaceVector(next);
        Ok(self.current)
    }
}
