//! Physics assertions for application data translation (PRD §8).
//!
//! Doctrine: `_mkb/test-doctrine.md`. **[D]** marks assertions a conventional
//! serialiser could not pass.

use crystallisation::holographic::{FrequencyMap, PixelGrid};
use crystallisation::linguistic::Crystal;
use crystallisation::resonant::ResonantChamber;
use crystallisation::CrystalError;

// ------------------------------------------------- Group 1: linguistic

/// 1.1 — every non-newline character becomes exactly one node, in order.
#[test]
fn characters_become_sequential_nodes() {
    let c = Crystal::crystallise("NEOS").unwrap();
    assert_eq!(c.len(), 4);
    assert_eq!(c.text(), "NEOS");
    for (i, n) in c.nodes().iter().enumerate() {
        assert_eq!(n.index, i, "nodes must be sequential");
    }
}

/// 1.2 [D] — **one line break bifurcates the extent to exactly 2.**
///
/// Axiom A1 via `(x)`, bit-exact. A conventional document model would leave
/// extent unchanged, or double it by copying — neither gives this by the
/// operator.
#[test]
fn one_line_break_gives_extent_exactly_two() {
    let flat = Crystal::crystallise("one line").unwrap();
    assert_eq!(flat.bifurcations(), 0);
    assert_eq!(flat.extent(), 1.0);

    let split = Crystal::crystallise("first\nsecond").unwrap();
    assert_eq!(split.bifurcations(), 1);
    assert_eq!(split.extent(), 2.0, "A1: a unit bifurcation is exactly 2");
}

/// 1.3 [D] — extent grows by **self**-`(x)`, not by doubling.
///
/// A1 says a bifurcation is `u (x) u`, so the second break squares the product
/// rather than stepping by one. A copy-based model gives 4; `2 (x) 2` gives
/// 20.97 — far more divergent than a unit step would.
#[test]
fn bifurcation_is_geometric_not_doubling() {
    let c = Crystal::crystallise("a\nb\nc").unwrap();
    assert_eq!(c.bifurcations(), 2);
    assert_ne!(c.extent(), 4.0, "doubling would give 4");
    assert!(
        (c.extent() - 20.970_562_748_477_143).abs() < 1e-9,
        "expected 2 (x) 2 = 20.9706, got {}",
        c.extent()
    );
}

/// 1.4 [D] — **an over-deep document is refused, never truncated.**
///
/// The ceiling is **3**, not the 4 that iterated *unit* steps give in
/// `lattice` addressing. A1 bifurcation is `u (x) u`, which squares the
/// product each time and so leaves the domain one step sooner. Same systemic
/// cause, different arity — worth pinning so the two are not conflated.
///
/// Losing content silently would be worse than declining.
#[test]
fn over_deep_document_is_refused() {
    let limit = Crystal::max_bifurcations();
    assert_eq!(limit, 3, "measured ceiling for self-(x) bifurcation");

    let at_limit = "a\n".repeat(limit);
    assert!(
        Crystal::crystallise(&at_limit).is_ok(),
        "exactly at the limit must crystallise"
    );

    let past = "a\n".repeat(limit + 1);
    match Crystal::crystallise(&past) {
        Err(CrystalError::TooDeep {
            bifurcations,
            limit: l,
        }) => {
            assert_eq!(bifurcations, limit + 1);
            assert_eq!(l, limit);
        }
        other => panic!("expected TooDeep, got {other:?}"),
    }
}

/// 1.5 — the ceiling is computed from the operator, not hardcoded.
#[test]
fn bifurcation_ceiling_is_derived_from_the_operator() {
    let limit = Crystal::max_bifurcations();
    let mut extent = lattice::LatticeScalar::new(1.0);
    let mut depth = 0;
    while let Ok(next) = extent.otimes(extent) {
        extent = next;
        depth += 1;
        if depth > 64 {
            break;
        }
    }
    assert_eq!(limit, depth, "the limit must track the real (x) domain");
}

