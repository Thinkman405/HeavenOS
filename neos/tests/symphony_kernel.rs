//! Physics assertions for symphony-kernel.
//!
//! Doctrine: `_mkb/test-doctrine.md`. Plan:
//! `subsystems/symphony-kernel/03_tests/output/test-plan.md`.
//!
//! **[D]** marks assertions a conventional implementation could not pass — a
//! Planck-constant quantizer, a priority-queue scheduler, or a heuristic
//! load balancer.

use symphony_kernel::constants::*;
use symphony_kernel::quantization::RECLAMATION_THRESHOLD;
use symphony_kernel::equilibrium::LoadField;
use symphony_kernel::{
    detuning, energy, evaluate_branch, is_reclaimable, resonates, xi, CoreTopology,
    DriftIntegrator, Frequency, Interference, KernelError, Phase,
};

const PLANCK_H: f64 = 6.626_070_15e-34;

// ----------------------------------------------------- Group 1: quantization

/// 1.1 — `C_H = h/sqrt(2*pi)`, sourced from the MKB and checked against the
/// closed form.
#[test]
fn howard_comma_is_planck_over_root_two_pi() {
    let expected = PLANCK_H / (2.0 * std::f64::consts::PI).sqrt();
    assert!(
        (HOWARD_COMMA - expected).abs() < 1e-45,
        "C_H = {HOWARD_COMMA}, expected h/sqrt(2pi) = {expected}"
    );
}

/// 1.2 [D] — `C_H` is neither `h` nor `hbar`. A Planck-based quantizer fails.
#[test]
fn howard_comma_is_neither_h_nor_hbar() {
    let hbar = PLANCK_H / (2.0 * std::f64::consts::PI);
    assert!((HOWARD_COMMA - PLANCK_H).abs() > 1e-35, "C_H must not be h");
    assert!((HOWARD_COMMA - hbar).abs() > 1e-35, "C_H must not be hbar");
}

/// 1.3 — hand-computed energy for a 1 GHz process.
#[test]
fn energy_of_one_gigahertz_matches_hand_computation() {
    let e = energy(Frequency::hertz(1e9)).0;
    let expected = 2.643_419_535_740_863e-25;
    assert!(
        ((e - expected) / expected).abs() < 1e-12,
        "E = {e}, expected {expected}"
    );
}

/// 1.4 [D] — the intended ratio to Planck, pinned so nobody "corrects" `C_H`.
#[test]
fn energy_is_planck_scaled_by_the_intended_factor() {
    let nu = 3.7e9;
    let ours = energy(Frequency::hertz(nu)).0;
    let planck = PLANCK_H * nu;
    let ratio = ours / planck;
    assert!(
        (ratio - 0.398_942_280_401_432_7).abs() < 1e-12,
        "E/hv = {ratio}, expected 0.3989422804014327 — this departure from \
         Planck is deliberate, see _mkb/reconciliation.md R5a"
    );
}

/// 1.5 — energy is linear in frequency.
#[test]
fn energy_is_linear_in_frequency() {
    let a = energy(Frequency::hertz(1.5e9)).0;
    let b = energy(Frequency::hertz(3.0e9)).0;
    assert!(((b / a) - 2.0).abs() < 1e-12);
}

/// 1.6 — GC falls out of the equation: `nu -> 0` implies no energy.
#[test]
fn vanishing_frequency_is_reclaimable() {
    assert!(is_reclaimable(Frequency::hertz(0.0)));
    assert!(is_reclaimable(RECLAMATION_THRESHOLD));
    assert!(!is_reclaimable(Frequency::hertz(1.0)));
    assert!(energy(Frequency::hertz(0.0)).0 == 0.0);
}

/// 1.7 [D] — the `nu`/`omega` conversion is exactly `2*pi`.
///
/// The separation itself is enforced at compile time: `energy()` takes
/// `Frequency`, so passing an `AngularFrequency` does not compile. That cannot
/// be asserted at runtime, so this pins the conversion instead.
#[test]
fn ordinary_and_angular_frequency_differ_by_tau() {
    let nu = Frequency::hertz(1e9);
    let omega = nu.to_angular();
    assert!((omega.get() / nu.get() - std::f64::consts::TAU).abs() < 1e-6);
    assert!((omega.to_ordinary().get() - nu.get()).abs() < 1e-3);
}

// -------------------------------------------------------- Group 2: resonance

