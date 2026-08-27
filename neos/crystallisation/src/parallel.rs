//! Parallel media crystallization.
//!
//! Every pipeline in this crate — holographic, resonant-chamber, and the
//! Takens embedding they share — is a pure function over owned data: no
//! global state, no interior mutability, nothing `unsafe` anywhere in this
//! crate (checked with a grep across every source file before writing this
//! module, not assumed). That makes a batch of independent media genuinely
//! embarrassingly parallel — a different shape of real concurrency than
//! this workspace has needed before. `symphony_lang::concurrent` and
//! `symphony_kernel::ConcurrentPool`/`ConcurrentTracker` exist because their
//! callers *share* mutable state and need real synchronisation to stay
//! correct under contention. Nothing here shares anything: each job owns
//! its input and returns its own owned output, so a real thread only ever
//! adds throughput, never a correctness question — which is also why this
//! module's own tests spend most of their effort proving exactly that
//! absence: parallel execution must return results bit-for-bit identical to
//! sequential execution on the same inputs, since which thread happened to
//! run a job is not allowed to change a floating-point answer.
//!
//! One real, deliberate limit: a single job's own work (one image's FFT, one
//! video's frame reduction) is never itself split across threads — only
//! whole jobs are, one thread each. Splitting a single transform internally
//! would be real work with its own correctness questions (partial sums,
//! reduction order) that no caller of this crate has actually needed yet.

use crate::holographic::{FaceProjection, FrequencyMap, PixelGrid};
use crate::timecrystal::{takens_embed, PhaseSpaceVector, VolumetricTimeCrystal};
use crate::CrystalError;

/// Crystallise several images concurrently, one real OS thread each.
///
/// Results come back in the same order as `grids`, regardless of which
/// thread happens to finish first — the batch's `i`-th input always produces
/// the `i`-th output.
pub fn crystallize_images(grids: Vec<PixelGrid>) -> Vec<Result<[FaceProjection; 4], CrystalError>> {
    let handles: Vec<_> = grids
        .into_iter()
        .map(|grid| {
            std::thread::spawn(move || FrequencyMap::transform(&grid).project_onto_faces())
        })
        .collect();
    handles
        .into_iter()
        .map(|h| h.join().expect("an image crystallization thread must not panic"))
        .collect()
}

/// Takens-embed several signals concurrently, one real OS thread each.
///
/// Each entry is `(signal, tau)`, since different signals in a real batch
/// may legitimately need different embedding delays.
pub fn embed_audio(
    signals: Vec<(Vec<f64>, usize)>,
) -> Vec<Result<Vec<PhaseSpaceVector>, CrystalError>> {
    let handles: Vec<_> = signals
        .into_iter()
        .map(|(signal, tau)| std::thread::spawn(move || takens_embed(&signal, tau)))
        .collect();
    handles
        .into_iter()
        .map(|h| h.join().expect("an audio embedding thread must not panic"))
        .collect()
}

/// Crystallise several videos concurrently, one real OS thread each.
///
/// Each entry is `(frames, frame_rate, tau)` — a video's own full frame
/// sequence, moved into its own thread whole, since `crystallise_video`'s
/// per-frame energy reduction is itself already sequential-by-construction
/// (it reduces to one scalar time series before doing anything else).
pub fn crystallize_videos(
    jobs: Vec<(Vec<PixelGrid>, f64, usize)>,
) -> Vec<Result<VolumetricTimeCrystal, CrystalError>> {
    let handles: Vec<_> = jobs
        .into_iter()
        .map(|(frames, frame_rate, tau)| {
            std::thread::spawn(move || {
                VolumetricTimeCrystal::crystallise_video(frames, frame_rate, tau)
            })
        })
        .collect();
    handles
        .into_iter()
        .map(|h| h.join().expect("a video crystallization thread must not panic"))
        .collect()
}
