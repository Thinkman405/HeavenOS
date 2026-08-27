//! Verifies the C FFI bridge (`media_ffi`) against `crystallisation`'s own
//! already-tested pipeline, calling the exact `extern "C"` functions a real
//! C caller would.
//!
//! This file proves the *values* are exactly right. It does not prove the
//! ABI genuinely links from another language — that's
//! `neos/media_ffi/ffi_test/main.c`, a real, independent C program compiled
//! with MSVC against the built `.dll`/`.lib`, which Rust calling its own
//! `extern "C"` functions (as every test here does) cannot substitute for.

use crystallisation::{decode_ppm, takens_embed, Crystal, FrequencyMap, PixelGrid, VolumetricTimeCrystal};
use media_ffi::{
    media_ffi_audio_result_error_message, media_ffi_audio_result_free, media_ffi_audio_result_is_ok,
    media_ffi_audio_result_node, media_ffi_audio_result_node_count, media_ffi_crystallise_image,
    media_ffi_crystallise_text, media_ffi_crystallise_video, media_ffi_embed_audio,
    media_ffi_image_result_coefficient, media_ffi_image_result_coefficient_count,
    media_ffi_image_result_error_message, media_ffi_image_result_face_count,
    media_ffi_image_result_face_energy, media_ffi_image_result_free, media_ffi_image_result_is_ok,
    media_ffi_text_result_bifurcations, media_ffi_text_result_error_message,
    media_ffi_text_result_extent, media_ffi_text_result_free, media_ffi_text_result_is_ok,
    media_ffi_text_result_node, media_ffi_text_result_node_count, media_ffi_video_result_error_message,
    media_ffi_video_result_free, media_ffi_video_result_fundamental_hz,
    media_ffi_video_result_input_energy, media_ffi_video_result_is_energy_conserving,
    media_ffi_video_result_is_ok, media_ffi_video_result_node, media_ffi_video_result_node_count,
};

/// Identical to `neos/src/main.rs`'s own `embedded_ppm()`.
fn embedded_ppm() -> Vec<u8> {
    let pixels: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2, 3, 4, 5, 6, 7];
    let mut bytes = b"P5\n4 4\n255\n".to_vec();
    bytes.extend_from_slice(pixels);
    bytes
}

/// The load-bearing claim: every value the FFI bridge reports must be
/// **exactly** what calling `crystallisation` directly produces — not
/// approximately, since it is the identical computation underneath, just
/// reached through the `extern "C"` surface instead of the Rust API.
#[test]
fn crystallise_image_matches_crystallisations_own_pipeline_exactly() {
    let bytes = embedded_ppm();
    let grid = decode_ppm(&bytes).expect("well-formed embedded PPM");
    let expected = FrequencyMap::transform(&grid)
        .project_onto_faces()
        .expect("16 coefficients divide evenly across 4 faces");

    unsafe {
        let result = media_ffi_crystallise_image(bytes.as_ptr(), bytes.len());
        assert!(!result.is_null(), "a well-formed image must return a handle");
        assert_eq!(media_ffi_image_result_is_ok(result), 1);
        assert_eq!(media_ffi_image_result_face_count(result), expected.len());

        for (face_idx, face) in expected.iter().enumerate() {
            let energy = media_ffi_image_result_face_energy(result, face_idx);
            assert_eq!(energy, face.energy(), "face {face_idx}: energy must match exactly");

            let count = media_ffi_image_result_coefficient_count(result, face_idx);
            assert_eq!(count, face.coefficients().len(), "face {face_idx}: coefficient count");

            for (i, c) in face.coefficients().iter().enumerate() {
                let (mut re, mut im) = (0.0, 0.0);
                let ok = media_ffi_image_result_coefficient(result, face_idx, i, &mut re, &mut im);
                assert_eq!(ok, 1, "face {face_idx}, coefficient {i}: read must succeed");
                assert_eq!(re, c.re, "face {face_idx}, coefficient {i}: real part");
                assert_eq!(im, c.im, "face {face_idx}, coefficient {i}: imaginary part");
            }
        }

        media_ffi_image_result_free(result);
    }
}

