//! Physics-based assertions for the lattice subsystem.
//!
//! Doctrine: `_mkb/test-doctrine.md`. Plan: `subsystems/lattice/03_tests/output/test-plan.md`.
//!
//! Correctness is evaluated via geometric and wave properties, not binary
//! equality of opaque values. Tests marked **[D]** in the plan are those a
//! conventional Euclidean implementation could not pass — they are what make
//! this suite meaningful rather than decorative.

use lattice::constants::*;
use lattice::tessellation as tess;
use lattice::{LatticeError, LatticeScalar, PoincarePoint};

fn s(v: f64) -> LatticeScalar {
    LatticeScalar::new(v)
}

// ---------------------------------------------------------------- Group 1: (x)

/// 1.1 [D] — Axiom A1. `1 (x) 1 = 2`, BIT-EXACTLY.
///
/// No epsilon: `sinh(arcsinh(1)) == 1` identically, so the result is exactly
/// 2.0 in IEEE-754. An epsilon here would hide a real regression.
#[test]
fn otimes_unit_bifurcation_is_exactly_two() {
    let r = s(1.0).otimes(s(1.0)).expect("1 (x) 1 is inside the domain");
    assert_eq!(
        r.get(),
        2.0,
        "axiom A1 violated: 1 (x) 1 must be exactly 2, got {}",
        r.get()
    );
}

/// 1.2 [D] — the doctrine check made explicit. A classical implementation
/// returns 1 here.
#[test]
fn otimes_is_not_classical_multiplication() {
    let r = s(1.0).otimes(s(1.0)).unwrap();
    assert_ne!(
        r.get(),
        1.0,
        "(x) collapsed to ordinary multiplication — the Lynchpin override is gone"
    );
}

/// 1.3 — hand-computed `2 (x) 3 = 6 + sinh(6*lambda)`.
#[test]
fn otimes_matches_hand_computed_value() {
    let r = s(2.0).otimes(s(3.0)).unwrap().get();
    let expected = 104.994_949_366_116_64_f64;
    assert!(
        ((r - expected) / expected).abs() < 1e-12,
        "2 (x) 3 = {r}, expected {expected}"
    );
}

/// 1.4 [D] — (x) is NOT associative, and not by a small margin.
///
/// Asserting non-associativity is deliberate. A suite that proved associativity
/// would be proving the operator had been implemented as ordinary
/// multiplication. `(2(x)3)(x)4` is finite and enormous; `2(x)(3(x)4)` leaves
/// the domain entirely.
#[test]
fn otimes_is_not_associative() {
    // Left association stays in the domain: 2(x)3 = 104.99, and 104.99*4 = 420
    // which is below the 805.56 limit. Right association does not.
    let left = s(2.0)
        .otimes(s(3.0))
        .and_then(|ab| ab.otimes(s(4.0)))
        .expect("(2(x)3)(x)4 stays inside the domain");
    let inner = s(3.0).otimes(s(4.0)).unwrap();
    let right = s(2.0).otimes(inner);

    assert!(
        left.get().is_finite(),
        "(2(x)3)(x)4 should be finite, got {}",
        left.get()
    );
    assert!(
        matches!(right, Err(LatticeError::Dissonant { .. })),
        "2(x)(3(x)4) should leave the domain, got {right:?}"
    );
}

/// 1.5 — the domain guard is enforced, not documented.
#[test]
fn otimes_rejects_products_outside_the_domain() {
    let over = s(OTIMES_DOMAIN_MAX_PRODUCT + 1.0).otimes(s(1.0));
    assert!(
        matches!(over, Err(LatticeError::Dissonant { .. })),
        "product above the domain limit must be rejected, got {over:?}"
    );

    let under = s(OTIMES_DOMAIN_MAX_PRODUCT - 1.0).otimes(s(1.0));
    assert!(
        under.is_ok(),
        "product just inside the domain must be accepted, got {under:?}"
    );
}

/// 1.6 — an `Ok` from (x) is never non-finite. If this fails, the bound is wrong.
#[test]
fn otimes_ok_results_are_always_finite() {
    for a in [0.5, 1.0, 2.0, 10.0, 27.0] {
        for b in [0.5, 1.0, 2.0, 10.0, 27.0] {
            if let Ok(r) = s(a).otimes(s(b)) {
                assert!(
                    r.get().is_finite(),
                    "{a} (x) {b} returned Ok with non-finite {}",
                    r.get()
                );
            }
        }
    }
}

