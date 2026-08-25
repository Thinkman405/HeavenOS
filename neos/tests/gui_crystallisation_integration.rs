//! Full-chain integration: real file bytes → `crystallisation` → `gui`.
//!
//! Every existing test on either side of this boundary stops short of it.
//! `crystallisation_codec.rs` decodes real PPM/WAV bytes but only checks the
//! decoded numbers. `gui.rs`'s `TetryenVisualisation` tests build a
//! `PixelGrid`/`PhaseSpaceVector` by hand and never touch `decode_ppm`,
//! `decode_wav`, or `crystallise_video`. Nothing had run the actual path a
//! real caller takes: bytes on disk → decoded → crystallised → rendered.
//! These assertions are that path, with no hand-built intermediate value
//! anywhere in the middle.

use crystallisation::timecrystal::{takens_embed, VolumetricTimeCrystal};
use crystallisation::{decode_ppm, decode_wav, FrequencyMap, PixelGrid};
use gui::TetryenVisualisation;

fn ppm_p5(width: usize, height: usize, maxval: u16, pixels: &[u8]) -> Vec<u8> {
    let mut b = format!("P5\n{width} {height}\n{maxval}\n").into_bytes();
    b.extend_from_slice(pixels);
    b
}

fn wav_pcm(channels: u16, rate: u32, bits: u16, samples: &[i32]) -> Vec<u8> {
    let bytes_per_sample = usize::from(bits / 8);
    let data_len = samples.len() * bytes_per_sample;
    let mut w = Vec::new();
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36u32 + data_len as u32).to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    let block_align = channels * bytes_per_sample as u16;
    w.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
    w.extend_from_slice(&block_align.to_le_bytes());
    w.extend_from_slice(&bits.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &s in samples {
        w.extend_from_slice(&(s as i16).to_le_bytes());
    }
    w
}

// ------------------------------------------------- Group 1: image -> render

/// 1.1 [D] — a real decoded PPM drives `TetryenVisualisation` end to end, and
/// the render reflects the file's actual content: the busiest face reaches
/// full amplitude, the others scale under it — the same property
/// `crystallisation_codec.rs` and `gui.rs` each verify on their own side of
/// the boundary, now verified across it with no hand-built `PixelGrid` or
/// `FaceProjection` anywhere in between.
#[test]
fn decoded_image_drives_the_render_end_to_end() {
    let file = ppm_p5(
        4,
        4,
        255,
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2, 3, 4, 5, 6, 7],
    );

    let grid = decode_ppm(&file).expect("real bytes must decode");
    let faces = FrequencyMap::transform(&grid)
        .project_onto_faces()
        .expect("16 coefficients divide evenly across 4 faces");
    let vis = TetryenVisualisation::from_face_projections(&faces, 1.2, 3.0);

    let energies: Vec<f64> = faces.iter().map(|f| f.energy()).collect();
    let (busiest, _) = energies
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();

    assert!(
        (vis.wave(busiest).unwrap().amplitude() - 1.0).abs() < 1e-12,
        "the face with the most real spectral energy must reach full amplitude"
    );
    assert!(vis.peak() > 0.0, "a real, non-uniform image must be drawable");
}

/// 1.2 [D] — a uniform PPM (every pixel identical) decodes and renders as
/// visually flat: face 0 (which carries the DC term) reaches full amplitude,
/// and the other three carry only floating-point DFT rounding residue —
/// **fourteen to fifteen orders of magnitude smaller**, not a meaningful
/// signal. A relative check, not `== 0.0`: the literal `O(n^2)` DFT sums many
/// terms that cancel analytically but not to the last bit in `f64`, so the
/// non-DC faces measure `~1e-27`–`1e-28` against face 0's `~1.25e4` rather
/// than exactly zero. Asserting exact equality here would be asserting
/// something the transform's own arithmetic cannot deliver.
#[test]
fn a_flat_decoded_image_renders_without_favouring_a_face() {
    let file = ppm_p5(4, 4, 255, &[7u8; 16]);
    let grid = decode_ppm(&file).unwrap();
    let faces = FrequencyMap::transform(&grid).project_onto_faces().unwrap();
    let vis = TetryenVisualisation::from_face_projections(&faces, 1.2, 3.0);

    assert!((vis.wave(0).unwrap().amplitude() - 1.0).abs() < 1e-12);

    let dc_energy = faces[0].energy();
    for f in 1..4 {
        let ratio = faces[f].energy() / dc_energy;
        assert!(
            ratio < 1e-14,
            "face {f} carries {ratio:e} of the DC energy — too large to be rounding noise"
        );
    }
}

/// 1.3 [D] — a malformed PPM is refused before it ever reaches the renderer;
/// the failure comes back as a real `CrystalError`, not a panic partway
/// through the chain.
#[test]
fn a_malformed_image_never_reaches_the_renderer() {
    let truncated = ppm_p5(8, 8, 255, &[1, 2, 3]); // needs 64 bytes, has 3
    assert!(decode_ppm(&truncated).is_err());
}

// ------------------------------------------------- Group 2: audio -> render