#[test]
fn a_null_byte_pointer_returns_null_directly() {
    unsafe {
        let result = media_ffi_crystallise_image(std::ptr::null(), 100);
        assert!(result.is_null(), "nothing can be decoded from a null pointer");
    }
}

/// The central design claim: a real crystallisation failure still returns
/// a valid, freeable handle — never null, which would be indistinguishable
/// from "nothing to report" at the call site.
#[test]
fn a_malformed_image_reports_a_real_error_not_a_null_handle() {
    let bytes = b"P5\n4 4\n255\n\x01\x02\x03".to_vec();
    unsafe {
        let result = media_ffi_crystallise_image(bytes.as_ptr(), bytes.len());
        assert!(!result.is_null(), "an error is still a valid, freeable handle");
        assert_eq!(media_ffi_image_result_is_ok(result), 0);
        assert_eq!(media_ffi_image_result_face_count(result), 0);

        let msg_ptr = media_ffi_image_result_error_message(result);
        assert!(!msg_ptr.is_null(), "a failed handle must carry a real error message");
        let msg = std::ffi::CStr::from_ptr(msg_ptr)
            .to_str()
            .expect("CrystalError's Display never emits non-UTF-8 bytes");
        assert!(!msg.is_empty());

        media_ffi_image_result_free(result);
    }
}

/// Out-of-range access must be refused cleanly, not read past the real data
/// — the output pointers are left completely untouched on refusal, which a
/// caller relies on to distinguish "nothing was written" from "zero was
/// written".
#[test]
fn out_of_range_access_is_refused_not_undefined() {
    let bytes = embedded_ppm();
    unsafe {
        let result = media_ffi_crystallise_image(bytes.as_ptr(), bytes.len());
        assert!(media_ffi_image_result_face_energy(result, 99).is_nan());
        assert_eq!(media_ffi_image_result_coefficient_count(result, 99), 0);

        let (mut re, mut im) = (-999.0, -999.0);
        let ok = media_ffi_image_result_coefficient(result, 99, 0, &mut re, &mut im);
        assert_eq!(ok, 0);
        assert_eq!(re, -999.0, "a refused read must not touch the output pointer");
        assert_eq!(im, -999.0, "a refused read must not touch the output pointer");

        media_ffi_image_result_free(result);
    }
}

/// Matches C's own `free(NULL)` convention.
#[test]
fn freeing_a_null_handle_does_not_panic() {
    unsafe {
        media_ffi_image_result_free(std::ptr::null_mut());
    }
}

/// Every accessor must be defensive against a null `result`, not just the
/// allocator functions — a caller that mishandled one null check elsewhere
/// should not crash the process on the next call.
#[test]
fn accessors_on_a_null_result_are_defensive_not_undefined() {
    unsafe {
        assert_eq!(media_ffi_image_result_is_ok(std::ptr::null()), 0);
        assert!(media_ffi_image_result_error_message(std::ptr::null()).is_null());
        assert_eq!(media_ffi_image_result_face_count(std::ptr::null()), 0);
        assert!(media_ffi_image_result_face_energy(std::ptr::null(), 0).is_nan());
        assert_eq!(media_ffi_image_result_coefficient_count(std::ptr::null(), 0), 0);

        let (mut re, mut im) = (-1.0, -1.0);
        let ok = media_ffi_image_result_coefficient(std::ptr::null(), 0, 0, &mut re, &mut im);
        assert_eq!(ok, 0);
        assert_eq!(re, -1.0);
        assert_eq!(im, -1.0);
    }
}

// ------------------------------------------------------------------ video

/// Same scale `tests/crystallisation_codec.rs`'s own `quantisable_frame`
/// uses — a realistic 8-bit-scale frame overflows the Howard-Comma
/// quantisable ceiling long before a video's worth of frames finishes.
const QUANTISABLE_AMPLITUDE: f64 = 2.0e-8;

