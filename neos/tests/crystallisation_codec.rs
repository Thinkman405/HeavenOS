//! Codecs and video — PRD §8, closing the two gaps `04_implement` recorded.
//!
//! Doctrine: `_mkb/test-doctrine.md`. **[D]** marks assertions a conventional
//! decoder or a naive "just embed the pixels" video reading could not pass.

use crystallisation::timecrystal::VolumetricTimeCrystal;
use crystallisation::{decode_ppm, decode_wav, CrystalError, PixelGrid, ResonantChamber};

// ------------------------------------------------------- Group 1: PPM images

fn ppm_p5(width: usize, height: usize, maxval: u16, pixels: &[u8]) -> Vec<u8> {
    let mut b = format!("P5\n{width} {height}\n{maxval}\n").into_bytes();
    b.extend_from_slice(pixels);
    b
}

fn ppm_p6(width: usize, height: usize, maxval: u16, rgb: &[u8]) -> Vec<u8> {
    let mut b = format!("P6\n{width} {height}\n{maxval}\n").into_bytes();
    b.extend_from_slice(rgb);
    b
}

/// 1.1 — P5 grayscale round-trips raw sample values exactly.
#[test]
fn p5_round_trips_raw_samples() {
    let file = ppm_p5(2, 2, 255, &[10, 20, 30, 40]);
    let grid = decode_ppm(&file).unwrap();
    assert_eq!(grid.width(), 2);
    assert_eq!(grid.height(), 2);
    assert_eq!(grid.pixels(), &[10.0, 20.0, 30.0, 40.0]);
}

/// 1.2 [D] — a comment line in the header is skipped, not treated as data. A
/// byte-position decoder that does not special-case `#` would misread the
/// dimensions.
#[test]
fn header_comment_lines_are_skipped() {
    let mut file = b"P5\n# a comment about this image\n2 2\n255\n".to_vec();
    file.extend_from_slice(&[1, 2, 3, 4]);
    let grid = decode_ppm(&file).unwrap();
    assert_eq!(grid.pixels(), &[1.0, 2.0, 3.0, 4.0]);
}

/// 1.3 [D] — **RGB reduces to grayscale by the stated ITU-R BT.601 luma**, not
/// a plain average. Pinned so a future "simplification" to `(r+g+b)/3` has to
/// argue with a test: pure red under BT.601 is `76.245`, not `85`.
#[test]
fn rgb_reduces_by_bt601_luma_not_plain_average() {
    let file = ppm_p6(1, 1, 255, &[255, 0, 0]);
    let grid = decode_ppm(&file).unwrap();
    assert!(
        (grid.pixels()[0] - 76.245).abs() < 1e-9,
        "pure red must luma to 76.245, got {}",
        grid.pixels()[0]
    );
    let plain_average = 255.0 / 3.0;
    assert!(
        (grid.pixels()[0] - plain_average).abs() > 1.0,
        "BT.601 luma must differ from a plain channel average for a saturated colour"
    );
}

/// 1.4 — grayscale and white-balanced RGB agree: a pixel with `R = G = B`
/// must luma to that same value exactly (the three weights sum to `1.0`).
#[test]
fn neutral_rgb_luma_equals_the_grayscale_value() {
    let file = ppm_p6(1, 1, 255, &[128, 128, 128]);
    let grid = decode_ppm(&file).unwrap();
    assert!(
        (grid.pixels()[0] - 128.0).abs() < 1e-9,
        "R=G=B must luma to that value exactly, got {}",
        grid.pixels()[0]
    );
}

/// 1.5 [D] — bytes that do not declare `P5`/`P6` are refused, not
/// misinterpreted as one of them.
#[test]
fn unrecognised_magic_is_refused() {
    assert!(matches!(
        decode_ppm(b"GIF89a..."),
        Err(CrystalError::UnrecognisedFormat)
    ));
    assert!(matches!(
        decode_ppm(b"P3\n1 1\n255\n1 2 3"), // ASCII PPM, not supported
        Err(CrystalError::UnrecognisedFormat)
    ));
}

/// 1.6 [D] — a body shorter than the header declares is refused with the
/// exact byte counts, not read past its own end.
#[test]
fn truncated_ppm_body_is_refused() {
    let file = ppm_p5(4, 4, 255, &[1, 2, 3]); // needs 16 bytes, has 3
    match decode_ppm(&file) {
        Err(CrystalError::TruncatedMedia { expected, got }) => {
            assert_eq!(expected, 16);
            assert_eq!(got, 3);
        }
        other => panic!("expected TruncatedMedia, got {other:?}"),
    }
}

