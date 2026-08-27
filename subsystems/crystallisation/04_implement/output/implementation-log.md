---
type: implementation-log
subsystem: crystallisation
stage: 04_implement
status: complete
toolchain: rustc 1.97.1 / cargo 1.97.1
result: 62 passed, 0 failed across crystallisation + crystallisation_codec + crystallisation_timecrystal (454 workspace-wide) — see the real-FFT addendum for the most recent change
consumes: [lattice, substrate]
---

# Crystallisation — Implementation Log

## Result

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 239 passed; 0 failed
```

Two of PRD §8's three pipelines are fully built. The third is **half** built, deliberately.

## Files written

| Path | Role |
|---|---|
| `neos/crystallisation/Cargo.toml` | manifest — `lattice` + `substrate` only |
| `src/lib.rs` | crate root, `CrystalError` |
| `src/linguistic.rs` | text → harmonic nodes, line breaks bifurcate |
| `src/holographic.rs` | DFT, frequency map, Tetryen face projection |
| `src/resonant.rs` | media → oscillators (half the pipeline) |
| `neos/tests/crystallisation.rs` | 18 assertions |

## Half a pipeline, on purpose

PRD §8 says media act as *"localized oscillators **or** volumetric time-crystals."*

The oscillator reading has law and is built. **The time-crystal reading has none.** Unlike the Tetryen — which was undefined until `Mathematical_Fra.pdf` supplied `E[Γ]` — no paper in the corpus defines a time-crystal. There is nothing to distil.

So there is **no time-crystal type, not even a stub**. A stub would imply a semantics nobody has specified, and `CLAUDE.md` forbids speculative math. "Driving physical vibrations" and "4D spatial rotation" rest on the same undefined half and are likewise absent.

This is the second §8 sentence that could not be fully implemented from the corpus — recorded rather than filled in.

## A contract number I got wrong

`01_derive` quoted a bifurcation ceiling of **4** and an extent of `4.828` after two line breaks. Both were wrong, and the tests caught it.

My probe iterated `e ⊗ 1` — a **unit step**. A1 defines a bifurcation as `u ⊗ u` — **self**-⊗, which squares the product each time:

```
unit step:  2.0 → 4.82843 → 40.0726 → 1.09e15 → refused   (4)
self-⊗:     2.0 → 20.9706 → 1.07e168 → refused             (3)
```

The implementation was right; the contract described a different operation than the design. Corrected in the contract, the test plan, and two assertions.

**The ⊗ ceiling has now appeared in three subsystems** — `lattice` addressing, `symphony-kernel` (as the domain guard), and here. The constraint is systemic to iterating ⊗ against a fixed domain; the *exact* depth depends on how fast the operands grow, which is why unit steps reach 4 and self-⊗ reaches 3. Worth not conflating them.

## Doctrine checks — two performed

| Sabotage | Tests failed |
|---|---|
| truncate over-deep documents instead of refusing | 1 |
| drop the imaginary part, making the transform lossy | 2 — Parseval **and** round-trip |

The second is the more informative: `dc_term_is_the_pixel_sum` and `uniform_image_has_only_a_dc_term` both **survived** a lossy transform, because a flat image and the DC term are real-valued. Only Parseval and the round trip notice. That is why both are asserted — either alone would leave the gap.

## A sibling of `ftg`, not a child

Dependencies are `lattice` and `substrate` only. The PRD frames §8 as the gateway's Layer 7, which invites a transport dependency, but crystallisation is a **representation transform** — whether the result then travels a network is `ftg`'s concern.

`ResonantChamber` deliberately has **no `energy()`**. Pricing it would need `E = C_H·ν` from `symphony-kernel`, and adding a kernel dependency to render media would invert the layering for one multiplication. A caller that wants energy already has `symphony_kernel::energy`.

## What is not built

- **Volumetric time-crystals**, media 4D rotation, "driving physical vibrations" — undefined, per above.
- **Codecs.** `PixelGrid` takes floats; nothing parses PNG or WAV. Not §8's subject.
- **Rendering.** `FaceProjection` is data; `gui` owns drawing. Nothing yet hands one to the renderer.
- **Spectral frequency estimation.** `from_samples` uses zero-crossing rate, adequate for a single tone and honest about not being a spectral estimate — a polyphonic stream has no one dominant frequency.
- ~~A real FFT.~~ Built in a later pass without a dependency — see the addendum at the end of this log.

## Human check

Read `over_deep_document_is_refused` and `parseval_holds`. The first is the ⊗ ceiling surfacing in a third place, at a different arity. The second is the property separating a representation from a summary — and the sabotage showed two other transform tests cannot substitute for it.

---

# Slice 2 — Volumetric Time Crystals

Added `_mkb/timecrystal.md`, `src/timecrystal.rs`, a `build.rs` for `C_H`, and `neos/tests/crystallisation_timecrystal.rs` (16 assertions). Workspace total: **255 passing**.

**Closes the open question from slice 1.** The time-crystal reading of PRD §8 now has an operational definition: Takens delay embedding into 4D phase space, Floquet quasi-energies quantised by the Howard Comma, and `SO(3,1)` modulation preserving phase-space volume.

The definition is recorded in `_mkb/` as a **synthesis** of two things that are already law — `C_H` and Tetryen geometry — not as a distillation, because no paper supplies it.

## Two rules the definition did not state, found by building it

### Joint quantisation is mandatory

The invariant `|E − Σ n_k C_H ν_k| ≤ ½ C_H ν₀` does not say how the `n_k` are chosen, and **the obvious choice violates it**. Independent per-mode rounding accumulates error while the bound is a *single* half-quantum:

```
independent rounding: residual 1.04e-31  vs floor 1.32e-32   (8x over, worst case 36x)
joint (fundamental absorbs): residual 2.11e-33               (inside)
```

Harmonics quantise freely; the fundamental takes the remainder. `independent_rounding_residual()` exists so the two can be *compared* in a test rather than the joint scheme being asserted better.

### There is a quantisable ceiling

`C_H ≈ 2.64e-34` J·s. A unit-amplitude tone needs **`2.5e35` quanta** — past `i64` entirely (`9.2e18`) and past `f64`'s exact-integer limit `2⁵³ ≈ 9.0e15`.

This surfaced as three test failures from integer overflow. The honest fix was not a wider type but a **refusal**: signals needing more than `2⁵³` quanta are rejected, exactly as ⊗ rejects an out-of-domain product. Maximum quantisable energy is about `1.9e-17` J at `ν₀ = 7.8` Hz, and the test suite works in that regime.

`Mode::occupation` is `f64` rather than an integer type, with the reason documented — no integer type spans the range, and refusal keeps integrality exact within the accepted band.

## The absolute-vs-relative trap, fourth occurrence

`is_floquet_periodic` took an **absolute** tolerance. After scaling the test signal to amplitude `2e-15`, a tolerance of `1e-9` was six orders *larger than the signal* — every period matched, and the negative assertion passed on nothing.

Now relative to the trajectory's own amplitude, with `floquet_check_is_scale_free` pinning it.

That is the fourth time this pattern has appeared — `symphony-kernel` convergence, `ftg` cancellation, `gui` scale-free, and now here. Twice it was caught by sabotage, twice by a test failing after a scale change. **A threshold has to be expressed in the units of the thing it bounds**, and that is now stated in the method's own docs.

## A test that was pretending

`non_unitary_modulation_is_refused` tried to construct a non-unitary transform and could not — every public constructor (`boost`, `rotation`, `compose`) lands in `SO(3,1)` by construction. The test contained dead scaffolding that asserted nothing about the guard.

Replaced with `every_constructible_transform_is_unitary`, which states the **stronger** true position: a non-unitary modulation is *unrepresentable* through the public API, so the runtime guard cannot be triggered from outside. The guard stays — it costs nothing and would catch a future constructor that broke the invariant — but the test no longer pretends to exercise it.

## Doctrine check

| Sabotage | Tests failed |
|---|---|
| round the fundamental independently instead of absorbing the residual | **3 of 16** |

## Reuse rather than a second transform

The mode spectrum comes from the **same DFT** the holographic pipeline uses — the signal is transformed as a `1 × N` grid. One home for the transform rather than an audio copy.

`C_H` is read from `constants.json` via this crate's own `build.rs`, not borrowed from `symphony-kernel`. The JSON is the one home; each crate reads it directly, which is the established pattern and avoids a kernel dependency for a single constant. The build script also asserts `frequency_variable == "nu"`, so a change to the angular convention would fail the build rather than silently shift every quasi-energy by `2π`.

## Still not built

- **Rendering** — unchanged from slice 1.
- ~~A real FFT~~ — built in a later pass. See the addendum at the end of this log.

**Wiring into `gui` is now built**, as a `gui`-side join rather than a change here: `gui::TetryenVisualisation::from_phase_vector` takes one `PhaseSpaceVector` and drives one standing wave per Tetryen node from its four components, reusing this file's own stated correspondence in §1 (*"the four components map to the four vertices of a fundamental Tetryen cell"*). `FaceProjection` is wired the same way via `from_face_projections`. See `gui`'s `CONTEXT.md` and root `CONTEXT.md`'s cross-cutting slices.

**Codecs and video are now built** — see Slice 3 below.

## Human check

Read `independent_rounding_would_break_the_bound` and `macroscopic_signal_exceeds_quantisation`. The first shows why the quantisation had to be joint, by measuring what the naive scheme would have left. The second is the Howard Comma's scale meeting IEEE-754 — a real ceiling, refused rather than hidden.

---

# Slice 3 — Codecs and Video

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 356 passed; 0 failed
cargo test  -p crystallisation --test crystallisation_codec → 20 passed
```