// ------------------------------------------- Group 1b: the numerical inverse
//
// `oslash` is explicitly not a true inverse of `otimes` (see its own doc
// comment) - `solve_otimes` is what "unwinding a path needs a numerical
// solve" actually meant. Verified by direct numerical sweep before any of
// this was written, matching this workspace's rule against speculative math.

/// 1.7 [D] — the canonical identity, solved rather than looked up:
/// `solve_otimes(1, 2)` must recover `x = 1`, since `1 (x) 1 = 2` bit-exactly.
#[test]
fn solving_the_canonical_identity_recovers_one() {
    let x = s(1.0).solve_otimes(s(2.0)).unwrap();
    assert!((x.get() - 1.0).abs() < 1e-9);
}

/// 1.8 [D] — **round-trip: `solve_otimes(a, a.otimes(x))` recovers `x`**,
/// across a wide sweep of signs and magnitudes, including near the `(x)`
/// domain edge where the forward function's slope is astronomically steep.
///
/// This is the property that actually matters — not that the solver runs,
/// but that it inverts. `1e-6` relative tolerance, well above the `~4e-14`
/// worst case measured independently before this test was written.
#[test]
fn solve_otimes_round_trips_across_signs_and_scales() {
    let mut checked = 0;
    for &a in &[-50.0, -5.0, -1.0, -0.3, -0.01, 0.01, 0.3, 1.0, 2.0, 5.0, 50.0] {
        for &x in &[-100.0, -10.0, -1.0, -0.5, -0.01, 0.01, 0.5, 1.0, 3.0, 10.0, 100.0] {
            let Ok(target) = s(a).otimes(s(x)) else {
                continue; // outside (x)'s own domain for this pair - not this test's concern
            };
            let recovered = s(a).solve_otimes(target).unwrap_or_else(|e| {
                panic!("a={a} x={x} target={target:?}: solve_otimes refused unexpectedly: {e}")
            });
            let rel = ((recovered.get() - x) / x.abs().max(1e-9)).abs();
            assert!(
                rel < 1e-6,
                "a={a} x={x}: recovered {} from target {target:?}, relative error {rel:e}",
                recovered.get()
            );
            checked += 1;
        }
    }
    assert!(checked > 80, "expected a meaningful sample, checked {checked}");
}

/// 1.9 [D] — **verified right at the domain edge**, for both signs of `a`.
/// This is exactly where a plain Newton iteration was tried and rejected
/// during derivation — `sinh`'s derivative there is large enough that Newton
/// steps overshoot outside the domain and diverge, even though the solution
/// is unique and well-behaved. Bisection cannot diverge, and this is the
/// assertion that would catch a regression back to Newton alone.
#[test]
fn solve_otimes_stays_accurate_at_the_domain_edge() {
    for a in [1.0, -1.0] {
        for x in [700.0, 800.0, 805.0] {
            let xv = if a > 0.0 { x } else { -x };
            let target = s(a).otimes(s(xv)).expect("chosen to stay just inside the domain");
            let recovered = s(a).solve_otimes(target).unwrap();
            let rel = ((recovered.get() - xv) / xv).abs();
            assert!(
                rel < 1e-9,
                "a={a} x={xv}: relative error {rel:e} at the domain edge, target={target:?}"
            );
        }
    }
}

/// 1.10 — `a = 0` has no unique inverse: `0 (x) x = 0` for every `x`.
#[test]
fn solving_with_zero_a_is_refused_as_degenerate() {
    assert!(matches!(
        s(0.0).solve_otimes(s(5.0)),
        Err(LatticeError::DegenerateInverse { .. })
    ));
}

/// 1.11 — a target far outside what `a (x) x` can reach is refused, not
/// answered with the nearest representable guess.
#[test]
fn an_unreachable_target_is_refused_not_guessed() {
    assert!(matches!(
        s(1.0).solve_otimes(s(f64::MAX)),
        Err(LatticeError::UnreachableTarget { .. })
    ));
}

/// 1.12 — solving is deterministic: the same `(a, target)` always returns
/// the same `x`, not merely one that happens to be close enough.
#[test]
fn solve_otimes_is_deterministic() {
    let target = s(3.0).otimes(s(0.7)).unwrap();
    let first = s(3.0).solve_otimes(target).unwrap();
    for _ in 0..20 {
        assert_eq!(s(3.0).solve_otimes(target).unwrap(), first);
    }
}