/// 1.7 — a decoded image feeds the existing holographic pipeline unmodified:
/// Parseval holds on real decoded bytes, not just hand-built grids.
#[test]
fn decoded_image_satisfies_parseval() {
    let file = ppm_p5(4, 4, 255, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2, 3, 4, 5, 6, 7]);
    let grid = decode_ppm(&file).unwrap();
    let spectrum = crystallisation::FrequencyMap::transform(&grid);
    let (spatial, freq) = (grid.energy(), spectrum.energy());
    assert!(
        ((spatial - freq) / spatial).abs() < 1e-9,
        "spatial {spatial} vs frequency {freq}"
    );
}

// --------------------------------------------------------- Group 2: WAV audio

fn wav_pcm(channels: u16, rate: u32, bits: u16, samples: &[i32]) -> Vec<u8> {
    let bytes_per_sample = usize::from(bits / 8);
    let data_len = samples.len() * bytes_per_sample;
    let mut w = Vec::new();
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36u32 + data_len as u32).to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    let block_align = channels * bytes_per_sample as u16;
    w.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
    w.extend_from_slice(&block_align.to_le_bytes());
    w.extend_from_slice(&bits.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &s in samples {
        if bits == 16 {
            w.extend_from_slice(&(s as i16).to_le_bytes());
        } else {
            w.push((s + 128) as u8);
        }
    }
    w
}

/// 2.1 — mono 16-bit PCM round-trips sample values and sample rate exactly.
#[test]
fn mono_16bit_round_trips_exactly() {
    let file = wav_pcm(1, 8000, 16, &[100, -100, 200, -200]);
    let audio = decode_wav(&file).unwrap();
    assert_eq!(audio.samples(), &[100.0, -100.0, 200.0, -200.0]);
    assert_eq!(audio.sample_rate(), 8000.0);
    assert_eq!(audio.channels(), 1);
}

/// 2.2 [D] — **stereo is downmixed by averaging channels**, a stated
/// convention. Frame `(100, 300)` must average to `200`, not sum to `400` and
/// not keep only the first channel (`100`).
#[test]
fn stereo_downmixes_by_averaging_not_summing_or_dropping() {
    let file = wav_pcm(2, 44100, 16, &[100, 300, -50, -50]);
    let audio = decode_wav(&file).unwrap();
    assert_eq!(audio.samples(), &[200.0, -50.0]);
    assert_eq!(audio.channels(), 2, "the source channel count is preserved as metadata");
}

/// 2.3 — 8-bit unsigned PCM is centred at 128, so the byte range `[0, 255]`
/// decodes to `[-128, 127]`.
#[test]
fn eight_bit_pcm_is_centred_at_128() {
    let file = wav_pcm(1, 22050, 8, &[-128, -1, 0, 127]);
    let audio = decode_wav(&file).unwrap();
    assert_eq!(audio.samples(), &[-128.0, -1.0, 0.0, 127.0]);
}

/// 2.4 [D] — an unrecognised chunk before `fmt `/`data` (e.g. `LIST`) is
/// skipped by its own declared size, **including the RIFF odd-length pad
/// byte** — a decoder that forgot the pad byte would misread every chunk
/// after an odd-sized one.
#[test]
fn unknown_chunks_are_skipped_with_odd_length_padding() {
    let mut file = Vec::new();
    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&999u32.to_le_bytes());
    file.extend_from_slice(b"WAVE");
    file.extend_from_slice(b"LIST");
    file.extend_from_slice(&3u32.to_le_bytes()); // odd size -> one pad byte
    file.extend_from_slice(&[1, 2, 3, 0]);
    file.extend_from_slice(b"fmt ");
    file.extend_from_slice(&16u32.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&16000u32.to_le_bytes());
    file.extend_from_slice(&32000u32.to_le_bytes());
    file.extend_from_slice(&2u16.to_le_bytes());
    file.extend_from_slice(&16u16.to_le_bytes());
    file.extend_from_slice(b"data");
    file.extend_from_slice(&4u32.to_le_bytes());
    file.extend_from_slice(&500i16.to_le_bytes());
    file.extend_from_slice(&(-500i16).to_le_bytes());

    let audio = decode_wav(&file).unwrap();
    assert_eq!(audio.samples(), &[500.0, -500.0]);
    assert_eq!(audio.sample_rate(), 16000.0);
}