Workspace total: **336 → 356**. Closes the last two gaps `04_implement` had recorded: nothing decoded a real file, and the volumetric time-crystal reading covered only a scalar time series, never a frame sequence.

## Two formats decoded, chosen for what they don't need

PNG needs DEFLATE; most audio containers need a real codec. Both are out of scope for a from-scratch kernel with zero runtime dependencies — pulling one in for a single format would be the same complexity `CLAUDE.md`'s closed-form-over-runtime-discovery rule warns against, just moved to the data layer. So `neos/crystallisation/src/codec.rs` decodes the two standard, uncompressed formats that need only header arithmetic and a byte copy: **PPM** (`P5`/`P6`, netpbm) for images, **PCM WAV** (RIFF, 8- or 16-bit) for audio. Both are real, external, decades-old formats — not invented here. What *is* a stated convention, not a distillation, is RGB→grayscale (ITU-R BT.601 luma) and multi-channel→mono (channel average) — recorded as choices in the doc comments, the same discipline `telemetry`'s traffic mapping and `gui`'s phase-vector `.abs()` already followed.

Every byte-level decision (chunk walking with the RIFF odd-length pad byte, big-endian 2-byte PPM samples, header comment lines) was verified against hand-built synthetic files in a scratch harness before being written into the crate — not asserted, evaluated.