/// 2.1 — `xi(R) = 1`, exactly. A correction factor must be unity at reference.
#[test]
fn xi_is_exactly_one_at_reference_scale() {
    assert_eq!(xi(1.0).unwrap(), 1.0);
    assert_eq!(XI_AT_REFERENCE, 1.0);
}

/// 2.2 [D] — **the safety property**: `xi` is bounded.
///
/// An unbounded correction in a clock path lets one bad sample stall the
/// scheduler. The rejected `sinh(1)/sinh(r/R)` form reaches 1.2e6 near zero
/// and fails this outright.
#[test]
fn xi_is_bounded_everywhere() {
    let mut max = f64::NEG_INFINITY;
    for i in 0..=30_000 {
        let r = i as f64 / 1000.0;
        let v = xi(r).expect("r >= 0 is in the domain");
        assert!(v.is_finite(), "xi({r}) = {v} is not finite");
        max = max.max(v);
    }
    assert!(
        max <= XI_SUPREMUM + 1e-12,
        "xi peaked at {max}, above the declared supremum {XI_SUPREMUM}"
    );
}

/// 2.2b [D] — boundedness holds across the **whole representable domain**, not
/// just the operating range.
///
/// This test exists because the one above did not catch a real divergence.
/// It sweeps `r` in `[0, 30]`, and the literal transcription of `xi` —
/// `sinh(r)/(r*sinh 1) * exp(1-r)` — is perfectly well behaved there. It
/// returns `+inf` from `r ~ 710.5` (where `sinh(r)` overflows `f64` before
/// `exp(1-r)` can rescue the product) and `NaN` above `~746`, as `Ok` values
/// that propagate straight into the load field.
///
/// A law that says "bounded" and a test that only checks part of the domain is
/// how an invariant gets violated in shipped code. The range is now the range
/// the law claims: everything representable.
#[test]
fn xi_is_bounded_across_the_entire_representable_domain() {
    let sampled = [
        0.0, 1e-300, 1e-30, 1e-9, 0.5, 1.0, 2.0, 30.0, 100.0, 709.0, 710.0,
        // the region the naive form destroys
        710.5, 711.0, 720.0, 745.0, 746.0, 750.0, 1e3, 1e6, 1e30, 1e300, f64::MAX,
        f64::INFINITY,
    ];
    for r in sampled {
        let v = xi(r).expect("every r >= 0 is in the domain, including infinity");
        assert!(
            v.is_finite(),
            "xi({r}) = {v} is not finite — the boundedness law is violated"
        );
        assert!(v >= 0.0, "xi({r}) = {v} went negative");
        assert!(
            v <= XI_SUPREMUM,
            "xi({r}) = {v} exceeds the declared supremum {XI_SUPREMUM}"
        );
    }

    // Dense sweep straight through the overflow region.
    let mut r = 700.0;
    while r < 760.0 {
        let v = xi(r).unwrap();
        assert!(v.is_finite() && v > 0.0, "xi({r}) = {v} in the overflow region");
        r += 0.25;
    }
}

/// 2.2c — the two algebraic forms of `xi` agree where both are valid.
///
/// `sinh(r)*e^(1-r) = (e - e^(1-2r))/2` identically, so the piecewise
/// implementation is not an approximation — it is the same function evaluated
/// where each branch is numerically sound. Pinned so a future "simplification"
/// back to one branch has to argue with a test.
#[test]
fn both_algebraic_forms_of_xi_agree_where_both_are_valid() {
    let naive = |r: f64| r.sinh() / (r * 1.0_f64.sinh()) * (1.0 - r).exp();

    let mut worst: f64 = 0.0;
    let mut r = 1e-6;
    while r < 700.0 {
        let (a, b) = (naive(r), xi(r).unwrap());
        assert!(a.is_finite(), "premise: the naive form is valid below 710");
        worst = worst.max(((a - b) / a).abs());
        r *= 1.05;
    }
    assert!(
        worst < 1e-14,
        "the piecewise form disagrees with the closed form by {worst} relative"
    );

    // Beyond 710 the naive form is the one that is wrong.
    assert!(!naive(711.0).is_finite(), "premise: naive overflows at 711");
    assert!(xi(711.0).unwrap().is_finite());
}

