//! Physics assertions for the GUI geometry layer.
//!
//! Doctrine: `_mkb/test-doctrine.md`. Plan:
//! `subsystems/gui/03_tests/output/test-plan.md`.
//!
//! **[D]** marks assertions a conventional Euclidean renderer could not pass -
//! one drawing straight edges and scaling to zoom.

use gui::constants::*;
use gui::fractal::{euclidean_scale_distances, pairwise_distances};
use gui::visualization::{classify, superpose_phases, Interference, StandingWave};
use gui::{isometry_floor, GeodesicEdge, GuiError, Tetryen, TetryenState, Viewport};
use lattice::PoincarePoint;

/// Geodesic membership floor.
///
/// **Not slack.** `acosh'(x)` is unbounded as `x -> 1`, so `acosh(1+eps) ~
/// sqrt(2 eps)`: a `1e-16` representation error surfaces as `~1e-8` here.
/// Measured floor `2.1e-08`. Demanding `1e-15` would fail for reasons unrelated
/// to the renderer - and this is still sharp, because a straight edge misses by
/// `3e-3`.
const GEODESIC_FLOOR: f64 = 1e-7;

fn p(c: [f64; 4]) -> PoincarePoint {
    PoincarePoint::new(c).expect("test point is inside the ball")
}

// ------------------------------------------------- Group 1: geodesic edges

/// 1.1 [D] - sampled points lie **on** the geodesic.
#[test]
fn sampled_points_lie_on_the_geodesic() {
    let e = GeodesicEdge::new(p([0.30, 0.10, 0.0, 0.0]), p([-0.20, 0.45, 0.10, 0.0])).unwrap();
    for i in 0..=20 {
        let f = i as f64 / 20.0;
        let pt = e.point_at(f).unwrap();
        let dev = e.deviation_at(&pt);
        assert!(
            dev.abs() <= GEODESIC_FLOOR,
            "point at fraction {f} deviates by {dev}"
        );
    }
}

/// 1.2 [D] - **a Euclidean chord fails the same test.**
///
/// The assertion that separates a hyperbolic renderer from a flat one. The
/// chord is strictly longer, by five orders of magnitude more than the floor.
#[test]
fn euclidean_chord_is_not_a_geodesic() {
    let (a, b) = (p([0.30, 0.10, 0.0, 0.0]), p([-0.20, 0.45, 0.10, 0.0]));
    let e = GeodesicEdge::new(a, b).unwrap();
    let (ca, cb) = (a.coords(), b.coords());

    let mut worst: f64 = 0.0;
    for f in [0.25, 0.5, 0.75] {
        let chord = p(std::array::from_fn(|i| ca[i] + f * (cb[i] - ca[i])));
        let dev = e.deviation_at(&chord);
        assert!(
            dev > 1e-4,
            "a straight edge must fail geodesic membership; deviation was {dev}"
        );
        worst = worst.max(dev);
    }
    assert!(
        worst > GEODESIC_FLOOR * 1000.0,
        "the chord must miss by far more than the numerical floor, got {worst}"
    );
}

/// 1.3 - endpoints are reproduced.
#[test]
fn edge_endpoints_are_exact() {
    let (a, b) = (p([0.2, 0.0, 0.0, 0.0]), p([0.0, 0.35, 0.0, 0.0]));
    let e = GeodesicEdge::new(a, b).unwrap();
    assert!(e.point_at(0.0).unwrap().distance_to(&a) < GEODESIC_FLOOR);
    assert!(e.point_at(1.0).unwrap().distance_to(&b) < GEODESIC_FLOOR);
}

/// 1.4 - the midpoint bisects.
#[test]
fn midpoint_bisects_the_edge() {
    let (a, b) = (p([0.25, 0.05, 0.0, 0.0]), p([-0.15, 0.30, 0.0, 0.0]));
    let e = GeodesicEdge::new(a, b).unwrap();
    let m = e.point_at(0.5).unwrap();
    let half = e.length() / 2.0;
    assert!((a.distance_to(&m) - half).abs() < 1e-9);
    assert!((m.distance_to(&b) - half).abs() < 1e-9);
}