## Video: a synthesis, not a new pipeline

`_mkb/timecrystal.md` §5 closes PRD §8's video reading by composing two things that are already law: each frame reduces to one number via the holographic pipeline's own `PixelGrid::energy()` (already Parseval-verified), and the resulting scalar-per-frame sequence is handed to `VolumetricTimeCrystal::crystallise` — **unmodified**. No new Takens embedding, no new Floquet formula. `crystallise_video_is_exactly_crystallise_on_frame_energies` verifies this literally: build the reduced signal by hand, call `crystallise` directly, and confirm the result is field-for-field identical to `crystallise_video`'s.

Takes `impl IntoIterator<Item = PixelGrid>` rather than a slice, so a caller streaming frames from disk never holds more than one decoded frame at a time — only the reduced `f64` needs to outlive each frame.

## A defect I found while wiring it, in my own reasoning this time

The first version of the "quantisable video" test used the same `2e-15` per-pixel amplitude the existing audio tests use, and every mode — including the fundamental — quantised to exactly zero occupation. Measured, not guessed: printed occupation counts across six amplitudes from `2e-15` to `2e-6`. `crystallise`'s `input_energy` is the sum of *squares* of the outer signal, and here the outer signal is itself already a sum of squares (`PixelGrid::energy` per frame) — squaring an already-audio-scaled amplitude a second time undershoots the quantum by another ~30 orders of magnitude. `2e-8` per pixel was the smallest scale giving every mode a nonzero count, comfortably clear of the ceiling on the other side.

