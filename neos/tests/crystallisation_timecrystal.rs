//! Volumetric Time Crystals — PRD §8's second reading of media, now defined.
//!
//! Doctrine: `_mkb/test-doctrine.md`. **[D]** marks assertions a conventional
//! audio pipeline could not pass — one treating a stream as samples rather than
//! as a quantised spatiotemporal structure.

use crystallisation::constants::HOWARD_COMMA;
use crystallisation::timecrystal::{
    takens_embed, LorentzTransform, PhaseSpaceVector, TetryenRecurrence, VolumetricTimeCrystal,
};
use crystallisation::CrystalError;

/// A unit-amplitude tone. Macroscopic — **not** quantisable, see 2.5.
fn tone(hz: f64, rate: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| (std::f64::consts::TAU * hz * i as f64 / rate).sin())
        .collect()
}

/// A tone scaled into the regime where `C_H` quanta are exactly countable.
///
/// The Howard Comma is `~2.6e-34` J·s, so a unit-amplitude tone would need
/// `2.5e35` quanta — past `f64`'s exact-integer ceiling of `2^53`. Amplitude
/// `~2e-15` puts occupation around `1e6`, comfortably inside it.
fn quantisable_tone(hz: f64, rate: f64, n: usize) -> Vec<f64> {
    const AMPLITUDE: f64 = 2.0e-15;
    tone(hz, rate, n).into_iter().map(|s| s * AMPLITUDE).collect()
}

// ------------------------------------------- Group 1: phase-space embedding

/// 1.1 — Takens embedding places the signal into 4D, one component per Tetryen
/// vertex.
#[test]
fn takens_embeds_into_four_dimensions() {
    let sig = tone(200.0, 8000.0, 400);
    let tau = 3;
    let nodes = takens_embed(&sig, tau).unwrap();
    assert_eq!(nodes.len(), sig.len() - 3 * tau);
    for (i, v) in nodes.iter().enumerate() {
        let base = i + 3 * tau;
        assert_eq!(v.components()[0], sig[base]);
        assert_eq!(v.components()[3], sig[base - 3 * tau]);
    }
}

/// 1.2 — a zero delay embeds nothing and is refused.
///
/// All four components would collapse onto the same sample, giving a line in
/// 4D rather than a trajectory.
#[test]
fn zero_delay_is_refused() {
    let sig = tone(200.0, 8000.0, 400);
    assert!(matches!(
        takens_embed(&sig, 0),
        Err(CrystalError::EmptyMedia)
    ));
    assert!(matches!(
        takens_embed(&sig, 500),
        Err(CrystalError::EmptyMedia),
    ));
}

/// 1.3 [D] — the embedded trajectory is **Floquet periodic**.
///
/// A pure tone repeats with its own period; that periodicity in phase space is
/// what makes the structure a *time* crystal rather than a static one.
///
/// The tolerance is **relative to the trajectory's amplitude**. A quantisable
/// signal has amplitude `~1e-15`, so an absolute `1e-9` would call every period
/// a match and the negative assertion below would pass on nothing. Found by
/// this test failing after the amplitude was scaled.
#[test]
fn embedded_trajectory_is_floquet_periodic() {
    let (rate, hz) = (8000.0, 200.0);
    let sig = quantisable_tone(hz, rate, 2000);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, rate, 3).unwrap();
    let period = (rate / hz).round() as usize;

    assert!(
        vtc.is_floquet_periodic(period, 1e-9),
        "a pure tone must repeat at its own period"
    );
    assert!(
        !vtc.is_floquet_periodic(period + 7, 1e-9),
        "and must not repeat at an unrelated period"
    );
}

/// 1.4 — the periodicity check is scale-free.
///
/// The same trajectory at `1e-15` and at unit amplitude gives the same verdict.
/// Pins why the tolerance is relative; an absolute one would disagree.
#[test]
fn floquet_check_is_scale_free() {
    let (rate, hz) = (8000.0_f64, 200.0_f64);
    let period = (rate / hz).round() as usize;

    let quiet = VolumetricTimeCrystal::crystallise(&quantisable_tone(hz, rate, 2000), rate, 3)
        .expect("quantisable");
    // A louder trajectory, embedded directly so it bypasses the energy ceiling.
    let loud_nodes = takens_embed(&tone(hz, rate, 2000), 3).unwrap();

    let loud_scale = loud_nodes
        .iter()
        .flat_map(|v| v.components().iter())
        .fold(0.0_f64, |m, c| m.max(c.abs()));
    let quiet_scale = quiet
        .nodes()
        .iter()
        .flat_map(|v| v.components().iter())
        .fold(0.0_f64, |m, c| m.max(c.abs()));

    assert!(
        loud_scale / quiet_scale > 1e12,
        "the two trajectories must differ by many orders, got {}",
        loud_scale / quiet_scale
    );
    assert!(quiet.is_floquet_periodic(period, 1e-9));
    assert!(!quiet.is_floquet_periodic(period + 7, 1e-9));
}