/// 1.6 — empty text crystallises to an empty structure, not an error.
#[test]
fn empty_text_is_an_empty_crystal() {
    let c = Crystal::crystallise("").unwrap();
    assert!(c.is_empty());
    assert_eq!(c.bifurcations(), 0);
    assert_eq!(c.extent(), 1.0);
}

/// 1.7 — phases are A2's permitted pair, nothing else.
#[test]
fn node_phases_are_the_permitted_orientations() {
    let c = Crystal::crystallise("Hello, NEOS!").unwrap();
    for n in c.nodes() {
        assert!(
            n.phase == substrate::constants::PHASE_TRUE
                || n.phase == substrate::constants::PHASE_FALSE,
            "node {:?} carries phase {} outside A2's pair",
            n.codepoint,
            n.phase
        );
    }
}

// ------------------------------------------------ Group 2: holographic

fn grid() -> PixelGrid {
    PixelGrid::new(
        4,
        4,
        vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0,
        ],
    )
    .unwrap()
}

/// 2.1 [D] — **Parseval holds**: spatial energy equals frequency energy.
///
/// The property that makes this a representation rather than a summary. A
/// lossy transform fails it.
#[test]
fn parseval_holds() {
    let g = grid();
    let f = FrequencyMap::transform(&g);
    let (spatial, freq) = (g.energy(), f.energy());
    assert!(
        ((spatial - freq) / spatial).abs() < 1e-9,
        "spatial {spatial} vs frequency {freq}"
    );
    assert!((spatial - 425.0).abs() < 1e-9, "hand-computed energy");
}

/// 2.2 — the DC term is the sum of all pixels.
#[test]
fn dc_term_is_the_pixel_sum() {
    let g = grid();
    let f = FrequencyMap::transform(&g);
    assert!((f.dc() - g.sum()).abs() < 1e-9);
}

/// 2.3 [D] — the transform round-trips.
///
/// A frequency map that could not be inverted would be a description of the
/// image, not the image.
#[test]
fn transform_round_trips() {
    let g = grid();
    let back = FrequencyMap::transform(&g).inverse();
    assert_eq!(back.height(), g.height());
    assert_eq!(back.width(), g.width());
    for (a, b) in g.pixels().iter().zip(back.pixels()) {
        assert!((a - b).abs() < 1e-9, "pixel drifted: {a} -> {b}");
    }
}

/// 2.4 [D] — coefficients project onto exactly **four** Tetryen faces.
#[test]
fn projection_covers_four_faces() {
    let f = FrequencyMap::transform(&grid());
    let faces = f.project_onto_faces().unwrap();
    assert_eq!(faces.len(), 4);
    for (i, face) in faces.iter().enumerate() {
        assert_eq!(face.face(), i);
        assert_eq!(face.coefficients().len(), 4, "16 coefficients / 4 faces");
    }
    let total: usize = faces.iter().map(|f| f.coefficients().len()).sum();
    assert_eq!(total, f.coefficients().len(), "no coefficient may be lost");
}

/// 2.5 — an uneven projection is refused, not silently unbalanced.
#[test]
fn uneven_projection_is_refused() {
    // 3x3 = 9 coefficients, not divisible by 4.
    let g = PixelGrid::new(3, 3, vec![1.0; 9]).unwrap();
    let f = FrequencyMap::transform(&g);
    assert!(matches!(
        f.project_onto_faces(),
        Err(CrystalError::UnevenProjection { coefficients: 9 })
    ));
}