/// 2.1 [D] — real decoded audio embeds into real phase-space vectors, and
/// **one of them drives the render.** No hand-built `PhaseSpaceVector`
/// anywhere in this test — `takens_embed`'s own components are what
/// `TetryenVisualisation` reads.
///
/// `takens_embed` carries no amplitude ceiling (unlike
/// `VolumetricTimeCrystal::crystallise` — see Group 3), so real WAV-scale
/// samples work directly with no rescaling.
#[test]
fn decoded_audio_embeds_and_drives_the_render_end_to_end() {
    let samples: Vec<i32> = (0..60)
        .map(|i| ((i as f64 * 0.4).sin() * 10000.0) as i32)
        .collect();
    let file = wav_pcm(1, 8000, 16, &samples);

    let audio = decode_wav(&file).expect("real bytes must decode");
    let nodes = takens_embed(audio.samples(), 3).expect("60 samples embed at tau=3");
    assert!(!nodes.is_empty());

    let vis = TetryenVisualisation::from_phase_vector(&nodes[10], 1.2, 3.0);
    assert!(vis.peak() > 0.0, "a real embedded audio sample must be drawable");

    let components = *nodes[10].components();
    let (busiest, _) = components
        .iter()
        .map(|c| c.abs())
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    assert!(
        (vis.wave(busiest).unwrap().amplitude() - 1.0).abs() < 1e-12,
        "the largest-magnitude real sample must reach full amplitude"
    );
}

/// 2.2 — silence decodes and renders as silence: a WAV file of all-zero
/// samples must embed to all-zero phase-space vectors and draw nothing.
#[test]
fn decoded_silence_renders_nothing() {
    let file = wav_pcm(1, 8000, 16, &[0; 60]);
    let audio = decode_wav(&file).unwrap();
    let nodes = takens_embed(audio.samples(), 3).unwrap();

    let vis = TetryenVisualisation::from_phase_vector(&nodes[5], 1.2, 3.0);
    assert_eq!(vis.peak(), 0.0);
}

// ------------------------------------------------- Group 3: video -> render

/// 3.1 [D] — a video decoded from real PPM frames, at a caller-chosen
/// quantisable scale, crystallises and drives the render end to end: decode
/// → `crystallise_video` → a real `Mode`'s implied structure is drawable.
///
/// `_mkb/timecrystal.md` §5.3 is explicit that raw decoded pixel bytes will
/// not fit `C_H`'s quantisable ceiling — verified separately in
/// `crystallisation_codec.rs` — so this test rescales after decoding, the
/// same way a real caller wanting a quantised video would have to. That
/// rescale happens **after** `decode_ppm`, never inside it: the codec
/// decodes bytes faithfully and does not know what the caller intends to do
/// with them.
#[test]
fn decoded_video_frames_crystallise_and_drive_the_render() {
    // Same order as `crystallisation_codec.rs`'s verified `2e-8`-per-pixel
    // scale for a 2x2 frame; confirmed here (via `is_energy_conserving`
    // below) rather than assumed to carry over unchanged.
    const SCALE: f64 = 3e-9;

    let frames: Vec<PixelGrid> = (0..20)
        .map(|i| {
            let v = (64.0 + 40.0 * (i as f64 * 0.3).sin()) as u8;
            let file = ppm_p5(2, 2, 255, &[v, v, v, v]);
            let decoded = decode_ppm(&file).unwrap();
            let rescaled: Vec<f64> = decoded.pixels().iter().map(|p| p * SCALE).collect();
            PixelGrid::new(decoded.height(), decoded.width(), rescaled).unwrap()
        })
        .collect();

    let vtc = VolumetricTimeCrystal::crystallise_video(frames, 30.0, 3)
        .expect("rescaled decoded frames must fit the quantisable ceiling");
    assert!(vtc.input_energy() > 0.0);
    assert!(vtc.is_energy_conserving());

    let vis = TetryenVisualisation::from_phase_vector(&vtc.nodes()[0], 1.2, 3.0);
    assert!(vis.peak() > 0.0, "a crystallised video's own phase-space node must be drawable");
}

/// 3.2 [D] — **the honest failure mode, exercised through real bytes.**
/// Frames decoded from real PPM files, left at their natural byte-range
/// scale, are refused rather than silently rescaled or truncated — the
/// property `_mkb/timecrystal.md` §5.3 states, now demonstrated starting
/// from actual decoded file content rather than a hand-built signal.
#[test]
fn realistic_decoded_video_is_refused_through_the_full_chain() {
    let frames: Vec<PixelGrid> = (0..10)
        .map(|_| {
            let file = ppm_p5(8, 8, 255, &[128u8; 64]);
            decode_ppm(&file).unwrap()
        })
        .collect();

    let result = VolumetricTimeCrystal::crystallise_video(frames, 30.0, 2);
    assert!(
        matches!(
            result,
            Err(crystallisation::CrystalError::EnergyExceedsQuantisation { .. })
        ),
        "unscaled real decoded frames must be refused, got {result:?}"
    );
}