/// 2.3 — `xi(0)` is the limit, not NaN. The expression is `0/0` there.
#[test]
fn xi_at_zero_is_the_supremum_not_nan() {
    let v = xi(0.0).expect("r = 0 is valid; it evaluates by limit");
    assert!(v.is_finite());
    assert!((v - XI_SUPREMUM).abs() < 1e-12);
    assert!((v - std::f64::consts::E / 1.0_f64.sinh()).abs() < 1e-12);
}

/// 2.4 — strictly decreasing.
#[test]
fn xi_is_strictly_decreasing() {
    let mut prev = xi(0.0).unwrap();
    for i in 1..=3000 {
        let v = xi(i as f64 / 100.0).unwrap();
        assert!(v < prev, "xi is not decreasing at r = {}", i as f64 / 100.0);
        prev = v;
    }
}

/// 2.5 — negative scale is genuinely outside the domain.
#[test]
fn xi_rejects_negative_scale() {
    assert!(matches!(xi(-0.1), Err(KernelError::UndefinedScale { .. })));
    assert!(matches!(xi(f64::NAN), Err(KernelError::UndefinedScale { .. })));
}

/// 2.6 — `H(kappa) -> 0` by cancellation across scales.
///
/// The invariant is *cancellation*, not mere decay: a decaying but one-signed
/// (or geometrically alternating) error sequence converges to a **non-zero**
/// limit — `sum A(-r)^k dt = A dt/(1+r)`. Only paired cancellation drives the
/// integral to zero, which is what "cancellation across fractal scales" means.
#[test]
fn drift_integral_converges_by_cancellation() {
    let mut d = DriftIntegrator::new();
    let mut err = 1.0;
    for _ in 0..200 {
        d.observe(err, 0.01);
        d.observe(-err, 0.01);
        err *= 0.9; // decaying envelope, each pair cancelling
    }
    assert!(
        d.is_converging(1e-9),
        "H(kappa) = {} should cancel to zero",
        d.residual()
    );
    assert!(
        d.peak_excursion() > 0.0,
        "the integrator must actually have moved before cancelling"
    );
    assert_eq!(d.samples(), 400);
}

/// A decaying but non-cancelling sequence settles away from zero — recorded so
/// the distinction above is not mistaken for a stricter tolerance.
#[test]
fn decaying_without_cancellation_does_not_converge_to_zero() {
    let mut d = DriftIntegrator::new();
    let mut err = 1.0;
    for _ in 0..200 {
        d.observe(err, 0.01);
        err *= -0.5;
    }
    let expected = 0.01 / 1.5; // A*dt/(1+r)
    assert!(
        (d.residual() - expected).abs() < 1e-9,
        "residual {} should settle at {expected}, not zero",
        d.residual()
    );
    assert!(!d.is_converging(1e-3));
}

/// 2.7 — a monitor that never fires is not a monitor.
#[test]
fn drift_integral_detects_divergence() {
    let mut d = DriftIntegrator::new();
    for _ in 0..100 {
        d.observe(1.0, 0.01); // systematic, undamped
    }
    assert!(
        !d.is_converging(1e-3),
        "a systematic drift of {} must be flagged",
        d.residual()
    );
    assert!(d.peak_excursion() > 0.9);
}

// ------------------------------------------------------ Group 3: equilibrium

fn spiked(n: usize) -> LoadField {
    let mut v = vec![0.0; n];
    v[0] = (n * 4) as f64;
    LoadField::new(v)
}

/// 3.1 — task density is mean-centred: the solvability condition.
#[test]
fn task_density_sums_to_zero() {
    for n in [7, 16, 31, 64] {
        let sum: f64 = spiked(n).task_density().iter().sum();
        assert!(
            sum.abs() < 1e-12,
            "sum(rho) = {sum} for n = {n}; L phi = -rho/eps0 has no solution unless this is 0"
        );
    }
}

/// 3.2 [D] — load converges to uniform on real {5,4} adjacency.
///
/// Judged on **relative** spread. An absolute threshold is meaningless as a
/// convergence criterion once loads carry physical magnitudes — see the
/// implementation log.
#[test]
fn load_converges_to_uniform_equilibrium() {
    for n in [7, 16, 31, 64] {
        let topo = CoreTopology::from_tiling(n);
        let mut field = spiked(n);
        let before = field.relative_spread();
        let alpha = topo.stability_bound() * 0.9;
        field
            .relax_to_equilibrium(&topo, alpha, 1e-9, 50_000)
            .expect("alpha is inside the stability bound");
        assert!(
            field.relative_spread() <= 1e-9,
            "n = {n}: relative spread {before} -> {} did not reach equilibrium",
            field.relative_spread()
        );
    }
}