/// 1.5 - length is `lattice`'s metric, not a local recomputation.
#[test]
fn edge_length_matches_lattice_metric() {
    let (a, b) = (p([0.3, 0.1, 0.0, 0.0]), p([-0.2, 0.4, 0.0, 0.0]));
    let e = GeodesicEdge::new(a, b).unwrap();
    assert!((e.length() - a.distance_to(&b)).abs() < 1e-12);
}

/// 1.6 - coincident endpoints are refused, not silently `NaN`.
#[test]
fn degenerate_edge_is_refused() {
    let a = p([0.2, 0.0, 0.0, 0.0]);
    assert!(matches!(
        GeodesicEdge::new(a, a),
        Err(GuiError::DegenerateEdge)
    ));
}

/// 1.7 - sampling advances monotonically along the edge.
#[test]
fn sampling_is_monotone() {
    let e = GeodesicEdge::new(p([0.3, 0.0, 0.0, 0.0]), p([-0.3, 0.2, 0.0, 0.0])).unwrap();
    let pts = e.sample(20).unwrap();
    let mut prev = 0.0;
    for pt in &pts {
        let d = e.from_point().distance_to(pt);
        assert!(d >= prev - GEODESIC_FLOOR, "sampling doubled back");
        prev = d;
    }
}

// ------------------------------------------------------ Group 2: the Tetryen

/// 2.1 - four nodes, six edges. Structural.
#[test]
fn tetryen_has_four_nodes_and_six_edges() {
    let t = Tetryen::new(0.5).unwrap();
    assert_eq!(t.nodes().len(), 4);
    assert_eq!(t.edges().unwrap().len(), 6);
}

/// 2.2 / 2.3 - regular: equal edges, equal radii.
#[test]
fn tetryen_is_regular() {
    for r in [0.3, 0.842_482_081_462_008, 1.5] {
        let t = Tetryen::new(r).unwrap();
        assert!(
            t.edge_spread() < 1e-12,
            "edge spread {} at radius {r}",
            t.edge_spread()
        );
        assert!(t.is_regular(1e-12));

        let origin = PoincarePoint::origin();
        let radii: Vec<f64> = t.nodes().iter().map(|n| origin.distance_to(n)).collect();
        let spread = radii.iter().cloned().fold(f64::MIN, f64::max)
            - radii.iter().cloned().fold(f64::MAX, f64::min);
        assert!(spread < 1e-12, "radius spread {spread} at radius {r}");
    }
}

/// 2.4 [D] - every one of the six edges is a geodesic.
#[test]
fn every_tetryen_edge_is_geodesic() {
    let t = Tetryen::new(0.6).unwrap();
    for (n, e) in t.edges().unwrap().iter().enumerate() {
        for i in 1..10 {
            let pt = e.point_at(i as f64 / 10.0).unwrap();
            assert!(
                e.deviation_at(&pt).abs() <= GEODESIC_FLOOR,
                "edge {n} is not geodesic at fraction {}",
                i as f64 / 10.0
            );
        }
    }
}

/// 2.5 / 2.6 - node amplitude follows `psi(r)`, and vanishes at the centre.
#[test]
fn node_amplitude_follows_the_standing_wave() {
    let t = Tetryen::new(0.5).unwrap();
    assert_eq!(t.node_amplitude(0.0), 0.0, "psi(0) must be exactly 0");
    for r in [0.25_f64, 1.0, 2.5] {
        let expected = (r / LATTICE_SCALE_R).sinh() * (-(r / LATTICE_SCALE_R)).exp();
        assert!((t.node_amplitude(r) - expected).abs() < 1e-12);
    }
}

/// 2.7 - an invalid radius is refused.
#[test]
fn invalid_radius_is_refused() {
    for r in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            matches!(Tetryen::new(r), Err(GuiError::InvalidRadius { .. })),
            "radius {r} must be refused"
        );
    }
}

/// 2.8 - regularity survives translation. It is a property of the shape.
#[test]
fn tetryen_stays_regular_off_origin() {
    let centre = p([0.25, 0.15, 0.0, 0.0]);
    let t = Tetryen::at(&centre, 0.5).unwrap();
    assert!(
        t.edge_spread() < 1e-9,
        "off-origin edge spread {}",
        t.edge_spread()
    );
    let base = Tetryen::new(0.5).unwrap();
    assert!((t.edge_length() - base.edge_length()).abs() < 1e-9);
}