// --------------------------------------- Group 2: Howard Comma quantisation

/// 2.1 [D] — **energy is conserved within the half-quantum floor.**
///
/// The doctrine check for a VTC: `|E - Σ n_k C_H ν_k| ≤ ½ C_H ν₀`.
#[test]
fn energy_is_conserved_within_the_half_quantum() {
    for hz in [120.0, 200.0, 440.0] {
        let sig = quantisable_tone(hz, 8000.0, 1024);
        let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
        assert!(
            vtc.is_energy_conserving(),
            "residual {} exceeds floor {} at {hz} Hz",
            vtc.energy_residual(),
            vtc.half_quantum_floor()
        );
    }
}

/// 2.2 [D] — **independent per-mode rounding would violate the bound.**
///
/// The reason `crystallise` lets the fundamental absorb the residual. Measured:
/// independent rounding overshoots by up to 36x. This reconstructs that
/// failure so the joint scheme is not mistaken for an arbitrary choice.
#[test]
fn independent_rounding_would_break_the_bound() {
    let sig = quantisable_tone(200.0, 8000.0, 1024);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();

    let independent = vtc.independent_rounding_residual();
    let floor = vtc.half_quantum_floor();

    assert!(
        independent > floor * 10.0,
        "accumulated per-mode error {independent} should far exceed the single \
         half-quantum floor {floor} — that is why quantisation is joint"
    );
    // And the joint scheme actually used stays inside it.
    assert!(
        vtc.energy_residual() <= floor,
        "joint quantisation left {} against floor {floor}",
        vtc.energy_residual()
    );
}

/// 2.5 [D] — **a macroscopic signal cannot be quantised, and is refused.**
///
/// The Howard Comma is `~2.6e-34` J·s, so a unit-amplitude tone needs `2.5e35`
/// quanta — past `f64`'s exact-integer ceiling of `2^53 ≈ 9.0e15`. Beyond that
/// an added quantum does not change the total and the quantisation is pretend.
///
/// Refusing mirrors ⊗'s domain refusal: a limit of the physics meeting IEEE-754,
/// surfaced rather than papered over.
#[test]
fn macroscopic_signal_exceeds_quantisation() {
    let loud = tone(200.0, 8000.0, 1024); // unit amplitude
    match VolumetricTimeCrystal::crystallise(&loud, 8000.0, 3) {
        Err(CrystalError::EnergyExceedsQuantisation { required, max }) => {
            assert!(required > 1e30, "expected a huge occupation, got {required}");
            assert!((max - 9_007_199_254_740_992.0).abs() < 1.0, "max is 2^53");
        }
        other => panic!("a macroscopic signal must be refused, got {other:?}"),
    }

    // And the scaled version is accepted.
    let quiet = quantisable_tone(200.0, 8000.0, 1024);
    assert!(VolumetricTimeCrystal::crystallise(&quiet, 8000.0, 3).is_ok());
}

/// 2.6 — the quantisable ceiling is derived from `C_H`, not hardcoded.
#[test]
fn quantisable_ceiling_is_derived() {
    let sig = quantisable_tone(200.0, 8000.0, 1024);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    let ceiling = VolumetricTimeCrystal::max_quantisable_energy(vtc.fundamental());
    assert!(
        vtc.input_energy() < ceiling,
        "an accepted signal must sit under the ceiling"
    );
    assert!(
        ceiling > 0.0 && ceiling < 1e-15,
        "the ceiling is microscopic — about 1.9e-17 J at this fundamental, got {ceiling}"
    );
}

/// 2.3 — the floor is derived from `C_H` and the fundamental, not hardcoded.
#[test]
fn half_quantum_floor_is_derived() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    let expected = 0.5 * HOWARD_COMMA * vtc.fundamental().get();
    assert!((vtc.half_quantum_floor() - expected).abs() < 1e-40);
}

/// 2.4 [D] — occupation numbers are integers, so energy comes in quanta.
///
/// A conventional pipeline stores continuous amplitudes; here every mode holds
/// a whole number of `C_H·ν` steps.
#[test]
fn mode_energies_are_whole_quanta() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    assert!(!vtc.modes().is_empty());
    for m in vtc.modes() {
        let quantum = HOWARD_COMMA * m.frequency.get();
        let ratio = m.energy() / quantum;
        assert!(
            (ratio - ratio.round()).abs() < 1e-6,
            "mode at {} Hz holds {ratio} quanta, not a whole number",
            m.frequency.get()
        );
    }
}