**This is not the same trap as the four earlier absolute-vs-relative occurrences** — it's a fifth, related one: getting the *absolute scale* wrong relative to `C_H`, in the too-small direction this time rather than too-large. `_mkb/timecrystal.md` already names the too-large half of this (§5.3, the refusal a realistic frame triggers); the too-small half surfaced only by actually running numbers, which is why it's recorded here rather than reasoned about from the shape of the formula.

## The ceiling applies to video exactly as it already applies to audio

`realistic_pixel_scale_video_is_refused_not_rescaled` feeds ordinary `0..255`-range pixels through `crystallise_video` and confirms the refusal: `required` comes back roughly `10^10`× past `max`. `crystallise_video` does not rescale — there is no rescaling formula in the corpus to invoke, and inventing one would be exactly the speculative math `CLAUDE.md` forbids. A caller wanting quantised video supplies frames already scaled the way the audio tests scale their signal.

## Doctrine checks — four performed

| Sabotage | Failures |
|---|---|
| RGB→grayscale by plain average instead of BT.601 luma | **1 of 20** |
| WAV downmix keeps only channel 0 | **1 of 20** |
| `crystallise_video` skips the frame-size-mismatch check | **1 of 20** |
| per-frame signal replaced with the frame index, not its energy | **3 of 20** |

All four bite, none collateral.

## Human check

Read `crystallise_video_is_exactly_crystallise_on_frame_energies` and the amplitude-scale finding above. The first is the literal verification of `_mkb/timecrystal.md` §5.1's central claim — that video crystallisation is composition, not a new algorithm. The second is a mistake I made and caught by measuring rather than by reasoning about the formula's shape, the same discipline `_mkb/gates.md` and the `ξ` overflow fix already established this session.

---

# Addendum — the Tetryen recurrence (`timecrystal::TetryenRecurrence`)

A second implementation of [`_mkb/tetryen_recurrence.md`](../../../../_mkb/tetryen_recurrence.md), the same synthesis `gui::evolution::TetryenState` already implements — closing the undistilled corpus's `ψ_{n+1}=f(ψ_n,ψ_{n-1})` placeholder a second time, from a different real data source, rather than duplicating physics that already exists.

## Why a second implementation, not a shared caller

`gui::TetryenState` takes a caller-supplied `omega` and couples across a real `Tetryen`'s own geodesic node distances — geometry this crate does not have. `crystallisation::TetryenRecurrence` instead drives `omega` from `VolumetricTimeCrystal::fundamental()`, a real, Howard-Comma-quantised frequency this crate already computes from actual media, so no arbitrary frequency is needed here the way `gui`'s demo has to supply one. Because there is no Tetryen geometry in this crate to derive a coupling weight from, `step` takes `coupling_weight: f64` from the caller — justified, not just convenient: `_mkb/tetryen_recurrence.md` §2 proves every pairwise weight on a *regular* Tetryen (the only kind this workspace constructs) is identical, so one caller-supplied number carries exactly what a real Tetryen would have supplied, nothing lost.

## The weight itself moved, not duplicated

The coupling weight both implementations use is `lattice::tetryen_node_envelope(r)`, `_mkb/tetryen.md`'s node standing-wave form. It used to live only inside `gui::renderer::Tetryen::node_amplitude`; since `crystallisation` cannot depend on `gui` (dependency direction runs `crystallisation → {lattice, substrate} → gui`, not through it), the fact relocated to `lattice` — the one crate both already depend on — rather than being reimplemented here or a dependency cycle being introduced. Full account in `lattice`'s own implementation-log addendum.

## Stability re-measured for this crate's own real parameters, not assumed from `gui`'s

