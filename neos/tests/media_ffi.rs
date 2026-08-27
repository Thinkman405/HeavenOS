//! Verifies the C FFI bridge (`media_ffi`) against `crystallisation`'s own
//! already-tested pipeline, calling the exact `extern "C"` functions a real
//! C caller would.
//!
//! This file proves the *values* are exactly right. It does not prove the
//! ABI genuinely links from another language — that's
//! `neos/media_ffi/ffi_test/main.c`, a real, independent C program compiled
//! with MSVC against the built `.dll`/`.lib`, which Rust calling its own
//! `extern "C"` functions (as every test here does) cannot substitute for.

use crystallisation::{decode_ppm, FrequencyMap};
use media_ffi::{
    media_ffi_crystallise_image, media_ffi_image_result_coefficient,
    media_ffi_image_result_coefficient_count, media_ffi_image_result_error_message,
    media_ffi_image_result_face_count, media_ffi_image_result_face_energy,
    media_ffi_image_result_free, media_ffi_image_result_is_ok,
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
