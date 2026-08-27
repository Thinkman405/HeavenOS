---
type: subsystem
subsystem: crystallisation
tier: presentation
language: Rust
stage: 04_implement
status: complete
result: "67 tests passing. All three pipelines built, incl. volumetric time crystals. Codecs (PPM/WAV) and video decode real files. TetryenRecurrence gives volumetric time-crystals the same discrete Tetryen time-evolution gui has, driven by the crystal's own real fundamental frequency. FrequencyMap::transform now runs a real row-column-decomposed FFT (radix-2 where the axis length is a power of two, exact O(n^2) fallback otherwise) instead of a direct O((HW)^2) sum, with zero change to any caller's output. crystallisation::parallel batches images/audio/video across real OS threads, verified bit-for-bit identical to sequential execution. Rendering still open."
slices: ["linguistic + holographic + oscillators", "volumetric time crystals", "codecs and video", "Tetryen recurrence (discrete time evolution)", "parallel media crystallization"]
prd_sections: ["8"]
binds_axioms: ["A1", "A3"]
split_from: ftg
consumes: [lattice, substrate]
---

# Crystallisation — flat media into resonant shapes

One job: convert linear 1D and 2D data into native 3D/4D spatial-harmonic form — text into harmonic nodes, images through a Continuous Fourier Transform onto Tetryen faces, audio and video into volumetric time-crystals.

## Status: complete — all three pipelines, plus codecs and video

Split out of [[ftg]] by decision, then built in three slices.

**The time-crystal gap is closed.** PRD §8 offers two readings of media — "localized oscillators **or** volumetric time-crystals". Slice 1 built the first and recorded the second as undefined, since no paper in the corpus supplies it. Slice 2 closed it with [`_mkb/timecrystal.md`](../../_mkb/timecrystal.md), a **synthesis** of two things that are already law — the Howard Comma and Tetryen geometry — recorded as a synthesis rather than a distillation.

Two execution rules emerged from building it that the definition did not state: quantisation must be **joint** (independent rounding breaks the half-quantum bound by up to 36×), and there is a **quantisable ceiling** at `2⁵³` quanta, about `1.9e-17` J.

**Slice 3 closes the last two open gaps: codecs and video.** `neos/crystallisation/src/codec.rs` decodes real PPM images and PCM WAV audio — the two uncompressed formats needing no external dependency. Video is a second synthesis, `_mkb/timecrystal.md` §5: each frame reduces to one number via the holographic pipeline's own `PixelGrid::energy()`, and the resulting scalar sequence goes through the **unmodified** VTC procedure §§1–4 already define. No new physics; a frame sequence becomes exactly the scalar time series `crystallise` already expects.

That composition inherits `§2.4`'s quantisable ceiling honestly rather than working around it: a realistic frame's raw pixel energy is roughly fifty orders of magnitude past what `C_H` can exactly quantise, and `crystallise_video` refuses rather than inventing a rescaling the corpus does not supply.

## The Tetryen recurrence, driven by real data

[`_mkb/tetryen_recurrence.md`](../../_mkb/tetryen_recurrence.md) — the same synthesis [[gui]]'s `TetryenState` implements — now has a second implementation here: `timecrystal::TetryenRecurrence`. Same exact discrete-oscillator identity, same law-sourced coupling weight, but driven by `VolumetricTimeCrystal::fundamental()` — a real, Howard-Comma-quantised frequency this record already computes from actual media — rather than an arbitrary `omega` supplied by a caller. Because this crate has no Tetryen geometry of its own (that lives in [[gui]] and, since the relocation below, in [[lattice]]), `coupling_weight` is a caller-supplied `f64` rather than something derived internally; this is justified by [`tetryen.md`](../../_mkb/tetryen.md)'s own proven fact that a *regular* Tetryen gives every node pair the identical weight regardless of instance, so one caller-supplied number loses nothing a real Tetryen would have provided.

Stability was re-measured for this record's own real parameters rather than assumed from `gui`'s — different fundamental frequency means a different safe `(dt, gamma)` region. Found via a disposable scratch harness (`neos/crystallisation/examples/scratch_recur_check.rs`, deleted after use): `gamma=1e10` stays bounded (~69023) over 5,000 steps at this fixture's fundamental; `gamma=1e12` reliably diverges by step 118. `CrystalError::Diverged{amplitude}` refuses a non-finite step rather than propagating it, matching `GuiError::Diverged` and `KernelError::Diverged`.

## A real FFT, closing the item every earlier slice carried forward

`FrequencyMap::transform`/`inverse` used to be a direct 2D sum, `O((HW)^2)`, recorded from the first slice onward as "fine for the grid sizes here and not worth a dependency yet." Closed without a dependency — the same discipline already used for PPM/WAV (hand-written codecs, not a format crate), applied to the transform itself. Two separable facts compose: the 2D DFT factors *exactly* into a row pass then a column pass of 1D DFTs (`e^{-2\pi i(uy/H+vx/W)} = e^{-2\pi i uy/H} \cdot e^{-2\pi i vx/W}`, no approximation), which alone drops the cost to `O(HW(H{+}W))`; each 1D pass then runs a real radix-2 Cooley-Tukey FFT when that axis's length is a power of two, falling back to the exact `O(N^2)` sum otherwise — real inputs here are not all powers of two (a `3x3` test fixture, an arbitrary-length decoded audio/video signal), and correctness for those matters more than a general mixed-radix implementation's added risk would buy. Verified against the original direct sum in a disposable scratch harness before anything real changed; every existing Parseval/round-trip/DC-term test passed unmodified afterward, plus a new test cross-checking actual coefficients against a second, independently written direct DFT. No caller's output changed — confirmed directly by running the demo binary and comparing the video crystallisation section's reported energy before and after.