`gui`'s measured safe region (`γ` up to `1e4` at `Δt=0.01`, `ω=3.0`) does not transfer here unchecked — this crate's real fundamental (`≈15.625 Hz` for the test fixture's tone, `≈1.5 Hz` for the demo binary's video) gives a different `ωΔt`, and the discrete-oscillator identity's own stability depends on that product. Swept in a disposable scratch harness (`neos/crystallisation/examples/scratch_recur_check.rs`, deleted after use) before writing the divergence test:

| `γ` | Behaviour at this fixture's fundamental |
|---|---|
| `1e10` | bounded — grows to `~69023` but stays finite over 5,000 steps |
| `1e11` | diverges |
| `1e12` | diverges reliably, by step 118 |

An initial guess of `γ=1e10` for the divergence test was wrong — checked before shipping, not after: the scratch harness showed it does *not* diverge. `γ=1e12` is the value actually used in the test, verified rather than assumed.

`CrystalError::Diverged{amplitude}` refuses a non-finite step rather than propagating it, matching `GuiError::Diverged` and `symphony_kernel::KernelError::Diverged` — the third crate now carrying this exact naming pattern.

## Doctrine checks — two performed

| Sabotage | Result |
|---|---|
| Coupling sign flipped (anti-diffusive) | **1 of 21 failed** — `coupling_pulls_differing_components_toward_each_other` |
| The `is_finite()` divergence guard removed | **1 of 21 failed** — `a_step_leaving_the_stability_region_is_refused_not_propagated`, and confirmed to terminate safely (not hang) even with the guard gone |

Both reverted after confirming; full workspace re-run clean at 423/423.

## Wired into the demo

`neos/src/main.rs` now runs this recurrence too, alongside `gui`'s: seeded from the real video time-crystal's own first embedded phase-space node, coupling weight computed via `lattice::tetryen_node_envelope` at the real geodesic separation between two of the demo's own Tetryen nodes, driven by the video crystal's real `fundamental()` (`1.5 Hz`). Confirmed by running the binary directly, not just by the assertion passing silently: `gamma=1.0, dt=1e-5` stays bounded over 200 steps at this specific VTC's real fundamental (the demo's own parameters, separate from and re-checked apart from the test fixture's `γ=1e12` divergence case above).

## Human check

Read `identical_components_evolve_by_the_uncoupled_identity_regardless_of_coupling` first, the same isolation strategy `gui`'s equivalent test uses. Then compare this file's `step` against `gui::evolution::TetryenState::step` side by side — the uncoupled term and the coupling weight should be identical in substance, differing only in where `omega` and the coupling weight come from.

---

