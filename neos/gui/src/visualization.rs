//! System state rendered as standing waves.
//!
//! Contract §5. Load, memory, and traffic appear as real-time standing waves
//! showing constructive and destructive energy states.
//!
//! [`TetryenVisualisation`] extends the same idea to PRD §8's crystallised
//! media: a real [`crystallisation::FaceProjection`] or
//! [`crystallisation::PhaseSpaceVector`] driving the amplitude at each of a
//! [`crate::renderer::Tetryen`]'s four nodes, the same way [`LoadVisualisation`]
//! drives one wave per core from the kernel's real load field.

/// A standing wave: `f(t) = 2A sin(kx) cos(wt)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StandingWave {
    amplitude: f64,
    k: f64,
    omega: f64,
}

impl StandingWave {
    pub const fn new(amplitude: f64, k: f64, omega: f64) -> Self {
        Self {
            amplitude,
            k,
            omega,
        }
    }

    /// Amplitude tracks the quantity being visualised, so **zero load renders
    /// zero amplitude** - the display cannot show activity where there is none.
    pub const fn for_load(load: f64, k: f64, omega: f64) -> Self {
        Self::new(load, k, omega)
    }

    pub const fn amplitude(self) -> f64 {
        self.amplitude
    }

    /// Displacement at position `x`, time `t`.
    pub fn at(&self, x: f64, t: f64) -> f64 {
        2.0 * self.amplitude * (self.k * x).sin() * (self.omega * t).cos()
    }

    /// Largest displacement this wave can reach.
    pub fn peak(&self) -> f64 {
        2.0 * self.amplitude.abs()
    }
}

/// How two waves combine where they meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interference {
    /// Aligned phases reinforce.
    Constructive,
    /// Opposed phases cancel.
    Destructive,
}

/// Superposition of two waves separated by `phase_delta`.
pub fn combine(a: &StandingWave, b: &StandingWave, phase_delta: f64, x: f64, t: f64) -> f64 {
    a.at(x, t) + 2.0 * b.amplitude * (b.k * x).sin() * (b.omega * t + phase_delta).cos()
}

/// Classify by phase separation.
///
/// Opposed phases (near `pi`) cancel; aligned phases (near `0`) reinforce.
pub fn classify(phase_delta: f64, tol: f64) -> Interference {
    let wrapped = phase_delta.rem_euclid(std::f64::consts::TAU);
    let from_pi = (wrapped - std::f64::consts::PI).abs();
    if from_pi <= tol {
        Interference::Destructive
    } else {
        Interference::Constructive
    }
}

/// Superposition of two unit waves at the given phases.
///
/// Exactly zero for opposed phases - destructive interference is **total**, not
/// merely dimmer. A renderer that faded overlapping waves would not reach zero.
pub fn superpose_phases(phase_a: f64, phase_b: f64, omega: f64, t: f64) -> f64 {
    (omega * t + phase_a).cos() + (omega * t + phase_b).cos()
}

/// One standing wave per core, driven by a real [`SystemSnapshot`].
///
/// This is PRD §9's "system load rendered as real-time standing waves" wired to
/// the kernel's actual load field rather than to a number a caller supplied.
///
/// Amplitudes come from [`SystemSnapshot::normalised_load`], **not** raw joules
/// - see [`crate::telemetry`] for why drawing raw energy would render a fully
/// loaded machine as idle.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadVisualisation {
    waves: Vec<StandingWave>,
}

impl LoadVisualisation {
    /// Build one wave per core from a live snapshot.
    pub fn from_snapshot(snapshot: &crate::telemetry::SystemSnapshot, k: f64, omega: f64) -> Self {
        Self {
            waves: snapshot
                .normalised_load()
                .into_iter()
                .map(|load| StandingWave::for_load(load, k, omega))
                .collect(),
        }
    }

    pub fn cores(&self) -> usize {
        self.waves.len()
    }

    pub fn wave(&self, core: usize) -> Option<&StandingWave> {
        self.waves.get(core)
    }

    /// Displacement of one core's wave.
    pub fn at(&self, core: usize, x: f64, t: f64) -> f64 {
        self.waves.get(core).map_or(0.0, |w| w.at(x, t))
    }

    /// Combined displacement of every core - the whole field at a glance.
    pub fn total(&self, x: f64, t: f64) -> f64 {
        self.waves.iter().map(|w| w.at(x, t)).sum()
    }

    /// Largest peak among the cores. Zero for an idle system.
    pub fn peak(&self) -> f64 {
        self.waves
            .iter()
            .map(StandingWave::peak)
            .fold(0.0_f64, f64::max)
    }