## Parallel media crystallization, closing on zero synchronisation

Checked before writing a line of code: every pipeline in this crate (holographic, resonant-chamber, and `takens_embed`) is a pure function over owned data — a grep across every source file in the crate found no `static`, no interior mutability, no `unsafe`, anywhere. That is a genuinely different shape of concurrency than `symphony_lang::concurrent` or `symphony_kernel::ConcurrentPool`/`ConcurrentTracker`, both of which exist specifically because their callers *share* mutable state and need real synchronisation to stay correct under contention. Nothing here shares anything, so `crystallisation::parallel::{crystallize_images, embed_audio, crystallize_videos}` need no `Mutex`, no `Condvar`, no resource tracker — one real `std::thread::spawn` per job, joined back in input order, is the entire mechanism.

The load-bearing claim, and where this record's own test effort actually went: parallel execution must return results **bit-for-bit identical** to sequential execution on the same inputs, since which thread happened to run a job is not allowed to change a floating-point answer. Verified directly for all three pipelines via `assert_eq!` against `PartialEq`-deriving result types (`FaceProjection`, `PhaseSpaceVector`, `VolumetricTimeCrystal`, `CrystalError`) — not approximately, exactly. A dedicated test also deliberately mixes job costs (one large non-power-of-two image first, three tiny ones after) so real threads cannot possibly all finish in input order, closing the one way a naive implementation could silently scramble results without any single-job test ever noticing.

The one genuinely new empirical finding: `FrequencyMap::transform`'s cost cliff between power-of-two and non-power-of-two sizes (documented above) is dramatic enough to use directly as a timing fixture — a `48x48` image (not a power of two, so it hits the exact `O(N^2)` fallback per axis) takes real, measurable CPU time per job, which is what makes a genuine-speedup test possible without either flakiness or an artificially inflated workload. Measured directly on the development machine (4 logical cores) across several runs before picking a bound: the parallel/sequential wall-time ratio for a 4-job batch of this size ranged `0.44`-`0.77`; the shipped test asserts `< 0.85`, generous enough to hold on a slower or more contended machine while still failing outright if this ever silently degenerated into a sequential loop wearing a `thread::spawn` name.

Sabotage: reversing the order handles are collected in (`handles.reverse()` before joining) failed both the plain bit-for-bit test and the mixed-cost ordering test, for the correct reason — output rows shifted relative to their real inputs. Reverted after confirming.

Wired into the demo binary: the report's image, audio, and video crystallisation now run on three real, concurrently spawned OS threads rather than one after another — output is unchanged (confirmed by running the binary before and after), only the execution shape is.

**A follow-up pass wired in the fourth pipeline the demo had never exercised: `linguistic::Crystal`.** It joins the other three as a fourth concurrent thread, crystallising a real two-line-break document. Running a genuine multi-break document through this path end-to-end for the first time — nothing before this had exercised `Crystal::crystallise` outside its own unit tests — surfaced a real, stale bug in `linguistic.rs`'s own module doc comment, not in the code: the worked table it shipped with claimed "about four" line breaks as the ceiling, with `break 2 -> 4.82843`, while the crate's own tests (`bifurcation_is_geometric_not_doubling`, `over_deep_document_is_refused`) already assert the real, exact values — a ceiling of **3**, and `2 (x) 2 = 20.970562748477143` — and the live demo run agrees with the tests, not the comment. Fixed the comment to state the tested values directly, computed via a disposable scratch check rather than re-derived by hand.

## A sibling of `ftg`, not a child

The PRD frames §8 as Layer 7 of the FTG, which invites the assumption that this record depends on transport. **It does not.**

Crystallisation is a *representation transform*. It converts media into the native NEOS form; whether that form then travels a network is [[ftg]]'s concern. Declaring a dependency on `ftg` would block this record longer than the architecture requires.

**Depends on:** [[lattice]] (⊗ arithmetic, Tetryen geometry, 4D rotation) and [[substrate]] (wave translation floor). Both are **built**, so this record is buildable the moment it is picked up.

## Scope

**Owns:** `neos/crystallisation/**`
**PRD sections:** §8 (Application Data Translation)
**Axioms that bind it:** A1 (line breaks and code structure trigger bifurcation events), A3 (projection into hyperbolic space)
**Law:** [`_mkb/tetryen.md`](../../_mkb/tetryen.md) for the projection surface; [`_mkb/operators.md`](../../_mkb/operators.md) for ⊗

## The three pipelines

Distinct enough that this record may itself split, the way `symphony` did. Do not assume one design covers all three.

| Pipeline | PRD §8 | Transform |
|---|---|---|
| **Linguistic** | text / code | character strings → sequential harmonic nodes; line breaks trigger bifurcation → navigable 3D polymer-like fractals |
| **Holographic** | images | pixel grids → Continuous Fourier Transform → spatial frequency maps → internal faces of scalable Tetryen geometry |
| **Resonant chambers** | audio / video | media as localised oscillators or volumetric time-crystals; **audio and video both decode from real files** (`codec.rs`) and both drive the same VTC procedure, video via a per-frame energy reduction |

## Overlap to watch

Holographic projection targets **Tetryen faces**, which [[gui]] also renders. That is fine while the definition stays in [`_mkb/tetryen.md`](../../_mkb/tetryen.md) — one home per fact — but the two records must not each grow their own Tetryen geometry. If a shared primitive is needed, it belongs in `lattice`, not in either consumer.

## Prerequisite for resuming

[[ftg]] complete, per the deferral decision. Nothing in the law blocks it.

## Do not

Load other subsystems' records. They don't share state; they share the factory (`_mkb/`, `_spec/`).