// --------------------------------------------- Group 3: fractal navigation

/// 3.1 [D] - **translation preserves every distance.**
///
/// The heart of "infinite resolution scaling". Nothing is magnified; the
/// observer moves.
#[test]
fn navigation_preserves_all_distances() {
    let t = Tetryen::new(0.5).unwrap();
    let before = pairwise_distances(t.nodes());

    for distance in [0.5_f64, 1.0, 3.0, 6.0] {
        let mut v = Viewport::identity();
        v.translate([1.0, 0.0, 0.0, 0.0], distance).unwrap();
        let moved: Vec<PoincarePoint> =
            t.nodes().iter().map(|n| v.project(n).unwrap()).collect();
        let after = pairwise_distances(&moved);

        let floor = v.isometry_floor();
        for (b, a) in before.iter().zip(after.iter()) {
            assert!(
                (b - a).abs() <= floor,
                "distance changed by {} at translation {distance} (floor {floor})",
                (b - a).abs()
            );
        }
    }
}

/// 3.2 [D] - a Euclidean scaling would **not** preserve distances.
///
/// Makes the contrast explicit: scaling is a different operation, not a cheaper
/// approximation of navigation.
#[test]
fn euclidean_scaling_is_a_different_operation() {
    let t = Tetryen::new(0.5).unwrap();
    let before = pairwise_distances(t.nodes());
    let scaled = euclidean_scale_distances(&before, 2.0);

    for (b, s) in before.iter().zip(scaled.iter()) {
        assert!(
            (s - b * 2.0).abs() < 1e-12,
            "scaling must multiply distances"
        );
        assert!(
            (s - b).abs() > 0.1,
            "and therefore must NOT preserve them, unlike an isometry"
        );
    }
}

/// 3.3 - the isometry floor scales with distance travelled.
#[test]
fn isometry_floor_grows_with_distance() {
    let near = isometry_floor(0.5);
    let far = isometry_floor(6.0);
    assert!(
        far > near * 100.0,
        "floor must grow with translation: near {near}, far {far}"
    );
}

/// 3.4 - rotation preserves distances too.
#[test]
fn rotation_preserves_distances() {
    let t = Tetryen::new(0.5).unwrap();
    let before = pairwise_distances(t.nodes());
    let mut v = Viewport::identity();
    v.rotate(0, 1, 0.7);
    let moved: Vec<PoincarePoint> = t.nodes().iter().map(|n| v.project(n).unwrap()).collect();
    for (b, a) in before.iter().zip(pairwise_distances(&moved).iter()) {
        assert!((b - a).abs() < 1e-12, "rotation changed a distance by {}", b - a);
    }
}

/// 3.5 - composed moves stay isometric; error must not compound into distortion.
#[test]
fn composed_navigation_stays_isometric() {
    let t = Tetryen::new(0.5).unwrap();
    let before = pairwise_distances(t.nodes());
    let mut v = Viewport::identity();
    for i in 0..5 {
        v.translate([1.0, 0.5, 0.0, 0.0], 0.4).unwrap();
        v.rotate(0, 2, 0.3 * f64::from(i));
    }
    let moved: Vec<PoincarePoint> = t.nodes().iter().map(|n| v.project(n).unwrap()).collect();
    let floor = v.isometry_floor();
    for (b, a) in before.iter().zip(pairwise_distances(&moved).iter()) {
        assert!(
            (b - a).abs() <= floor,
            "composed moves distorted by {} (floor {floor})",
            (b - a).abs()
        );
    }
}

/// 3.6 - an isometry cannot push a point out of the space.
#[test]
fn projected_points_stay_in_the_ball() {
    let t = Tetryen::new(0.5).unwrap();
    let mut v = Viewport::identity();
    v.translate([1.0, 0.0, 0.0, 0.0], 6.0).unwrap();
    for n in t.nodes() {
        assert!(v.project(n).unwrap().norm() < 1.0);
    }
}

// ------------------------------------- Group 4: interference visualisation

/// 4.1 - zero load renders zero amplitude.
#[test]
fn zero_load_renders_nothing() {
    let w = StandingWave::for_load(0.0, 1.2, 3.0);
    assert_eq!(w.peak(), 0.0);
    for t in [0.0, 0.3, 1.7] {
        assert_eq!(w.at(0.8, t), 0.0);
    }
}