// ----------------------------------------------------- Group 2: hyperbolic d_H

fn p(c: [f64; 4]) -> PoincarePoint {
    PoincarePoint::new(c).expect("test point must be inside the ball")
}

/// 2.1 — `d(u,u) = 0` exactly; the arcosh argument is exactly 1.
#[test]
fn distance_to_self_is_exactly_zero() {
    let u = p([0.3, -0.2, 0.1, 0.05]);
    assert_eq!(u.distance_to(&u), 0.0);
}

/// 2.2 — symmetry, exactly. The expression is symmetric in its operands.
#[test]
fn distance_is_exactly_symmetric() {
    let u = p([0.5, 0.0, 0.0, 0.0]);
    let v = p([0.0, 0.5, 0.0, 0.0]);
    assert_eq!(u.distance_to(&v), v.distance_to(&u));
}

/// 2.3 — against the closed form `2*atanh(r)`.
///
/// Epsilon is measured, not guessed: the arcosh route and `ln 3` differ by
/// 2 ulp, so 1e-15 sits just above the observed gap.
#[test]
fn distance_from_origin_matches_closed_form() {
    let d = PoincarePoint::origin().distance_to(&p([0.5, 0.0, 0.0, 0.0]));
    let closed = 2.0 * 0.5_f64.atanh();
    assert!(
        (d - closed).abs() < 1e-15,
        "d = {d}, closed form 2*atanh(0.5) = {closed}, gap {}",
        (d - closed).abs()
    );
}

/// 2.4 — triangle inequality, with slack for three accumulated arcosh calls.
#[test]
fn distance_satisfies_triangle_inequality() {
    let u = p([0.5, 0.0, 0.0, 0.0]);
    let v = p([0.2, 0.3, 0.1, 0.0]);
    let w = p([0.0, 0.5, 0.0, 0.0]);
    let direct = u.distance_to(&w);
    let via = u.distance_to(&v) + v.distance_to(&w);
    assert!(
        direct <= via + 1e-12,
        "triangle inequality violated: d(u,w) = {direct} > {via}"
    );
}

/// 2.5 [D] — the sharpest doctrine discriminator in the suite.
///
/// Distance diverges as the boundary is approached. A Euclidean metric on the
/// unit ball stays bounded by 1; this passes 9.9 and keeps climbing.
#[test]
fn distance_diverges_at_the_ball_boundary() {
    let o = PoincarePoint::origin();
    let mut prev = 0.0;
    for n in [0.9, 0.99, 0.999, 0.9999] {
        let d = o.distance_to(&p([n, 0.0, 0.0, 0.0]));
        assert!(
            d > prev,
            "distance must increase monotonically toward the boundary: {d} !> {prev}"
        );
        prev = d;
    }
    assert!(
        prev > 9.9,
        "at ||u|| = 0.9999 the geodesic distance should exceed 9.9, got {prev} \
         — a Euclidean metric would be bounded by 1"
    );
}

/// 2.6 [D] — hyperbolic distance strictly exceeds the Euclidean chord.
#[test]
fn hyperbolic_distance_exceeds_euclidean() {
    let a = [0.5, 0.0, 0.0, 0.0];
    let b = [0.0, 0.5, 0.0, 0.0];
    let d_h = p(a).distance_to(&p(b));
    let d_e = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt();
    assert!(
        d_h > d_e,
        "hyperbolic distance {d_h} must exceed Euclidean {d_e}"
    );
}

// ------------------------------------------------ Group 3: constructor invariants

/// 3.1 / 3.2 — the boundary and beyond are not in the space.
#[test]
fn point_rejects_boundary_and_beyond() {
    assert!(matches!(
        PoincarePoint::new([1.0, 0.0, 0.0, 0.0]),
        Err(LatticeError::Unmappable { .. })
    ));
    assert!(matches!(
        PoincarePoint::new([1.5, 0.0, 0.0, 0.0]),
        Err(LatticeError::Unmappable { .. })
    ));
    assert!(matches!(
        PoincarePoint::new([0.6, 0.6, 0.6, 0.6]), // norm 1.2
        Err(LatticeError::Unmappable { .. })
    ));
}

/// 3.3 — the invariant must not be over-tight.
#[test]
fn point_accepts_just_inside_the_boundary() {
    assert!(PoincarePoint::new([0.9999, 0.0, 0.0, 0.0]).is_ok());
}