/// 3.2b — relative spread is scale-free: the same field scaled by 1e-25
/// converges identically. This is the property an absolute tolerance lacks.
#[test]
fn convergence_criterion_is_scale_free() {
    let topo = CoreTopology::from_tiling(16);
    let tiny = LoadField::new(spiked(16).load().iter().map(|v| v * 1e-25).collect());
    let mut a = spiked(16);
    let mut b = tiny;
    let alpha = topo.stability_bound() * 0.9;
    let sa = a.relax_to_equilibrium(&topo, alpha, 1e-9, 50_000).unwrap();
    let sb = b.relax_to_equilibrium(&topo, alpha, 1e-9, 50_000).unwrap();
    assert_eq!(sa, sb, "convergence must not depend on absolute magnitude");
    assert!(b.relative_spread() <= 1e-9);
}

/// 3.2c — an unstable coupling is rejected even when the field is already
/// converged. Validation happens before the loop, not inside it.
#[test]
fn already_converged_field_still_rejects_bad_coupling() {
    let topo = CoreTopology::from_tiling(16);
    let mut flat = LoadField::new(vec![1.0; 16]);
    assert!(flat.relative_spread() <= 1e-9, "this field starts converged");
    assert!(matches!(
        flat.relax_to_equilibrium(&topo, topo.stability_bound() * 2.0, 1e-9, 10),
        Err(KernelError::Unstable { .. })
    ));
}

/// 3.3 — total load is conserved. A balancer that loses work is worse than none.
#[test]
fn total_load_is_conserved_under_relaxation() {
    let n = 31;
    let topo = CoreTopology::from_tiling(n);
    let mut field = spiked(n);
    let before = field.total();
    let alpha = topo.stability_bound() * 0.9;
    field.relax_to_equilibrium(&topo, alpha, 1e-9, 50_000).unwrap();
    assert!(
        (field.total() - before).abs() < 1e-9,
        "total load {before} -> {}; the Laplacian is conservative",
        field.total()
    );
}

/// 3.4 — an out-of-bound coupling is rejected, not silently oscillated.
#[test]
fn unstable_coupling_is_rejected() {
    let topo = CoreTopology::from_tiling(31);
    let mut field = spiked(31);
    let bad = topo.stability_bound() * 1.01;
    assert!(matches!(
        field.relax(&topo, bad),
        Err(KernelError::Unstable { .. })
    ));
    assert!(matches!(
        field.relax(&topo, 0.0),
        Err(KernelError::Unstable { .. })
    ));
}

/// 3.5 — the bound is derived from topology, never constant.
#[test]
fn stability_bound_is_derived_from_topology() {
    let topo = CoreTopology::from_tiling(31);
    let expected = DIFFUSION_STABILITY_FACTOR / (2.0 * topo.max_degree() as f64);
    assert!((topo.stability_bound() - expected).abs() < 1e-12);
    assert!(topo.stability_bound() > 0.0);
}

/// 3.6 [D] — a bounded patch is not vertex-transitive.
///
/// Boundary cores have fewer in-patch neighbours than interior ones. A balancer
/// assuming uniform degree 5 mis-weights the boundary.
#[test]
fn boundary_cores_have_lower_degree_than_interior() {
    let topo = CoreTopology::from_tiling(31);
    assert!(
        topo.min_degree() < topo.max_degree(),
        "a finite patch must have a boundary: min {} max {}",
        topo.min_degree(),
        topo.max_degree()
    );
    assert_eq!(topo.max_degree(), 5, "interior cells have five face-neighbours");
}

// -------------------------------------------------- Group 4: lattice topology

/// 4.1 / 4.3 — adjacency comes from `lattice` and keeps its guarantees.
#[test]
fn topology_adjacency_comes_from_lattice() {
    let topo = CoreTopology::from_tiling(64);
    assert_eq!(topo.len(), 64);
    for i in 0..topo.len() {
        assert!(
            topo.degree(i) <= 5,
            "core {i} has {} neighbours; {{5,4}} cells have at most five",
            topo.degree(i)
        );
        assert!(!topo.neighbors(i).contains(&i), "no core is its own neighbour");
    }
    assert_eq!(topo.max_degree(), 5);
}