/// 4.2 - amplitude is linear in load.
#[test]
fn amplitude_is_linear_in_load() {
    let a = StandingWave::for_load(0.5, 1.2, 3.0);
    let b = StandingWave::for_load(1.0, 1.2, 3.0);
    assert!((b.peak() / a.peak() - 2.0).abs() < 1e-12);
    assert!((a.at(0.9, 0.4) * 2.0 - b.at(0.9, 0.4)).abs() < 1e-12);
}

/// 4.3 - the wave matches `2A sin(kx) cos(wt)`.
#[test]
fn standing_wave_matches_closed_form() {
    let (amp, k, w, x, t) = (0.75_f64, 1.2_f64, 3.0_f64, 0.9_f64, 0.4_f64);
    let wave = StandingWave::new(amp, k, w);
    let expected = 2.0 * amp * (k * x).sin() * (w * t).cos();
    assert!((wave.at(x, t) - expected).abs() < 1e-12);
}

/// 4.4 [D] - opposed phases cancel **exactly**.
///
/// Destructive interference is total, not merely dimmer. A renderer that faded
/// overlapping waves would never reach zero.
#[test]
fn opposed_phases_cancel_exactly() {
    for t in [0.0, 0.25, 0.5, 1.0, 2.5] {
        let s = superpose_phases(0.0, std::f64::consts::PI, 1.0, t);
        assert!(s.abs() < 1e-15, "residual {s} at t = {t}");
    }
    // At t = 0 the cancellation is bit-exact, not merely within tolerance.
    assert_eq!(superpose_phases(0.0, std::f64::consts::PI, 1.0, 0.0), 0.0);
}

/// 4.5 - aligned phases reinforce.
#[test]
fn aligned_phases_reinforce() {
    let single = (0.0_f64).cos();
    let both = superpose_phases(0.0, 0.0, 1.0, 0.0);
    assert!((both - 2.0 * single).abs() < 1e-12);
}

/// 4.6 - classification tracks the phase separation.
#[test]
fn interference_classification_tracks_phase() {
    assert_eq!(classify(std::f64::consts::PI, 1e-9), Interference::Destructive);
    assert_eq!(classify(0.0, 1e-9), Interference::Constructive);
    assert_eq!(classify(0.4, 1e-9), Interference::Constructive);
}

// ------------------------------------- Group 5: consumed, not reimplemented

/// 5.2 - curvature is lattice-native, which is what makes the metric valid.
#[test]
fn curvature_is_lattice_native() {
    assert_eq!(CURVATURE_K, -1.0);
    assert_eq!(LATTICE_SCALE_R, 1.0);
}

// ------------------------------------- Group 6: crystallised media, rendered
//
// PRD §8 crystallises media onto Tetryen structure two ways; these assertions
// join both to the renderer via `TetryenVisualisation`, mirroring how Group 4
// already joins the kernel's load field.

fn holographic_faces() -> [crystallisation::FaceProjection; 4] {
    let g = crystallisation::PixelGrid::new(
        4,
        4,
        vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0,
        ],
    )
    .unwrap();
    crystallisation::FrequencyMap::transform(&g)
        .project_onto_faces()
        .unwrap()
}

/// 6.1 [D] — the busiest face reaches full amplitude, and the others scale
/// **relative** to it rather than to any absolute spectral-energy unit.
///
/// Measured face energies for this grid are `5460, 284, 772, 284` — not
/// remotely close to equal, so this is a real discrimination, not four faces
/// happening to normalise to the same thing.
#[test]
fn busiest_face_reaches_full_amplitude() {
    let faces = holographic_faces();
    let energies: Vec<f64> = faces.iter().map(|f| f.energy()).collect();
    let (busiest, _) = energies
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();

    let vis = gui::TetryenVisualisation::from_face_projections(&faces, 1.2, 3.0);
    assert!(
        (vis.wave(busiest).unwrap().amplitude() - 1.0).abs() < 1e-12,
        "the busiest face must normalise to exactly 1.0"
    );
    for i in 0..4 {
        if i != busiest {
            assert!(
                vis.wave(i).unwrap().amplitude() < 1.0,
                "face {i} must not exceed the busiest face's amplitude"
            );
        }
    }
    // Relative ordering must survive normalisation: face 0 (5460) drove the
    // busiest wave; face 2 (772) must still outdraw faces 1 and 3 (284 each).
    assert!(vis.wave(2).unwrap().amplitude() > vis.wave(1).unwrap().amplitude());
    assert!(vis.wave(2).unwrap().amplitude() > vis.wave(3).unwrap().amplitude());
}