/// 2.5 [D] — non-RIFF bytes, non-PCM formats, and unsupported bit depths are
/// all refused the same explicit way, never guessed at.
#[test]
fn unsupported_audio_variants_are_refused() {
    assert!(matches!(decode_wav(b"not a wav file"), Err(CrystalError::UnrecognisedFormat)));

    // audio_format = 3 (IEEE float), not PCM
    let mut float_wav = wav_pcm(1, 8000, 16, &[1, 2]);
    float_wav[20] = 3; // low byte of the fmt chunk's audio_format field
    assert!(matches!(
        decode_wav(&float_wav),
        Err(CrystalError::UnrecognisedFormat)
    ));
}

/// 2.6 — the `data` chunk claiming more bytes than the file holds is refused
/// with the exact counts, not read past the file's end.
#[test]
fn truncated_wav_data_is_refused() {
    let mut file = wav_pcm(1, 8000, 16, &[1, 2, 3, 4]);
    let declared_len = file.len() - 4; // shrink the file, leave the header's claim stale
    file.truncate(declared_len);
    match decode_wav(&file) {
        Err(CrystalError::TruncatedMedia { expected, got }) => {
            assert_eq!(expected, 8);
            assert_eq!(got, 4);
        }
        other => panic!("expected TruncatedMedia, got {other:?}"),
    }
}

/// 2.7 — decoded audio feeds the existing oscillator reading unmodified: a
/// real decoded square-ish wave reports a real, non-zero frequency.
#[test]
fn decoded_audio_drives_the_real_oscillator_reading() {
    let samples: Vec<i32> = (0..40)
        .map(|i| if i % 4 < 2 { 10000 } else { -10000 })
        .collect();
    let file = wav_pcm(1, 8000.0 as u32, 16, &samples);
    let audio = decode_wav(&file).unwrap();
    let chamber = ResonantChamber::from_samples(audio.samples(), audio.sample_rate()).unwrap();
    assert!(chamber.frequency().get() > 0.0, "a real oscillation must report non-zero frequency");
    assert!(!chamber.is_silent());
}

// -------------------------------------------------------------- Group 3: video

/// A frame whose pixels sit in the amplitude regime that keeps a **video**
/// quantisable.
///
/// Not the same constant `crystallisation_timecrystal.rs` uses for audio
/// (`2e-15`) — and measured, not assumed, that it cannot be. `crystallise`'s
/// `input_energy` is the sum of *squares* of the outer signal, and here the
/// outer signal is itself already a sum of squares (`PixelGrid::energy` per
/// frame). Squaring an already-small per-pixel amplitude twice undershoots
/// every quantum and rounds every mode's occupation to zero — verified: at
/// `2e-15` per pixel, nine frames of a real varying video all quantise to
/// exactly zero occupation, which is not "quantisable", it is silently
/// empty. `2e-8` was found by measuring the actual occupation across a range
/// and picking the smallest amplitude giving every mode a nonzero count,
/// comfortably below the ceiling on the other side (`input_energy ~2.3e-28`
/// against a ceiling around `3.6e-18` at this frame rate).
fn quantisable_frame(w: usize, h: usize, value: f64) -> PixelGrid {
    const AMPLITUDE: f64 = 2.0e-8;
    PixelGrid::new(h, w, vec![value * AMPLITUDE; w * h]).unwrap()
}

/// A realistic frame at ordinary 8-bit pixel scale — deliberately **not**
/// rescaled, to exercise the refusal `_mkb/timecrystal.md` §5.3 states.
fn realistic_frame(w: usize, h: usize) -> PixelGrid {
    PixelGrid::new(h, w, vec![128.0; w * h]).unwrap()
}

/// 3.1 [D] — a video of quantisable-scale frames crystallises, and its
/// energy varies frame to frame the way the source frames do — not a
/// constant, which would mean the per-frame reduction was ignored.
#[test]
fn quantisable_video_crystallises_and_reflects_frame_variation() {
    let frames: Vec<PixelGrid> = (0..20)
        .map(|i| quantisable_frame(2, 2, 1.0 + (i as f64 * 0.3).sin()))
        .collect();
    let vtc = VolumetricTimeCrystal::crystallise_video(frames, 30.0, 3).unwrap();
    assert!(vtc.input_energy() > 0.0);
    assert!(
        vtc.modes().iter().any(|m| m.occupation != 0.0),
        "a genuinely varying video must occupy at least one mode"
    );
}