/// 2.4b [D] — **Parseval holds on non-square grids**, not just the one square
/// shape every other test in this suite happens to use.
///
/// Every `PixelGrid` built anywhere in this test suite before this one was
/// square (`4x4` or `3x3`), including the grid `parseval_holds` itself uses.
/// `FrequencyMap::energy`'s divisor is `height * width` — on a square grid
/// that is indistinguishable from `width * width` or `height * height`, so a
/// transposition in the divisor would pass every existing test. Verified by
/// sabotage: swapping the divisor to `width * width` left all 363
/// workspace tests green, this one included nowhere among them. Five shapes,
/// including the extremes `1x9`/`9x1`, at a tolerance tight enough
/// (`1e-12`) that only the correct divisor can pass it.
#[test]
fn parseval_holds_on_non_square_grids() {
    for (h, w) in [(3usize, 5usize), (2, 7), (6, 2), (1, 9), (9, 1)] {
        let pixels: Vec<f64> = (0..h * w)
            .map(|i| (i as f64 * 1.7 - 3.0).sin() * 5.0 + i as f64)
            .collect();
        let g = PixelGrid::new(h, w, pixels).unwrap();
        let f = FrequencyMap::transform(&g);
        let (spatial, freq) = (g.energy(), f.energy());
        assert!(
            ((spatial - freq) / spatial).abs() < 1e-12,
            "h={h} w={w}: spatial {spatial} vs frequency {freq}"
        );
    }
}

/// 2.4c [D] — Parseval holds at the **`1xN` shape production code actually
/// uses.** `VolumetricTimeCrystal::crystallise` builds every audio/video
/// signal as `PixelGrid::new(1, signal.len(), signal)` internally — this is
/// not a hypothetical shape, it is the one every VTC test already exercises
/// indirectly, just never asserted directly against `FrequencyMap::energy`
/// at this size. Swept to `N = 2000`, matching realistic signal lengths
/// elsewhere in this workspace, to catch any precision drift the `O(N^2)`
/// naive DFT might accumulate at scale — measured relative error stays under
/// `1e-13` even at the largest size, comfortably inside the assertion.
#[test]
fn parseval_holds_at_production_signal_lengths() {
    for n in [50usize, 200, 400, 800, 2000] {
        let pixels: Vec<f64> = (0..n)
            .map(|i| (i as f64 * 0.31).sin() * 3.0 + (i as f64 * 0.07).cos())
            .collect();
        let g = PixelGrid::new(1, n, pixels).unwrap();
        let f = FrequencyMap::transform(&g);
        let (spatial, freq) = (g.energy(), f.energy());
        assert!(
            ((spatial - freq) / spatial).abs() < 1e-9,
            "n={n}: spatial {spatial} vs frequency {freq}"
        );
    }
}

/// 2.6 — a malformed grid is refused at construction.
#[test]
fn malformed_grid_is_refused() {
    assert!(matches!(
        PixelGrid::new(3, 3, vec![1.0; 8]),
        Err(CrystalError::MalformedGrid { .. })
    ));
    assert!(matches!(
        PixelGrid::new(0, 4, vec![]),
        Err(CrystalError::MalformedGrid { .. })
    ));
}

/// 2.7 — a uniform image concentrates all energy in DC.
#[test]
fn uniform_image_has_only_a_dc_term() {
    let g = PixelGrid::new(4, 4, vec![3.0; 16]).unwrap();
    let f = FrequencyMap::transform(&g);
    assert!((f.dc() - 48.0).abs() < 1e-9);
    for (i, c) in f.coefficients().iter().enumerate().skip(1) {
        assert!(
            c.magnitude() < 1e-9,
            "coefficient {i} should vanish for a flat image, got {}",
            c.magnitude()
        );
    }
}