fn quantisable_value(i: usize) -> f64 {
    (1.0 + (i as f64 * 0.3).sin()) * QUANTISABLE_AMPLITUDE
}

/// The exact flat, frame-major/row-major buffer `media_ffi_crystallise_video`
/// expects.
fn quantisable_video_buffer(frame_count: usize, width: usize, height: usize) -> Vec<f64> {
    let mut buf = Vec::with_capacity(frame_count * width * height);
    for i in 0..frame_count {
        buf.extend(std::iter::repeat(quantisable_value(i)).take(width * height));
    }
    buf
}

/// The identical video, built the way `crystallisation`'s own API expects,
/// for the bit-for-bit cross-check.
fn quantisable_video_frames(frame_count: usize, width: usize, height: usize) -> Vec<PixelGrid> {
    (0..frame_count)
        .map(|i| PixelGrid::new(height, width, vec![quantisable_value(i); width * height]).unwrap())
        .collect()
}

#[test]
fn crystallise_video_matches_crystallisations_own_pipeline_exactly() {
    let (frame_count, width, height, frame_rate, tau) = (20usize, 2usize, 2usize, 30.0, 3usize);
    let buffer = quantisable_video_buffer(frame_count, width, height);
    let frames = quantisable_video_frames(frame_count, width, height);
    let expected = VolumetricTimeCrystal::crystallise_video(frames, frame_rate, tau)
        .expect("a real, quantisable-scale video crystallises");

    unsafe {
        let result = media_ffi_crystallise_video(buffer.as_ptr(), frame_count, width, height, frame_rate, tau);
        assert!(!result.is_null(), "a well-formed video must return a handle");
        assert_eq!(media_ffi_video_result_is_ok(result), 1);

        assert_eq!(media_ffi_video_result_node_count(result), expected.nodes().len());
        assert_eq!(media_ffi_video_result_input_energy(result), expected.input_energy());
        assert_eq!(
            media_ffi_video_result_is_energy_conserving(result),
            if expected.is_energy_conserving() { 1 } else { 0 }
        );
        assert_eq!(media_ffi_video_result_fundamental_hz(result), expected.fundamental().get());

        for (i, node) in expected.nodes().iter().enumerate() {
            let mut out = [0.0; 4];
            let ok = media_ffi_video_result_node(result, i, out.as_mut_ptr());
            assert_eq!(ok, 1, "node {i}: read must succeed");
            assert_eq!(&out, node.components(), "node {i}: components must match exactly");
        }

        media_ffi_video_result_free(result);
    }
}

#[test]
fn a_null_frame_buffer_returns_null_directly() {
    unsafe {
        let result = media_ffi_crystallise_video(std::ptr::null(), 20, 2, 2, 30.0, 3);
        assert!(result.is_null());
    }
}

/// Real 8-bit-scale pixel values (never rescaled, per
/// `_mkb/timecrystal.md` §5.3), not garbage bytes — the same kind of
/// honest, real-world failure `crystallisation_codec.rs`'s own
/// `realistic_frame` exercises.
#[test]
fn an_unrescaled_video_reports_a_real_error_not_a_null_handle() {
    let (frame_count, width, height) = (5usize, 2usize, 2usize);
    let buffer = vec![128.0; frame_count * width * height];
    unsafe {
        let result = media_ffi_crystallise_video(buffer.as_ptr(), frame_count, width, height, 30.0, 2);
        assert!(!result.is_null(), "an error is still a valid, freeable handle");
        assert_eq!(media_ffi_video_result_is_ok(result), 0);
        assert_eq!(media_ffi_video_result_node_count(result), 0);

        let msg_ptr = media_ffi_video_result_error_message(result);
        assert!(!msg_ptr.is_null());
        let msg = std::ffi::CStr::from_ptr(msg_ptr).to_str().unwrap();
        assert!(!msg.is_empty());

        media_ffi_video_result_free(result);
    }
}