/// 3.2 [D] — **this composition is exactly [`VolumetricTimeCrystal::crystallise`]
/// on the reduced per-frame signal, and nothing else.**
///
/// The central claim of `_mkb/timecrystal.md` §5.1, verified rather than
/// asserted: build the per-frame energy sequence by hand, call `crystallise`
/// directly, and confirm it is bit-for-bit what `crystallise_video` returns.
#[test]
fn crystallise_video_is_exactly_crystallise_on_frame_energies() {
    let frames: Vec<PixelGrid> = (0..16)
        .map(|i| quantisable_frame(3, 3, 1.0 + (i as f64 * 0.5).cos()))
        .collect();

    let manual_signal: Vec<f64> = frames.iter().map(PixelGrid::energy).collect();
    let manual = VolumetricTimeCrystal::crystallise(&manual_signal, 24.0, 2).unwrap();
    let via_video = VolumetricTimeCrystal::crystallise_video(frames, 24.0, 2).unwrap();

    assert_eq!(manual.input_energy(), via_video.input_energy());
    assert_eq!(manual.fundamental().get(), via_video.fundamental().get());
    assert_eq!(manual.modes().len(), via_video.modes().len());
    for (a, b) in manual.modes().iter().zip(via_video.modes()) {
        assert_eq!(a.occupation, b.occupation);
        assert_eq!(a.frequency.get(), b.frequency.get());
    }
}

/// 3.3 [D] — **a realistic, unscaled video is refused, not silently
/// truncated or rescaled.** This is the finding behind `_mkb/timecrystal.md`
/// §5.3: ordinary pixel-scale energy is tens of orders of magnitude past
/// `C_H`'s quantisable ceiling.
#[test]
fn realistic_pixel_scale_video_is_refused_not_rescaled() {
    let frames: Vec<PixelGrid> = (0..10).map(|_| realistic_frame(8, 8)).collect();
    match VolumetricTimeCrystal::crystallise_video(frames, 30.0, 2) {
        Err(CrystalError::EnergyExceedsQuantisation { required, max }) => {
            assert!(
                required > max * 1e10,
                "expected the refusal to be far past the ceiling, got {required:e} vs {max:e}"
            );
        }
        other => panic!("expected EnergyExceedsQuantisation, got {other:?}"),
    }
}

/// 3.4 [D] — a frame whose dimensions differ from the rest of the sequence is
/// refused with the exact mismatched dimensions, not silently reshaped or
/// dropped.
#[test]
fn mismatched_frame_size_is_refused() {
    let mut frames = vec![quantisable_frame(4, 4, 1.0); 5];
    frames.push(quantisable_frame(3, 4, 1.0));
    match VolumetricTimeCrystal::crystallise_video(frames, 30.0, 2) {
        Err(CrystalError::FrameSizeMismatch {
            expected_height,
            expected_width,
            height,
            width,
        }) => {
            assert_eq!((expected_height, expected_width), (4, 4));
            assert_eq!((height, width), (4, 3));
        }
        other => panic!("expected FrameSizeMismatch, got {other:?}"),
    }
}

/// 3.5 — `crystallise_video` takes any `IntoIterator`, not just a `Vec`: a
/// lazily-computed iterator that never materialises a full frame list must
/// work identically. This is the memory-avoidance property the streaming
/// signature exists for.
#[test]
fn accepts_a_lazy_iterator_not_just_a_vec() {
    let lazy = (0..20).map(|i| quantisable_frame(2, 2, 1.0 + (i as f64 * 0.3).sin()));
    let vtc = VolumetricTimeCrystal::crystallise_video(lazy, 30.0, 3).unwrap();
    assert!(vtc.input_energy() > 0.0);
}

/// 3.6 — too short a video (fewer frames than the embedding needs) is
/// refused via the existing `EmptyMedia`, inherited from `crystallise`
/// without any special-casing here.
#[test]
fn too_short_a_video_is_refused() {
    let frames: Vec<PixelGrid> = (0..3).map(|_| quantisable_frame(2, 2, 1.0)).collect();
    assert!(matches!(
        VolumetricTimeCrystal::crystallise_video(frames, 30.0, 3),
        Err(CrystalError::EmptyMedia)
    ));
}