/// 6.2 — exactly four waves, one per Tetryen node; nothing past index 3.
#[test]
fn face_visualisation_has_exactly_four_waves() {
    let faces = holographic_faces();
    let vis = gui::TetryenVisualisation::from_face_projections(&faces, 1.2, 3.0);
    for i in 0..4 {
        assert!(vis.wave(i).is_some(), "node {i} must have a wave");
    }
    assert!(vis.wave(4).is_none(), "a Tetryen has exactly four nodes");
}

/// 6.3 [D] — **phase-space amplitude is driven by magnitude, not sign.**
///
/// A raw signal component can be negative; a strongly negative sample is still
/// strongly *active*, not idle. Two components of equal magnitude and opposite
/// sign must draw identically.
#[test]
fn phase_vector_amplitude_ignores_component_sign() {
    let node = crystallisation::PhaseSpaceVector([2.0, -2.0, 2.0, -2.0]);
    let vis = gui::TetryenVisualisation::from_phase_vector(&node, 1.2, 3.0);
    for i in 0..4 {
        assert!(
            (vis.wave(i).unwrap().amplitude() - 1.0).abs() < 1e-12,
            "node {i}: equal-magnitude components must draw identically regardless of sign"
        );
    }
}

/// 6.4 — a genuinely varied phase-space vector, taken from a real embedded
/// trajectory, discriminates: the largest-magnitude component reaches full
/// amplitude and the rest scale under it.
#[test]
fn phase_vector_amplitude_discriminates_by_magnitude() {
    let signal: Vec<f64> = (0..40).map(|i| (i as f64 * 0.3).sin() * 3.0).collect();
    let nodes = crystallisation::takens_embed(&signal, 2).unwrap();
    let node = nodes[0];
    let components = *node.components();

    let vis = gui::TetryenVisualisation::from_phase_vector(&node, 1.2, 3.0);
    let (busiest, _) = components
        .iter()
        .map(|c| c.abs())
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    assert!(
        (vis.wave(busiest).unwrap().amplitude() - 1.0).abs() < 1e-12,
        "the largest-magnitude component must reach full amplitude"
    );
    assert!(
        vis.peak() > 0.0,
        "a real embedded trajectory must be drawable"
    );
}

/// 6.5 — a silent phase-space vector renders nothing, matching an idle load
/// field's `LoadVisualisation::peak() == 0.0`.
#[test]
fn silent_phase_vector_renders_zero() {
    let node = crystallisation::PhaseSpaceVector([0.0, 0.0, 0.0, 0.0]);
    let vis = gui::TetryenVisualisation::from_phase_vector(&node, 1.2, 3.0);
    assert_eq!(vis.peak(), 0.0);
    for i in 0..4 {
        assert_eq!(vis.wave(i).unwrap().amplitude(), 0.0);
    }
}

// --------------------------------------- Group 7: Tetryen recurrence
//
// `_mkb/tetryen_recurrence.md` — a synthesis closing the undistilled
// corpus's `f(psi_n, psi_{n-1})` placeholder. See that file for the full
// derivation; these assertions check the implementation matches it exactly.

/// [D] When every node starts at the same amplitude, every pairwise
/// difference is exactly zero, so the coupling term vanishes identically
/// **regardless of `gamma`** — the step must reduce to the plain uncoupled
/// identity `psi_{n+1} = 2cos(w dt) psi_n - psi_{n-1}`, matched here
/// against the closed form `cos(phi + w dt)` it is derived from. A
/// conventional ad hoc discretisation would not reproduce this to 1e-9.
#[test]
fn identical_nodes_evolve_by_the_uncoupled_identity_regardless_of_coupling() {
    let tetryen = Tetryen::new(0.5).unwrap();
    let (omega, dt, phi) = (3.0_f64, 0.01_f64, 0.7_f64);
    let prev_val = (phi - omega * dt).cos();
    let curr_val = phi.cos();
    let expected_next = (phi + omega * dt).cos();

    for gamma in [0.0, 1.0, 100.0] {
        let mut state = TetryenState::seeded([curr_val; 4], [prev_val; 4]);
        let next = state.step(&tetryen, omega, dt, gamma).unwrap();
        for &v in &next {
            assert!(
                (v - expected_next).abs() < 1e-9,
                "gamma={gamma}: expected {expected_next}, got {v}"
            );
        }
    }
}