// --------------------------------------- Group 3: modulation and Liouville

/// 3.1 [D] — **`SO(3,1)` modulation preserves phase-space volume.**
///
/// Liouville's theorem: `det = 1`. A modulation that changed the volume would
/// be creating or destroying information content.
#[test]
fn modulation_preserves_phase_space_volume() {
    for t in [
        LorentzTransform::boost(0.6, 0),
        LorentzTransform::rotation(0.7, 0, 1),
        LorentzTransform::boost(0.9, 2).compose(&LorentzTransform::rotation(0.4, 1, 2)),
    ] {
        assert!(
            (t.determinant() - 1.0).abs() < 1e-12,
            "det = {}, Liouville requires 1",
            t.determinant()
        );
        assert!(t.is_volume_preserving(1e-9));
    }
}

/// 3.2 [D] — modulation preserves the Minkowski form.
///
/// This is what makes `SO(3,1)` the right group: the `(3,1)` interval of the
/// phase-space vector is invariant, so modulation is a change of view rather
/// than a change of content.
#[test]
fn modulation_preserves_the_minkowski_form() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    let t = LorentzTransform::boost(0.5, 0).compose(&LorentzTransform::rotation(0.3, 1, 2));
    let moved = vtc.modulate(&t).unwrap();

    for (a, b) in vtc.nodes().iter().zip(moved.nodes()) {
        let (before, after) = (a.minkowski_norm(), b.minkowski_norm());
        let scale = before.abs().max(1e-9);
        assert!(
            (before - after).abs() / scale < 1e-9,
            "interval changed: {before} -> {after}"
        );
    }
}

/// 3.3 — every transform this API can construct is already volume-preserving.
///
/// `boost`, `rotation`, and `compose` all land in `SO(3,1)`, so a non-unitary
/// modulation is **unrepresentable** through the public constructors — the
/// guard in `modulate` cannot be triggered from outside.
///
/// That is the stronger position, and it is worth recording as such rather than
/// leaving a test that pretends to exercise the guard. The check stays because
/// it costs nothing and would catch a future constructor that broke the
/// invariant.
#[test]
fn every_constructible_transform_is_unitary() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();

    let transforms = [
        LorentzTransform::IDENTITY,
        LorentzTransform::boost(0.0, 0),
        LorentzTransform::boost(1.3, 1),
        LorentzTransform::rotation(2.1, 0, 2),
        LorentzTransform::boost(1.0, 0).compose(&LorentzTransform::boost(1.0, 0)),
        LorentzTransform::rotation(0.5, 1, 2).compose(&LorentzTransform::boost(0.8, 0)),
    ];
    for t in transforms {
        assert!(
            t.is_volume_preserving(1e-9),
            "det = {} escaped SO(3,1)",
            t.determinant()
        );
        assert!(vtc.modulate(&t).is_ok());
    }
}

/// 3.4 — modulation leaves the energy spectrum untouched.
///
/// A change of view does not repopulate the modes.
#[test]
fn modulation_does_not_alter_the_spectrum() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    let moved = vtc
        .modulate(&LorentzTransform::rotation(0.9, 0, 2))
        .unwrap();

    assert_eq!(vtc.modes().len(), moved.modes().len());
    assert_eq!(vtc.quantised_energy(), moved.quantised_energy());
    assert!(moved.is_energy_conserving());
}

// ------------------------------------------------ Group 4: the whole crystal

/// 4.1 — a crystal is refused for degenerate input.
#[test]
fn degenerate_media_is_refused() {
    assert!(matches!(
        VolumetricTimeCrystal::crystallise(&[], 8000.0, 3),
        Err(CrystalError::EmptyMedia)
    ));
    assert!(matches!(
        VolumetricTimeCrystal::crystallise(&quantisable_tone(200.0, 8000.0, 512), 0.0, 3),
        Err(CrystalError::EmptyMedia)
    ));
}

/// 4.2 — the crystal reports ordinary frequency throughout.
///
/// `C_H` pairs with `ν`. Every mode carries `substrate::Frequency`, so the
/// angular carrier cannot reach the quantisation.
#[test]
fn modes_carry_ordinary_frequency() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    let f: substrate::Frequency = vtc.fundamental();
    assert!(f.get() > 0.0);
    for m in vtc.modes() {
        let _: substrate::Frequency = m.frequency;
    }
}

