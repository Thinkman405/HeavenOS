//! Parallel media crystallization.
//!
//! Doctrine: `_mkb/test-doctrine.md`. `crystallisation::parallel`'s own
//! module docs make the central claim: every pipeline in this crate is a
//! pure function over owned data, so a batch run concurrently must return
//! results bit-for-bit identical to the same batch run sequentially — no
//! tolerance, no "close enough". These tests check exactly that, for all
//! three pipelines, plus a genuine (not merely claimed) real speedup.

use crystallisation::{
    crystallize_images, crystallize_videos, embed_audio, takens_embed, FrequencyMap, PixelGrid,
    VolumetricTimeCrystal,
};
use std::time::Instant;

fn make_grid(size: usize, seed: f64) -> PixelGrid {
    let pixels: Vec<f64> = (0..size * size)
        .map(|i| ((i as f64 * 0.017 + seed).sin() * 100.0).abs())
        .collect();
    PixelGrid::new(size, size, pixels).unwrap()
}

fn make_signal(len: usize, freq: f64) -> Vec<f64> {
    (0..len).map(|i| (i as f64 * freq).sin()).collect()
}

/// Scaled the same way `tests/crystallisation_codec.rs`'s own
/// `quantisable_frame` is — real 8-bit-scale pixel values overflow the
/// Howard-Comma quantisable ceiling long before a video's worth of frames
/// finishes, so a real video test has to work in this rescaled regime, not
/// invent its own.
const QUANTISABLE_AMPLITUDE: f64 = 2.0e-8;
fn quantisable_frame(w: usize, h: usize, value: f64) -> PixelGrid {
    PixelGrid::new(h, w, vec![value * QUANTISABLE_AMPLITUDE; w * h]).unwrap()
}
fn quantisable_video(frame_count: usize, w: usize, h: usize) -> Vec<PixelGrid> {
    (0..frame_count)
        .map(|i| quantisable_frame(w, h, 1.0 + (i as f64 * 0.3).sin()))
        .collect()
}

// ------------------------------------------------------------------ images

#[test]
fn parallel_image_crystallization_matches_sequential_bit_for_bit() {
    let grids: Vec<PixelGrid> = (0..6).map(|i| make_grid(16, i as f64 * 0.7)).collect();
    let sequential: Vec<_> = grids
        .iter()
        .map(|g| FrequencyMap::transform(g).project_onto_faces())
        .collect();
    let parallel = crystallize_images(grids);
    assert_eq!(
        sequential, parallel,
        "which thread crystallised an image must not change its result"
    );
}

/// Deliberately mixed costs so real threads cannot possibly all finish in
/// input order: a slow non-power-of-two job first, three fast ones after.
/// If the batch function ever indexed by arrival instead of input position,
/// this is the shape of test that would catch it and the one above would
/// not, since same-size jobs there give no real completion-order variance.
#[test]
fn parallel_image_crystallization_preserves_input_order_even_when_jobs_finish_out_of_order() {
    let grids = vec![
        make_grid(90, 1.0),
        make_grid(8, 2.0),
        make_grid(8, 3.0),
        make_grid(8, 4.0),
    ];
    let sequential: Vec<_> = grids
        .iter()
        .map(|g| FrequencyMap::transform(g).project_onto_faces())
        .collect();
    let parallel = crystallize_images(grids);
    assert_eq!(
        sequential, parallel,
        "results must stay indexed to their input position, not arrival order"
    );
}

/// The actual concurrency claim, not just correctness: real OS threads must
/// make a batch of substantial, independent jobs meaningfully faster than
/// running them one after another. `size = 48` is deliberately not a power
/// of two — it lands on `FrequencyMap`'s exact `O(N^2)` fallback per axis,
/// giving each job real, substantial CPU cost (measured ~150-250ms on this
/// machine) rather than the fast radix-2 path's near-instant one.
///
/// The bound is deliberately generous, not tight: measured directly on this
/// machine (4 logical cores) across several runs, the parallel/sequential
/// ratio for this exact workload ranged 0.44-0.77. `0.85` leaves real margin
/// for a slower or more contended environment while still failing outright
/// if this ever silently degenerated into a sequential loop wearing a
/// `thread::spawn` name.
#[test]
fn parallel_image_crystallization_is_genuinely_faster_than_sequential() {
    let size = 48;
    let grids: Vec<PixelGrid> = (0..4).map(|i| make_grid(size, i as f64)).collect();

    let t0 = Instant::now();
    for g in &grids {
        let _ = FrequencyMap::transform(g).project_onto_faces();
    }
    let sequential = t0.elapsed();

    let t1 = Instant::now();
    let results = crystallize_images(grids);
    let parallel = t1.elapsed();
    for r in &results {
        assert!(r.is_ok(), "unexpected crystallization failure: {r:?}");
    }

    assert!(
        parallel < sequential.mul_f64(0.85),
        "expected a real parallel speedup: sequential {sequential:?}, parallel {parallel:?}"
    );
}

// ------------------------------------------------------------------- audio

#[test]
fn parallel_audio_embedding_matches_sequential_bit_for_bit() {
    let signals: Vec<(Vec<f64>, usize)> = (0..5)
        .map(|i| (make_signal(64, 0.1 + i as f64 * 0.02), 3))
        .collect();
    let sequential: Vec<_> = signals.iter().map(|(s, tau)| takens_embed(s, *tau)).collect();
    let parallel = embed_audio(signals);
    assert_eq!(
        sequential, parallel,
        "which thread embedded a signal must not change its result"
    );
}

// ------------------------------------------------------------------- video

#[test]
fn parallel_video_crystallization_matches_sequential_bit_for_bit() {
    let jobs: Vec<(Vec<PixelGrid>, f64, usize)> = (0..4)
        .map(|i| (quantisable_video(20, 2, 2), 30.0, 2 + i % 2))
        .collect();
    let sequential: Vec<_> = jobs
        .iter()
        .cloned()
        .map(|(frames, fps, tau)| VolumetricTimeCrystal::crystallise_video(frames, fps, tau))
        .collect();
    let parallel = crystallize_videos(jobs);
    assert_eq!(
        sequential, parallel,
        "which thread crystallised a video must not change its result"
    );
}