/// Every Tetryen this crate constructs is regular, so every pairwise
/// geodesic distance between its four nodes is identical — and therefore
/// so is every coupling weight. The structural fact `tetryen_recurrence.md`
/// states directly, checked here rather than assumed.
#[test]
fn regular_tetryen_gives_uniform_coupling_weights() {
    let tetryen = Tetryen::new(0.5).unwrap();
    let nodes = tetryen.nodes();
    let mut weights = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            if i != j {
                let d = nodes[i].distance_to(&nodes[j]);
                weights.push(tetryen.node_amplitude(d));
            }
        }
    }
    let first = weights[0];
    assert!(first > 0.0, "premise: coupling weights must be a real, nonzero effect");
    for w in &weights {
        assert!(
            (w - first).abs() < 1e-9,
            "a regular Tetryen must give uniform coupling weights, got {w} vs {first}"
        );
    }
}

/// Coupling is a real, directional effect: an outlier node relaxes toward
/// its lower neighbours, and a low neighbour is pulled up toward the
/// outlier — not merely "coupling changes something," but the correct
/// diffusive sign.
#[test]
fn coupling_pulls_differing_nodes_toward_each_other() {
    let tetryen = Tetryen::new(0.5).unwrap();
    let (omega, dt) = (3.0, 0.01);
    let initial = [10.0, 0.0, 0.0, 0.0];

    let mut with_coupling = TetryenState::at_rest(initial);
    let mut without = TetryenState::at_rest(initial);
    let next_coupled = with_coupling.step(&tetryen, omega, dt, 100.0).unwrap();
    let next_uncoupled = without.step(&tetryen, omega, dt, 0.0).unwrap();

    assert!(
        next_coupled[0] < next_uncoupled[0],
        "the outlier node must relax toward its lower neighbours under coupling"
    );
    assert!(
        next_coupled[1] > next_uncoupled[1],
        "a low neighbour must be pulled up toward the outlier under coupling"
    );
}

/// The recurrence's own documented safe region (`_mkb/tetryen_recurrence.md`
/// §3) stays bounded over a long run — not just briefly.
#[test]
fn stays_bounded_within_the_documented_safe_region() {
    let tetryen = Tetryen::new(0.5).unwrap();
    let mut state = TetryenState::at_rest([1.0, 0.5, -0.3, 0.8]);
    for _ in 0..50_000 {
        let next = state
            .step(&tetryen, 3.0, 0.01, 1.0)
            .expect("documented-safe parameters must not diverge");
        for &v in &next {
            assert!(v.abs() < 100.0, "amplitude {v} escaped a generous bound");
        }
    }
}

/// [D] A step that leaves the measured stability region is refused, not
/// silently propagated as `inf`/`NaN` — the same discipline `otimes`
/// applies at its own domain limit.
#[test]
fn a_step_leaving_the_stability_region_is_refused_not_propagated() {
    let tetryen = Tetryen::new(0.5).unwrap();
    let mut state = TetryenState::at_rest([1.0, 0.5, -0.3, 0.8]);
    let mut diverged = false;
    for _ in 0..5000 {
        match state.step(&tetryen, 3.0, 0.01, 1e6) {
            Ok(_) => {}
            Err(GuiError::Diverged { amplitude }) => {
                assert!(!amplitude.is_finite());
                diverged = true;
                break;
            }
            Err(other) => panic!("unexpected error: {other}"),
        }
    }
    assert!(
        diverged,
        "gamma=1e6 at dt=0.01 is documented to leave the stable region"
    );
}

/// `at_rest` seeds both time slices identically.
#[test]
fn at_rest_seeds_identically() {
    let state = TetryenState::at_rest([1.0, 2.0, 3.0, 4.0]);
    assert_eq!(state.amplitudes(), [1.0, 2.0, 3.0, 4.0]);
}
