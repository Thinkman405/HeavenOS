---
type: subsystem
subsystem: crystallisation
tier: presentation
language: Rust
stage: 04_implement
status: complete
result: "61 tests passing. All three pipelines built, incl. volumetric time crystals. Codecs (PPM/WAV) and video decode real files. TetryenRecurrence gives volumetric time-crystals the same discrete Tetryen time-evolution gui has, driven by the crystal's own real fundamental frequency. Rendering still open."
slices: ["linguistic + holographic + oscillators", "volumetric time crystals", "codecs and video", "Tetryen recurrence (discrete time evolution)"]
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