/// 3.4 — NaN and infinity.
///
/// A naive `norm < 1.0` check would **accept** NaN, since every comparison
/// against NaN is false. Targeted explicitly.
#[test]
fn point_rejects_nan_and_infinity() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            matches!(
                PoincarePoint::new([bad, 0.0, 0.0, 0.0]),
                Err(LatticeError::Unmappable { .. })
            ),
            "{bad} must be rejected as a coordinate"
        );
    }
}

// ----------------------------------------------------- Group 4: tessellation

/// 4.1 / 4.2 — the reconciliation R3 decision, pinned so it cannot revert to 5.
#[test]
fn tessellation_identity_is_five_four_with_degree_four() {
    assert_eq!(SCHLAFLI, (5, 4));
    assert_eq!(
        VERTEX_DEGREE, 4,
        "vertex degree must be 4 — Schlafli {{5,4}} means FOUR pentagons per \
         vertex. See _mkb/reconciliation.md R3."
    );
    assert_eq!(EDGES_PER_CELL, 5);
}

/// 4.3 [D] — hyperbolicity. Euclidean tilings give exactly 4.
#[test]
fn tessellation_is_hyperbolic() {
    assert!(tess::is_hyperbolic());
    let (p, q) = SCHLAFLI;
    assert_eq!((p - 2) * (q - 2), 6, "expected (p-2)(q-2) = 6 for {{5,4}}");
}

/// 4.4 — interior angle is exactly pi/2.
#[test]
fn interior_angle_is_exactly_half_pi() {
    assert_eq!(tess::interior_angle(), std::f64::consts::FRAC_PI_2);
}

/// 4.5 [D] — Gauss-Bonnet. A hyperbolic cell's area is fixed by its angles
/// alone, with no free scale parameter. There is no Euclidean analogue of this
/// assertion, which is exactly why it belongs in the suite.
#[test]
fn cell_area_equals_half_pi_by_gauss_bonnet() {
    let area = tess::cell_area();
    assert!(
        (area - std::f64::consts::FRAC_PI_2).abs() < 1e-15,
        "cell area {area} should be pi/2 by Gauss-Bonnet at K = -1"
    );
}

/// 4.6 / 4.7 — closed-form radii of the fundamental right triangle.
#[test]
fn tessellation_radii_match_closed_forms() {
    assert!((tess::circumradius() - 0.842_482_081_462_008).abs() < 1e-12);
    assert!((tess::inradius() - 0.626_869_662_906_178).abs() < 1e-12);
    assert!((tess::half_edge_length() - 0.530_637_530_952_517_6).abs() < 1e-12);
}

/// 4.8 — the inradius is **less** than the circumradius.
///
/// This test previously asserted the opposite and justified it in a comment as
/// "counter-intuitive but correct". It was neither: a centre-to-edge distance
/// cannot exceed a centre-to-vertex distance in any geometry. The old
/// `circumradius()` was returning the half-edge length.
#[test]
fn inradius_is_less_than_circumradius() {
    assert!(
        tess::inradius() < tess::circumradius(),
        "centre-to-edge ({}) must be less than centre-to-vertex ({})",
        tess::inradius(),
        tess::circumradius()
    );
}

/// 4.9 — hyperbolic Pythagoras ties the three together: `cosh c = cosh a cosh b`.
///
/// This is the relation that would have caught the swap immediately, so it is
/// now pinned.
#[test]
fn radii_satisfy_hyperbolic_pythagoras() {
    let c = tess::circumradius().cosh();
    let ab = tess::half_edge_length().cosh() * tess::inradius().cosh();
    assert!(
        (c - ab).abs() < 1e-12,
        "cosh(circumradius) = {c} should equal cosh(half_edge)*cosh(inradius) = {ab}"
    );
}

/// Curvature is native, which is what makes the distance formula valid.
#[test]
fn curvature_is_native_minus_one() {
    assert_eq!(CURVATURE_K, -1.0);
    assert_eq!(LATTICE_SCALE_R, 1.0);
}

/// Cell adjacency: faces vs vertices are different counts, and confusing them
/// is the likeliest adjacency bug.
#[test]
fn cell_adjacency_counts_are_distinct() {
    let c = tess::Cell::at_origin();
    assert_eq!(c.neighbor_count(), 5, "a pentagon has five face-neighbours");
    assert_eq!(c.cells_per_vertex(), 4, "four cells meet at each vertex");
    assert_ne!(c.neighbor_count(), c.cells_per_vertex());
}