/// 2.9 [D] — **`FrequencyMap::transform` agrees with an independent, direct
/// `O(N^2)` DFT**, at power-of-two sizes specifically — the shapes that
/// route through the real radix-2 FFT rather than falling back to the exact
/// sum. `parseval_holds`/`transform_round_trips` already exercise the FFT
/// path incidentally (`grid()` is `4x4`), but only check properties the
/// *exact* DFT would also satisfy; this checks the actual coefficients
/// against a second, independently written implementation, the same
/// cross-validation discipline `lattice::pathfinding` vs `ftg`'s own `bfs`
/// already established — not "the formula looks textbook so it must
/// agree," an *actual* second computation.
#[test]
fn fft_matches_an_independent_direct_dft_at_power_of_two_sizes() {
    fn direct_2d_dft(pixels: &[f64], h: usize, w: usize) -> Vec<(f64, f64)> {
        let tau = std::f64::consts::TAU;
        let mut out = vec![(0.0, 0.0); h * w];
        for u in 0..h {
            for v in 0..w {
                let (mut re, mut im) = (0.0, 0.0);
                for y in 0..h {
                    for x in 0..w {
                        let ang = -tau * (u as f64 * y as f64 / h as f64 + v as f64 * x as f64 / w as f64);
                        let p = pixels[y * w + x];
                        re += p * ang.cos();
                        im += p * ang.sin();
                    }
                }
                out[u * w + v] = (re, im);
            }
        }
        out
    }

    for &(h, w) in &[(4usize, 4usize), (8, 2), (1, 16), (2, 8)] {
        let pixels: Vec<f64> = (0..h * w).map(|i| (i as f64 * 1.9).sin() * 4.0 + i as f64 * 0.2).collect();
        let grid = PixelGrid::new(h, w, pixels.clone()).unwrap();
        let expected = direct_2d_dft(&pixels, h, w);
        let actual = FrequencyMap::transform(&grid);
        for (i, (c, (re, im))) in actual.coefficients().iter().zip(expected.iter()).enumerate() {
            assert!(
                (c.re - re).abs() < 1e-8 && (c.im - im).abs() < 1e-8,
                "{h}x{w} coefficient {i}: got ({}, {}), independent DFT gives ({re}, {im})",
                c.re,
                c.im
            );
        }
    }
}

// -------------------------------------------------- Group 3: resonant

/// 3.1 — a tone maps to an oscillator near its true frequency.
#[test]
fn tone_maps_to_an_oscillator() {
    let rate = 8000.0;
    let hz = 200.0;
    let samples: Vec<f64> = (0..8000)
        .map(|i| (std::f64::consts::TAU * hz * i as f64 / rate).sin())
        .collect();
    let chamber = ResonantChamber::from_samples(&samples, rate).unwrap();
    let got = chamber.frequency().get();
    assert!(
        (got - hz).abs() / hz < 0.02,
        "expected ~{hz} Hz, got {got}"
    );
    assert_eq!(chamber.sample_count(), 8000);
    assert!(!chamber.is_silent());
}

/// 3.2 — silence oscillates at nothing.
#[test]
fn silence_is_silent() {
    let chamber = ResonantChamber::from_samples(&[0.0; 512], 8000.0).unwrap();
    assert_eq!(chamber.frequency().get(), 0.0);
    assert!(chamber.is_silent());
}

/// 3.3 — an empty stream has no frequency at all.
#[test]
fn empty_media_is_refused() {
    assert!(matches!(
        ResonantChamber::from_samples(&[], 8000.0),
        Err(CrystalError::EmptyMedia)
    ));
    assert!(matches!(
        ResonantChamber::from_samples(&[1.0], 0.0),
        Err(CrystalError::EmptyMedia)
    ));
}

/// 3.4 — the chamber reports **ordinary** frequency.
///
/// It returns `substrate::Frequency`, so a media rate cannot reach the angular
/// carrier path. Assigning it to that type is the compile-time proof.
#[test]
fn chamber_reports_ordinary_frequency() {
    let chamber = ResonantChamber::from_samples(&[1.0, -1.0, 1.0, -1.0], 4.0).unwrap();
    let f: substrate::Frequency = chamber.frequency();
    assert!(f.get() >= 0.0);
    // And the explicit conversion is the only route to angular.
    assert!((f.to_angular().get() / f.get().max(1e-12) - std::f64::consts::TAU).abs() < 1e-6);
}