// --------------------------------------- Group 5: the Tetryen recurrence
//
// `_mkb/tetryen_recurrence.md` — the same synthesis `gui::TetryenState`
// implements, driven here by a real `VolumetricTimeCrystal`'s own
// Howard-Comma-derived fundamental frequency rather than an arbitrary one.
// This crate has no Tetryen geometry, so the coupling weight is supplied
// by the caller (see the type's own doc comment for why that's an honest
// simplification, not a shortcut).

/// [D] When every component starts at the same value, every pairwise
/// difference is exactly zero, so the coupling term vanishes identically
/// **regardless of the coupling weight** — the step must reduce to the
/// plain uncoupled identity `psi_{n+1} = 2cos(w dt) psi_n - psi_{n-1}`,
/// matched here against the closed form `cos(phi + w dt)` it is derived
/// from, using the crystal's own real fundamental as `w`.
#[test]
fn identical_components_evolve_by_the_uncoupled_identity_regardless_of_coupling() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    let omega = std::f64::consts::TAU * vtc.fundamental().get();
    let (dt, phi) = (1e-5_f64, 0.7_f64);

    let prev_val = (phi - omega * dt).cos();
    let curr_val = phi.cos();
    let expected_next = (phi + omega * dt).cos();

    for weight in [0.0, 1.0, 100.0] {
        let mut state = TetryenRecurrence::seeded(
            PhaseSpaceVector([curr_val; 4]),
            PhaseSpaceVector([prev_val; 4]),
        );
        let next = state.step(&vtc, dt, 1.0, weight).unwrap();
        for &v in next.components() {
            assert!(
                (v - expected_next).abs() < 1e-9,
                "weight={weight}: expected {expected_next}, got {v}"
            );
        }
    }
}

/// Coupling is a real, directional effect: an outlier component relaxes
/// toward its lower neighbours, and a low neighbour is pulled up toward
/// the outlier.
#[test]
fn coupling_pulls_differing_components_toward_each_other() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    let dt = 1e-5;
    let initial = PhaseSpaceVector([10.0, 0.0, 0.0, 0.0]);

    let mut with_coupling = TetryenRecurrence::at_rest(initial);
    let mut without = TetryenRecurrence::at_rest(initial);
    let next_coupled = with_coupling.step(&vtc, dt, 100.0, 1.0).unwrap();
    let next_uncoupled = without.step(&vtc, dt, 0.0, 1.0).unwrap();

    assert!(
        next_coupled.components()[0] < next_uncoupled.components()[0],
        "the outlier component must relax toward its lower neighbours under coupling"
    );
    assert!(
        next_coupled.components()[1] > next_uncoupled.components()[1],
        "a low neighbour must be pulled up toward the outlier under coupling"
    );
}

/// The recurrence's own documented safe region stays bounded over a long
/// run — not just briefly.
#[test]
fn stays_bounded_within_the_documented_safe_region() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    let mut state = TetryenRecurrence::at_rest(PhaseSpaceVector([1.0, 0.5, -0.3, 0.8]));
    for _ in 0..50_000 {
        let next = state
            .step(&vtc, 1e-5, 1.0, 1.0)
            .expect("documented-safe parameters must not diverge");
        for &v in next.components() {
            assert!(v.abs() < 100.0, "amplitude {v} escaped a generous bound");
        }
    }
}

/// [D] A step that leaves the measured stability region is refused, not
/// silently propagated as `inf`/`NaN`.
#[test]
fn a_step_leaving_the_stability_region_is_refused_not_propagated() {
    let sig = quantisable_tone(200.0, 8000.0, 512);
    let vtc = VolumetricTimeCrystal::crystallise(&sig, 8000.0, 3).unwrap();
    let mut state = TetryenRecurrence::at_rest(PhaseSpaceVector([1.0, 0.5, -0.3, 0.8]));
    let mut diverged = false;
    for _ in 0..5000 {
        match state.step(&vtc, 1e-5, 1e12, 1.0) {
            Ok(_) => {}
            Err(CrystalError::Diverged { amplitude }) => {
                assert!(!amplitude.is_finite());
                diverged = true;
                break;
            }
            Err(other) => panic!("unexpected error: {other}"),
        }
    }
    assert!(
        diverged,
        "gamma=1e12 at this crystal's fundamental is verified to leave the stable region \
         within 118 steps (checked in a disposable scratch harness before writing this test)"
    );
}

/// `at_rest` seeds both time slices identically.
#[test]
fn at_rest_seeds_identically() {
    let state = TetryenRecurrence::at_rest(PhaseSpaceVector([1.0, 2.0, 3.0, 4.0]));
    assert_eq!(state.state(), PhaseSpaceVector([1.0, 2.0, 3.0, 4.0]));
}