    /// Spread between the busiest and quietest core's peaks.
    ///
    /// This is what makes imbalance *visible*: a relaxed field draws flat, an
    /// imbalanced one draws ragged.
    pub fn amplitude_spread(&self) -> f64 {
        if self.waves.is_empty() {
            return 0.0;
        }
        let peaks: Vec<f64> = self.waves.iter().map(StandingWave::peak).collect();
        let hi = peaks.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let lo = peaks.iter().cloned().fold(f64::INFINITY, f64::min);
        hi - lo
    }
}

/// Build four normalised amplitudes from four raw magnitudes.
///
/// Shared by both [`TetryenVisualisation`] constructors so the
/// relative-not-absolute discipline lives in one place rather than being
/// repeated per data source. Scaled against the largest magnitude, same shape
/// as [`SystemSnapshot::normalised_load`](crate::telemetry::SystemSnapshot::normalised_load)
/// and for the same reason: a raw amplitude here can be arbitrarily small
/// (spectral energy, a raw sample value), and an unscaled amplitude would
/// render every node as visually silent regardless of the source's actual
/// relative structure.
fn normalise_four(raw: [f64; 4]) -> [f64; 4] {
    let peak = raw.iter().cloned().fold(0.0_f64, f64::max);
    if peak > 0.0 {
        std::array::from_fn(|i| raw[i] / peak)
    } else {
        [0.0; 4]
    }
}

/// One standing wave per node of a [`crate::renderer::Tetryen`], driven by
/// real crystallised media rather than a caller-supplied number.
///
/// PRD §8 crystallises media onto Tetryen structure two ways this type joins
/// to the renderer:
///
/// - **Holographic** (`crystallisation::holographic`) projects an image's
///   spectrum onto the Tetryen's four **faces**, one
///   [`FaceProjection`](crystallisation::FaceProjection) per face, already in
///   face order.
/// - **Volumetric time-crystal** (`crystallisation::timecrystal`) embeds a
///   signal into 4D phase space, and `_mkb/timecrystal.md` section 1 already
///   states the correspondence used here: *"the four components map to the
///   four vertices of a fundamental Tetryen cell."* No new geometry is
///   introduced - the four-ness was law before this type existed.
#[derive(Debug, Clone, PartialEq)]
pub struct TetryenVisualisation {
    waves: [StandingWave; 4],
}

impl TetryenVisualisation {
    /// One wave per face, from a real holographic projection.
    pub fn from_face_projections(
        faces: &[crystallisation::FaceProjection; 4],
        k: f64,
        omega: f64,
    ) -> Self {
        let raw: [f64; 4] = std::array::from_fn(|i| faces[i].energy());
        Self::from_amplitudes(raw, k, omega)
    }

    /// One wave per vertex, from a single point of a volumetric time-crystal's
    /// embedded trajectory.
    ///
    /// A phase-space component is a raw signal value, not an energy - it can
    /// be negative, and a strongly negative sample is still strongly *active*,
    /// not idle. So amplitude is driven by each component's magnitude, a
    /// chosen mapping rather than a derived one, stated here rather than left
    /// implicit - the same discipline `telemetry`'s traffic mapping already
    /// follows.
    pub fn from_phase_vector(
        node: &crystallisation::PhaseSpaceVector,
        k: f64,
        omega: f64,
    ) -> Self {
        let raw: [f64; 4] = std::array::from_fn(|i| node.components()[i].abs());
        Self::from_amplitudes(raw, k, omega)
    }

    fn from_amplitudes(raw: [f64; 4], k: f64, omega: f64) -> Self {
        let normalised = normalise_four(raw);
        Self {
            waves: std::array::from_fn(|i| StandingWave::for_load(normalised[i], k, omega)),
        }
    }

    /// Exactly four, one per Tetryen node - matching
    /// [`Tetryen::nodes`](crate::renderer::Tetryen::nodes).
    pub fn wave(&self, node: usize) -> Option<&StandingWave> {
        self.waves.get(node)
    }

    /// Displacement of one node's wave.
    pub fn at(&self, node: usize, x: f64, t: f64) -> f64 {
        self.waves.get(node).map_or(0.0, |w| w.at(x, t))
    }

    /// Largest peak among the four nodes. Zero when every amplitude is zero.
    pub fn peak(&self) -> f64 {
        self.waves.iter().map(StandingWave::peak).fold(0.0_f64, f64::max)
    }
}