/// 4.2 — adjacency survives the patch restriction symmetrically.
#[test]
fn topology_adjacency_is_symmetric() {
    let topo = CoreTopology::from_tiling(64);
    for i in 0..topo.len() {
        for &j in topo.neighbors(i) {
            assert!(
                topo.neighbors(j).contains(&i),
                "core {i} lists {j} but not conversely"
            );
        }
    }
}

/// 4.4 — a disconnected patch would silently break solvability by enlarging
/// `L`'s nullspace beyond the constants.
#[test]
fn topology_is_connected() {
    for n in [1, 2, 7, 16, 31, 64] {
        assert!(
            CoreTopology::from_tiling(n).is_connected(),
            "topology for {n} cores must be connected"
        );
    }
}

// ------------------------------------- Group 5: the three geometric gates
//
// Law: `_mkb/gates.md`, a synthesis closing PRD section 3. Interference was
// already law; phase shift and scale modulation are derived there from the
// teardown shift, the standing-wave stability variance, and `xi`.

/// 5.1 [D] — A2's two orientations are separated by **exactly** pi, which is
/// exactly the Phase Inversion Teardown shift.
///
/// This is the whole basis of gate 2: the teardown shift is not merely
/// compatible with A2's set, it is the map between its two elements.
#[test]
fn the_two_orientations_are_exactly_a_teardown_shift_apart() {
    let separation = Phase::Positive.radians() - Phase::Negative.radians();
    assert_eq!(
        separation, PHASE_INVERSION_SHIFT,
        "A2's orientations must be exactly the teardown shift apart"
    );
    assert_eq!(separation, std::f64::consts::PI);
}

/// 5.2 [D] — the pi shift is an **involution on A2's set**, and the set is
/// closed under it. Closure is what makes it a gate rather than an escape.
#[test]
fn inversion_is_a_closed_involution() {
    for p in [Phase::Negative, Phase::Positive] {
        assert_ne!(p.invert(), p);
        assert_eq!(p.invert().invert(), p);
        // Closed: the result is still one of the two permitted orientations.
        assert!(matches!(p.invert(), Phase::Negative | Phase::Positive));
    }
    assert_eq!(Phase::Negative.invert(), Phase::Positive);
    assert_eq!(Phase::Positive.invert(), Phase::Negative);
}

/// 5.3 [D] — the teardown identity `f_total = f_A + f_B = 0`, bit-exactly.
///
/// This exact zero is why FTG session teardown needs no acknowledgement.
#[test]
fn a_phase_and_its_inversion_superpose_to_exactly_zero() {
    for p in [Phase::Negative, Phase::Positive] {
        let sum = p.radians().sin() + p.invert().radians().sin();
        assert_eq!(sum, 0.0, "superposition must cancel exactly, got {sum}");
    }
}

/// 5.4 [D] — the resonance band is the **derived** `1/8`, not a tuned value.
///
/// It is `link_stability_phase_variance / (2*pi)`. Asserted against the
/// generated constant so a drift in either would fail here as well as in
/// `build.rs`.
#[test]
fn the_resonance_band_is_derived_from_the_stability_variance() {
    assert_eq!(RESONANCE_BAND, 0.125);
    let derived = (std::f64::consts::PI / 4.0) / (2.0 * std::f64::consts::PI);
    assert_eq!(derived, RESONANCE_BAND);
}

/// 5.5 [D] — the closed-form boundary at equal scale.
///
/// `|a-b| / mean(a,b) = 1/8` solves to `b = a*17/15`. A conventional
/// "frequencies are close enough" check has no such closed form to hit.
#[test]
fn the_detuning_boundary_matches_its_closed_form() {
    let a = 440.0;
    let boundary = a * 17.0 / 15.0;
    let d = detuning(a, 1.0, boundary, 1.0).unwrap();
    assert!(
        (d - RESONANCE_BAND).abs() < 1e-12,
        "expected the boundary to land on the band, got {d}"
    );

    assert_eq!(
        resonates(a, 1.0, boundary * 0.999, 1.0).unwrap(),
        Interference::Constructive
    );
    assert_eq!(
        resonates(a, 1.0, boundary * 1.001, 1.0).unwrap(),
        Interference::Destructive
    );
}