# Addendum — a real FFT, closing the one item every earlier slice carried forward

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 454 passed; 0 failed
cargo test  -p crystallisation → 62 passed (21 + 20 + 21)
```

Workspace total: **453 → 454**. `crystallisation` 20 → 21 (one new cross-check test; the rest of the increase is the workspace's own concurrent tests landing in the same window).

Every prior slice's "still not built" list named the same thing: `FrequencyMap::transform`'s direct 2D sum is `O((HW)^2)`, "fine for the grid sizes here and not worth a dependency yet." Closed without a dependency — the discipline this crate already follows for PPM/WAV (hand-written codecs over pulling in a format crate) applied to the transform itself.

## The free win, before any fast algorithm

The 2D DFT separates into two passes of 1D DFTs — `e^{-2*pi*i*(uy/H + vx/W)} = e^{-2*pi*i*uy/H} * e^{-2*pi*i*vx/W}`, an exact algebraic identity, not an approximation or a special case. Transforming every row (length `W`) and then every column (length `H`) computes the *identical* coefficients as the direct 2D sum, at `O(HW*(H+W))` instead of `O((HW)^2)` — a real complexity improvement that costs nothing in accuracy and needs no fast algorithm to claim. This alone was verified against the existing direct sum in a disposable scratch harness (`neos/crystallisation/examples/scratch_fft_check.rs`, deleted after use) before a single line of the real module changed, across power-of-two shapes, non-power-of-two shapes (`3x3`, `5x7`), and the exact `1xN` shape production code uses for every audio/video signal — agreement to ~1e-12 in every case.

## The real FFT, on top

Each 1D pass then dispatches to a radix-2 Cooley-Tukey FFT (`O(N log N)`) when that axis's length is a power of two, and to the exact `O(N^2)` 1D sum otherwise. This split is not a compromise forced by running out of time for the general case — it is the honest shape of what this crate's actual inputs are: a `3x3` grid is one of this crate's own uneven-projection test fixtures, and a decoded audio/video signal's sample count has no reason to be a power of two. A general mixed-radix or Bluestein FFT would handle those too, at real additional numerical-code complexity and real additional risk of a subtle correctness bug in a from-scratch implementation; falling back to the already-exact direct sum for anything not a clean power of two costs nothing in correctness and a great deal less in implementation risk, for cases that are already small enough that `O(N^2)` is not the bottleneck.

## Verified before it replaced anything, and again after

Same scratch harness as above confirmed `fft_radix2`'s output matches the direct sum at power-of-two sizes specifically (`4x4`, `8x8`, `2x8`, `16x4`) to ~1e-12, before `FrequencyMap::transform`/`inverse` were touched. After the real module changed: every existing test in `neos/tests/crystallisation.rs` passed unmodified, including `parseval_holds`, `transform_round_trips`, and `dc_term_is_the_pixel_sum` — all of which already exercise the FFT path incidentally, since the shared `grid()` fixture is `4x4`. A new test, `fft_matches_an_independent_direct_dft_at_power_of_two_sizes`, checks the actual coefficients (not just Parseval/round-trip, which an exact-but-differently-ordered computation would also satisfy) against a second, independently written direct-sum implementation — the same cross-validation discipline `lattice::pathfinding` vs `ftg`'s own `bfs_hops` already established for this workspace.

## Doctrine checks — two performed

| Sabotage | Result |
|---|---|
| Inverse sign not flipped (`inverse { 1.0 } else { -1.0 }` → `inverse { -1.0 } else { -1.0 }`) | **1 failed** — `transform_round_trips`: the inverse stopped inverting |
| Bit-reversal permutation skipped entirely | **2 failed** — `fft_matches_an_independent_direct_dft_at_power_of_two_sizes` (wrong coefficients directly) and `transform_round_trips` (round trip no longer recovers the original) |

Both reverted after confirming; full suite re-confirmed at 21/21 in this file, 454/454 workspace-wide.

## What stays exactly as documented

No behaviour changed for any caller: `crystallise_video`'s per-frame energy reduction, the quantisation ceiling, the Parseval invariant, the DC-term identity — all unchanged, because the transform's actual *output* is unchanged, only how it's computed. Running the demo binary confirms this directly: the video crystallisation section reports the identical energy (`9.8302e-25 J`) and `conserving: true` before and after this change.

## Human check

Read `fft_matches_an_independent_direct_dft_at_power_of_two_sizes` alongside `parseval_holds`/`transform_round_trips` — the new test checks the actual numbers a second implementation independently computes; the older tests check properties an exact-but-differently-computed result would also satisfy. Both kinds of test are doing real work here, and neither substitutes for the other.

---

# Addendum — parallel media crystallization (`crystallisation::parallel`)

```
cargo build --workspace  → Finished, no warnings
cargo test  --workspace  → 472 passed; 0 failed
cargo test  -p crystallisation --test crystallisation_parallel → 5 passed
```

Workspace total: **467 → 472**. This crate's own test count: 62 → 67.

## Checked before writing anything: is this actually safe with zero synchronisation?

A grep across every file in `neos/crystallisation/src/` for `static `, `thread_local`, `OnceCell`, `lazy_static`, `RefCell`, `Cell<`, and `unsafe` returned nothing. Every pipeline — `holographic`, `timecrystal`'s `takens_embed`/`crystallise`/`crystallise_video`, `linguistic` — is a pure function or method over data the caller already owns or borrows. That is the actual justification for building `crystallisation::parallel` with no `Mutex`, no `Condvar`, no resource tracker at all: `symphony_lang::concurrent` and `symphony_kernel::ConcurrentPool`/`ConcurrentTracker` needed real synchronisation because their callers genuinely share mutable state across threads; nothing here does, so a real `std::thread::spawn` per job with no lock anywhere is not a shortcut, it is the correct amount of mechanism for what this crate actually has.

## Three functions, one shape, no shared abstraction

`crystallize_images(Vec<PixelGrid>) -> Vec<Result<[FaceProjection; 4], CrystalError>>`, `embed_audio(Vec<(Vec<f64>, usize)>) -> Vec<Result<Vec<PhaseSpaceVector>, CrystalError>>`, and `crystallize_videos(Vec<(Vec<PixelGrid>, f64, usize)>) -> Vec<Result<VolumetricTimeCrystal, CrystalError>>` are three near-identical functions — spawn one thread per job, join every handle in input order — rather than one generic batch function over a trait object or closure. Deliberate: the three pipelines have unrelated input/output types, and per this workspace's own stated discipline against premature abstraction, three similar ten-line functions are clearer than a generic layer built to serve exactly three call sites.

## The real claim, and how it was tested

Real concurrency here cannot change output — that is the whole point of "pure function, no shared state." So the tests split into two genuinely different concerns: *correctness* (parallel must equal sequential, exactly) and *concurrency* (it must also actually be faster, not just correct by accident because it silently runs one thread at a time).

For correctness, every pipeline's parallel-vs-sequential comparison uses `assert_eq!` against a `PartialEq`-deriving result type — `FaceProjection`, `PhaseSpaceVector`, `VolumetricTimeCrystal`, and `CrystalError` all already derive it, so no new derive was needed anywhere in `holographic.rs`/`timecrystal.rs`. `parallel_image_crystallization_preserves_input_order_even_when_jobs_finish_out_of_order` is the sharper of the four correctness tests: it deliberately gives job 0 far more work than jobs 1-3 (a `90x90` non-power-of-two image against three `8x8` ones), so real threads cannot possibly all finish in input order — a naive implementation that collected results by *arrival* instead of by *input position* would pass every same-cost test in this file and fail only this one.

For concurrency, `parallel_image_crystallization_is_genuinely_faster_than_sequential` needed a real, substantial per-job cost to measure against, found by profiling rather than guessed: `FrequencyMap::transform` at a non-power-of-two size lands on the exact `O(N^2)` fallback per axis (documented in the FFT addendum above), which turns out to be dramatic — `96x96` measured **321ms** against `128x128`'s **14.5ms**, despite being smaller, purely because 128 is a power of two and 96 is not. `48x48` was chosen as a size cheap enough to keep the test suite fast but costly enough (~150-250ms per job under this workspace's `opt-level = 1` test profile) to make thread-spawn overhead negligible by comparison. The bound itself (`parallel < 0.85 * sequential`) was set from six actual timing runs on the development machine (4 logical cores), which ranged `0.44`-`0.77` — generous margin in both directions, not a number chosen to make one lucky run pass.

## Sabotage — one performed, sharply targeted

| Sabotage | Result |
|---|---|
| `crystallize_images` reverses its `Vec<JoinHandle>` before collecting results | **2 of 5 failed** — the plain bit-for-bit test and the mixed-cost ordering test, both for the same real reason: output rows no longer lined up with their input position |

Reverted after confirming; full suite re-confirmed at 5/5 in this file, 472/472 workspace-wide. Only one sabotage was needed here, unlike most of this workspace's other concurrency work, precisely because there is no shared-state boundary to attack — the entire correctness surface is "did the results come back matched to the right input," which the ordering test already isolates sharply.

## Wired into the demo

`neos/src/main.rs`'s crystallisation section — previously three sequential calls (decode+transform image, decode+embed audio, decode+crystallise video) — now spawns one real thread per pipeline directly (not through the batch functions above, since the demo's three media types are genuinely different, not a batch of one type) and joins all three before printing. Output is byte-for-byte unchanged, confirmed by running the binary before and after; only the execution shape changed, from sequential to three real concurrent threads.

## Human check

Read `parallel_image_crystallization_preserves_input_order_even_when_jobs_finish_out_of_order` alongside `parallel_image_crystallization_matches_sequential_bit_for_bit` — the second alone would pass even if results were collected by arrival order rather than input order, since same-size jobs give real threads no reason to finish in any particular sequence. The first is the one that actually forces the distinction.