#[test]
fn video_out_of_range_node_access_is_refused_not_undefined() {
    let (frame_count, width, height, frame_rate, tau) = (20usize, 2usize, 2usize, 30.0, 3usize);
    let buffer = quantisable_video_buffer(frame_count, width, height);
    unsafe {
        let result = media_ffi_crystallise_video(buffer.as_ptr(), frame_count, width, height, frame_rate, tau);
        let mut out = [-999.0; 4];
        let ok = media_ffi_video_result_node(result, 9_999, out.as_mut_ptr());
        assert_eq!(ok, 0);
        assert_eq!(out, [-999.0; 4], "a refused read must not touch the output buffer");
        media_ffi_video_result_free(result);
    }
}

#[test]
fn freeing_a_null_video_handle_does_not_panic() {
    unsafe {
        media_ffi_video_result_free(std::ptr::null_mut());
    }
}

// ------------------------------------------------------------------ audio

fn embedded_signal(len: usize) -> Vec<f64> {
    (0..len).map(|i| (i as f64 * 0.4).sin()).collect()
}

#[test]
fn embed_audio_matches_crystallisations_own_pipeline_exactly() {
    let signal = embedded_signal(64);
    let tau = 3;
    let expected = takens_embed(&signal, tau).expect("64 samples embed at tau=3");

    unsafe {
        let result = media_ffi_embed_audio(signal.as_ptr(), signal.len(), tau);
        assert!(!result.is_null(), "a well-formed signal must return a handle");
        assert_eq!(media_ffi_audio_result_is_ok(result), 1);
        assert_eq!(media_ffi_audio_result_node_count(result), expected.len());

        for (i, node) in expected.iter().enumerate() {
            let mut out = [0.0; 4];
            let ok = media_ffi_audio_result_node(result, i, out.as_mut_ptr());
            assert_eq!(ok, 1, "node {i}: read must succeed");
            assert_eq!(&out, node.components(), "node {i}: components must match exactly");
        }

        media_ffi_audio_result_free(result);
    }
}

#[test]
fn a_null_signal_pointer_returns_null_directly() {
    unsafe {
        let result = media_ffi_embed_audio(std::ptr::null(), 64, 3);
        assert!(result.is_null());
    }
}

/// A signal too short for the requested `tau` — a real, honest failure
/// (`takens_embed` needs enough samples to actually form an embedded
/// vector), not garbage input.
#[test]
fn a_signal_too_short_for_tau_reports_a_real_error_not_a_null_handle() {
    let signal = vec![1.0, 2.0];
    unsafe {
        let result = media_ffi_embed_audio(signal.as_ptr(), signal.len(), 10);
        assert!(!result.is_null(), "an error is still a valid, freeable handle");
        assert_eq!(media_ffi_audio_result_is_ok(result), 0);
        assert_eq!(media_ffi_audio_result_node_count(result), 0);

        let msg_ptr = media_ffi_audio_result_error_message(result);
        assert!(!msg_ptr.is_null());
        let msg = std::ffi::CStr::from_ptr(msg_ptr).to_str().unwrap();
        assert!(!msg.is_empty());

        media_ffi_audio_result_free(result);
    }
}

#[test]
fn audio_out_of_range_node_access_is_refused_not_undefined() {
    let signal = embedded_signal(64);
    unsafe {
        let result = media_ffi_embed_audio(signal.as_ptr(), signal.len(), 3);
        let mut out = [-999.0; 4];
        let ok = media_ffi_audio_result_node(result, 9_999, out.as_mut_ptr());
        assert_eq!(ok, 0);
        assert_eq!(out, [-999.0; 4], "a refused read must not touch the output buffer");
        media_ffi_audio_result_free(result);
    }
}

#[test]
fn freeing_a_null_audio_handle_does_not_panic() {
    unsafe {
        media_ffi_audio_result_free(std::ptr::null_mut());
    }
}