/// 5.6 [D] — **the gate reads scale**. Identical nominal frequencies detune
/// when observed far enough apart, because `xi` is strictly decreasing.
///
/// Verified boundaries against `R = 1`: `r ~ 1.1892236` above and
/// `r ~ 0.8241412` below. This is what makes it "scale modulation" and not a
/// frequency test.
///
/// The two boundaries are **not symmetric** about the reference — `+18.92%`
/// against `-17.59%` — because `xi` is not linear. A test written around a
/// symmetric tolerance would be testing a different function.
#[test]
fn identical_frequencies_detune_across_scale() {
    assert_eq!(
        resonates(440.0, 1.0, 440.0, 1.0).unwrap(),
        Interference::Constructive,
        "same scale must resonate exactly"
    );

    // Straddle each boundary from both sides.
    for (scale, expected) in [
        (1.18922, Interference::Constructive),
        (1.18923, Interference::Destructive),
        (0.82415, Interference::Constructive),
        (0.82413, Interference::Destructive),
    ] {
        assert_eq!(
            resonates(440.0, 1.0, 440.0, scale).unwrap(),
            expected,
            "scale {scale} landed on the wrong side; detuning = {}",
            detuning(440.0, 1.0, 440.0, scale).unwrap()
        );
    }

    // Asymmetry, stated rather than assumed.
    let up: f64 = 1.1892236 - 1.0;
    let down: f64 = 1.0 - 0.8241412;
    assert!(
        (up - down).abs() > 0.01,
        "the boundaries are asymmetric because xi is nonlinear: {up} vs {down}"
    );
}

/// 5.7 [D] — the interference and resonance gates are **independent**: they
/// disagree on a pair, so neither is expressible through the other.
#[test]
fn the_gates_disagree_and_are_therefore_both_needed() {
    // Opposed phases, identical frequency and scale.
    assert_eq!(
        evaluate_branch(Phase::Positive, Phase::Negative),
        Interference::Destructive
    );
    assert_eq!(
        resonates(440.0, 1.0, 440.0, 1.0).unwrap(),
        Interference::Constructive
    );

    // And the converse pairing: aligned phases that detune badly.
    assert_eq!(
        evaluate_branch(Phase::Positive, Phase::Positive),
        Interference::Constructive
    );
    assert_eq!(
        resonates(440.0, 1.0, 100.0, 1.0).unwrap(),
        Interference::Destructive
    );
}

/// 5.8 — the gate refuses rather than answering when the mean effective
/// frequency collapses. `xi(r) -> 0`, so far enough out the ratio is `0/0`.
#[test]
fn a_collapsed_pair_is_refused_not_answered() {
    assert!(matches!(
        resonates(1.0, 1e308, 1.0, 1e308),
        Err(KernelError::UndefinedScale { .. })
    ));
    assert!(matches!(
        detuning(1.0, f64::INFINITY, 1.0, f64::INFINITY),
        Err(KernelError::UndefinedScale { .. })
    ));
    // A negative scale is outside xi's domain and surfaces the same way.
    assert!(matches!(
        resonates(440.0, -1.0, 440.0, 1.0),
        Err(KernelError::UndefinedScale { .. })
    ));
}

/// 5.9 — the gate is symmetric in its operands. Superposition does not care
/// which oscillator you name first.
#[test]
fn the_resonance_gate_is_symmetric() {
    for (na, ra, nb, rb) in [
        (440.0, 1.0, 440.0, 1.0),
        (440.0, 1.0, 500.0, 1.0),
        (440.0, 0.5, 900.0, 2.0),
        (12.0, 3.0, 700.0, 0.25),
    ] {
        assert_eq!(
            resonates(na, ra, nb, rb).unwrap(),
            resonates(nb, rb, na, ra).unwrap(),
            "gate asymmetric on ({na},{ra}) vs ({nb},{rb})"
        );
        let (f, r) = (
            detuning(na, ra, nb, rb).unwrap(),
            detuning(nb, rb, na, ra).unwrap(),
        );
        assert_eq!(f, r, "detuning asymmetric");
    }
}

/// 5.10 [D] — the gate survives the region where the naive `xi` diverged.
/// A poisoned correction would make every pair out here resonate or NaN.
#[test]
fn the_resonance_gate_survives_the_former_overflow_region() {
    for r in [710.5, 711.0, 745.0, 1e3, 1e30] {
        let d = detuning(440.0, 1.0, 440.0, r).unwrap();
        assert!(d.is_finite(), "detuning at scale {r} is not finite: {d}");
        assert_eq!(
            resonates(440.0, 1.0, 440.0, r).unwrap(),
            Interference::Destructive,
            "a scale of {r} must detune from the reference"
        );
    }
}