// ------------------------------------------------------------------- text

#[test]
fn crystallise_text_matches_crystallisations_own_pipeline_exactly() {
    let text = "first\nsecond line\nthird";
    let expected = Crystal::crystallise(text).expect("well under the four-break ceiling");

    unsafe {
        let result = media_ffi_crystallise_text(text.as_ptr(), text.len());
        assert!(!result.is_null(), "a well-formed document must return a handle");
        assert_eq!(media_ffi_text_result_is_ok(result), 1);
        assert_eq!(media_ffi_text_result_node_count(result), expected.len());
        assert_eq!(media_ffi_text_result_bifurcations(result), expected.bifurcations());
        assert_eq!(media_ffi_text_result_extent(result), expected.extent());

        for (i, node) in expected.nodes().iter().enumerate() {
            let (mut codepoint, mut phase) = (0u32, 0.0f64);
            let ok = media_ffi_text_result_node(result, i, &mut codepoint, &mut phase);
            assert_eq!(ok, 1, "node {i}: read must succeed");
            assert_eq!(codepoint, node.codepoint as u32, "node {i}: codepoint must match exactly");
            assert_eq!(phase, node.phase, "node {i}: phase must match exactly");
        }

        media_ffi_text_result_free(result);
    }
}

#[test]
fn a_null_text_pointer_returns_null_directly() {
    unsafe {
        let result = media_ffi_crystallise_text(std::ptr::null(), 10);
        assert!(result.is_null());
    }
}

/// Over the four-real-break ceiling (`Crystal::max_bifurcations() == 3`) —
/// a real refusal, not truncation.
#[test]
fn an_over_deep_document_reports_a_real_error_not_a_null_handle() {
    let text = "a\n".repeat(4);
    unsafe {
        let result = media_ffi_crystallise_text(text.as_ptr(), text.len());
        assert!(!result.is_null(), "an error is still a valid, freeable handle");
        assert_eq!(media_ffi_text_result_is_ok(result), 0);
        assert_eq!(media_ffi_text_result_node_count(result), 0);
        assert!(media_ffi_text_result_extent(result).is_nan());

        let msg_ptr = media_ffi_text_result_error_message(result);
        assert!(!msg_ptr.is_null());
        let msg = std::ffi::CStr::from_ptr(msg_ptr).to_str().unwrap();
        assert!(!msg.is_empty());

        media_ffi_text_result_free(result);
    }
}

/// Invalid UTF-8 is a real, expected caller error at this boundary — the
/// bridge must report it cleanly, not panic on an internal `str::from_utf8`
/// unwrap.
#[test]
fn invalid_utf8_reports_a_real_error_not_a_panic() {
    let bytes: [u8; 3] = [b'a', 0x80, b'b']; // 0x80 is a bare continuation byte
    unsafe {
        let result = media_ffi_crystallise_text(bytes.as_ptr(), bytes.len());
        assert!(!result.is_null());
        assert_eq!(media_ffi_text_result_is_ok(result), 0);
        let msg_ptr = media_ffi_text_result_error_message(result);
        assert!(!msg_ptr.is_null());
        media_ffi_text_result_free(result);
    }
}

#[test]
fn text_out_of_range_node_access_is_refused_not_undefined() {
    let text = "hello";
    unsafe {
        let result = media_ffi_crystallise_text(text.as_ptr(), text.len());
        let (mut codepoint, mut phase) = (999u32, -999.0f64);
        let ok = media_ffi_text_result_node(result, 9_999, &mut codepoint, &mut phase);
        assert_eq!(ok, 0);
        assert_eq!(codepoint, 999, "a refused read must not touch the output");
        assert_eq!(phase, -999.0, "a refused read must not touch the output");
        media_ffi_text_result_free(result);
    }
}

#[test]
fn freeing_a_null_text_handle_does_not_panic() {
    unsafe {
        media_ffi_text_result_free(std::ptr::null_mut());
    }
}
