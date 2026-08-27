//! The C FFI bridge — `crystallisation`'s pipelines, exposed across a real
//! C ABI boundary, one pipeline at a time.
//!
//! # Why the image pipeline only, for now
//!
//! This is the first pass at the FFI layer this workspace's own plan names
//! (media framework -> adapter -> C FFI bridge). Rather than expose all four
//! `crystallisation` pipelines at once, each pipeline gets its own bridge
//! built and verified in turn — the same discipline this workspace has
//! followed at every other layer boundary (`symphony_lang::sandbox` composed
//! exactly the pieces it needed, not a generalised framework;
//! `crystallisation::parallel` shipped three near-identical functions rather
//! than one generic batch abstraction). An FFI boundary is exactly the wrong
//! place to generalise before the pattern is proven: get the ownership,
//! null-safety, and error discipline right for one real pipeline, verified
//! against a real, independently-compiled C program (not just Rust calling
//! its own `extern "C"` functions, which would not actually prove
//! cross-language safety) — the image bridge went first; the video bridge
//! (`media_ffi_crystallise_video` and friends, below) repeats the identical
//! shape for `crystallisation::timecrystal::VolumetricTimeCrystal`.
//!
//! # The actual safety contract an FFI crate has to uphold
//!
//! Every type crossing this boundary is either a plain scalar (`f64`,
//! `usize`, `c_int`) or an **opaque** pointer — [`MediaFfiImageResult`]'s
//! fields are never visible to C, only its address. That is not a style
//! choice: a struct whose Rust layout is visible to C is one where adding a
//! field later silently breaks every existing C caller, with no compiler
//! error on either side to catch it. C only ever holds
//! `MediaFfiImageResult *`, passed back into accessor functions that read
//! the real (Rust-side) fields on its behalf.
//!
//! Every allocation this crate hands to C is freed by this crate, never by
//! C's own `free()` — Rust's allocator and the C runtime's allocator are not
//! guaranteed to be the same allocator, so crossing that streams is a real,
//! not theoretical, memory-corruption risk. [`media_ffi_image_result_free`]
//! is the one and only way to release a result, and — matching C's own
//! `free(NULL)` convention — tolerates a null pointer as a no-op.
//!
//! # The panic boundary
//!
//! An unwind that reaches an `extern "C" fn` boundary aborts the whole
//! process under current Rust semantics — not silent undefined behaviour,
//! but still not something a C caller with other in-flight work should be
//! exposed to for what might be a single bad input. [`media_ffi_crystallise_image`]
//! is the one function on a real call path into `crystallisation`'s own
//! logic (decode, then transform), so it is the one wrapped in
//! [`std::panic::catch_unwind`]: a caught panic becomes an ordinary error
//! result, exactly like a `CrystalError` would, rather than taking the
//! caller's whole process down. The accessor functions below only ever
//! match/index over data this crate already validated when it was created,
//! so they are not wrapped — there is nothing on their path expected to
//! panic short of the caller already having violated a documented safety
//! precondition, which `catch_unwind` cannot make safe regardless.
//!
//! # Never null on success or failure — the ambiguity that would create
//!
//! [`media_ffi_crystallise_image`] returns a null pointer in exactly one
//! case: the caller passed a null `bytes` pointer, which cannot be turned
//! into anything meaningful. Every other outcome — a real result *or* a
//! real crystallisation error (a malformed PPM, an uneven coefficient
//! projection) — returns a valid, freeable handle. Encoding failure as
//! "returned null" would collide with "the pipeline produced an empty
//! result", which `crystallisation`'s own real pipelines can legitimately
//! do; encoding it *inside* the handle, queried via
//! [`media_ffi_image_result_is_ok`], keeps the two distinguishable.
//!
//! # What this deliberately does not build yet
//!
//! No audio/text bridges (same pattern, not yet repeated — see above). No
//! adapter layer distinct from this crate (the opaque-handle design above
//! *is* the adapter step this workspace's own plan named, folded into the
//! bridge rather than a separate crate, since splitting "translate to
//! FFI-safe shapes" from "expose them as `extern "C"`" into two crates per
//! pipeline would be premature). No `cbindgen`-generated header —
//! `media_ffi.h` is hand-written and verified against this crate's actual
//! signatures by compiling and running a real, independent C program
//! against it; a real, stated risk of that choice is that the two can drift
//! if a signature changes here without the header being updated by hand.

use std::ffi::{c_char, CString};
use std::os::raw::c_int;
use std::slice;

use crystallisation::{takens_embed, Crystal, FrequencyMap, PixelGrid, VolumetricTimeCrystal};

struct FaceSummary {
    energy: f64,
    coefficients: Vec<(f64, f64)>,
}

/// Opaque across the FFI boundary. See the module docs' safety contract —
/// C never sees a field of this struct, only its address.
pub struct MediaFfiImageResult {
    faces: Vec<FaceSummary>,
    error: Option<CString>,
}

impl MediaFfiImageResult {
    fn ok(faces: Vec<FaceSummary>) -> Self {
        Self { faces, error: None }
    }

    fn err(message: String) -> Self {
        // A NUL byte can never occur in a Rust-formatted error message here
        // (every CrystalError variant's Display impl formats plain numbers
        // and static text), so this cannot fail in practice — but "cannot
        // fail in practice" is not a proof, so it is handled rather than
        // unwrapped: a message that somehow did contain one becomes a
        // fallback message instead of a panic across the FFI boundary,
        // which would be undefined behaviour on the C side.
        let error = CString::new(message)
            .unwrap_or_else(|_| CString::new("crystallisation error (message unrepresentable)").unwrap());
        Self {
            faces: Vec::new(),
            error: Some(error),
        }
    }
}

fn crystallise(bytes: &[u8]) -> MediaFfiImageResult {
    let grid: PixelGrid = match crystallisation::decode_ppm(bytes) {
        Ok(g) => g,
        Err(e) => return MediaFfiImageResult::err(e.to_string()),
    };
    let faces = match FrequencyMap::transform(&grid).project_onto_faces() {
        Ok(f) => f,
        Err(e) => return MediaFfiImageResult::err(e.to_string()),
    };
    let summaries = faces
        .iter()
        .map(|face| FaceSummary {
            energy: face.energy(),
            coefficients: face.coefficients().iter().map(|c| (c.re, c.im)).collect(),
        })
        .collect();
    MediaFfiImageResult::ok(summaries)
}

/// Decode a real PPM image (`bytes`, `len` long) and crystallise it through
/// the holographic pipeline, returning an opaque handle.
///
/// Returns a valid handle for both success and every `crystallisation`-level
/// failure (malformed PPM, uneven projection) — check
/// [`media_ffi_image_result_is_ok`] first. Returns null **only** when
/// `bytes` itself is null, since there is nothing to decode in that case.
///
/// # Safety
/// `bytes` must be either null, or point to at least `len` readable bytes
/// for the duration of this call. The returned pointer, if non-null, must
/// eventually be passed to [`media_ffi_image_result_free`] exactly once.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_crystallise_image(
    bytes: *const u8,
    len: usize,
) -> *mut MediaFfiImageResult {
    if bytes.is_null() {
        return std::ptr::null_mut();
    }
    let slice = if len == 0 { &[] } else { slice::from_raw_parts(bytes, len) };
    let outcome = std::panic::catch_unwind(|| crystallise(slice)).unwrap_or_else(|payload| {
        let reason = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "crystallisation panicked with a non-string payload".to_string());
        MediaFfiImageResult::err(format!("internal panic during crystallisation: {reason}"))
    });
    Box::into_raw(Box::new(outcome))
}

/// `1` if `result` holds a real crystallised image, `0` if it holds an
/// error (or `result` itself is null).
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_image`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_image_result_is_ok(result: *const MediaFfiImageResult) -> c_int {
    match result.as_ref() {
        Some(r) => c_int::from(r.error.is_none()),
        None => 0,
    }
}

/// The error message, as a NUL-terminated C string, or null if `result` is
/// ok or is itself null. The returned pointer is borrowed from `result` —
/// valid until `result` is freed, never freed separately.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_image`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_image_result_error_message(
    result: *const MediaFfiImageResult,
) -> *const c_char {
    match result.as_ref().and_then(|r| r.error.as_ref()) {
        Some(msg) => msg.as_ptr(),
        None => std::ptr::null(),
    }
}

/// How many faces `result` holds — always `4` on success (a Tetryen has
/// four faces), `0` on error or a null `result`.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_image`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_image_result_face_count(result: *const MediaFfiImageResult) -> usize {
    result.as_ref().map_or(0, |r| r.faces.len())
}

/// A face's total energy, or `NAN` (checkable via `isnan()` in `<math.h>`)
/// if `result` is null/an error, or `face` is out of range — callers are
/// expected to check [`media_ffi_image_result_is_ok`] and
/// [`media_ffi_image_result_face_count`] first; this never panics either
/// way.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_image`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_image_result_face_energy(
    result: *const MediaFfiImageResult,
    face: usize,
) -> f64 {
    result
        .as_ref()
        .and_then(|r| r.faces.get(face))
        .map_or(f64::NAN, |f| f.energy)
}

/// How many frequency coefficients `face` holds, or `0` if `result`/`face`
/// is invalid.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_image`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_image_result_coefficient_count(
    result: *const MediaFfiImageResult,
    face: usize,
) -> usize {
    result
        .as_ref()
        .and_then(|r| r.faces.get(face))
        .map_or(0, |f| f.coefficients.len())
}

/// Read one complex coefficient out through the two output pointers,
/// demonstrating the output-parameter idiom C APIs rely on for anything
/// wider than a single scalar return. Writes `*out_re`/`*out_im` and
/// returns `1` on success; on any failure (`result` null/an error, `face`
/// or `index` out of range, or `out_re`/`out_im` null) returns `0` and
/// leaves the out-pointers untouched.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_image`] and not yet freed. `out_re`/`out_im`, if
/// non-null, must each point to a writable `f64`.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_image_result_coefficient(
    result: *const MediaFfiImageResult,
    face: usize,
    index: usize,
    out_re: *mut f64,
    out_im: *mut f64,
) -> c_int {
    if out_re.is_null() || out_im.is_null() {
        return 0;
    }
    let Some(&(re, im)) = result
        .as_ref()
        .and_then(|r| r.faces.get(face))
        .and_then(|f| f.coefficients.get(index))
    else {
        return 0;
    };
    *out_re = re;
    *out_im = im;
    1
}

/// Release a handle returned by [`media_ffi_crystallise_image`]. Tolerates
/// a null pointer as a no-op, matching C's own `free(NULL)` convention.
///
/// # Safety
/// `result` must either be null, or a handle returned by
/// [`media_ffi_crystallise_image`] that has not already been freed — double
/// freeing, like double-freeing through any allocator, is undefined
/// behaviour this function cannot detect or prevent.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_image_result_free(result: *mut MediaFfiImageResult) {
    if !result.is_null() {
        drop(Box::from_raw(result));
    }
}

// ============================================================ video bridge
//
// Repeats the image bridge's proven shape exactly for
// `crystallisation::timecrystal::VolumetricTimeCrystal::crystallise_video`:
// an opaque handle, never null except for a null input pointer, freed by
// this crate alone, and the one real-logic call wrapped in
// `catch_unwind`. The one real design decision specific to video: frames
// cross the boundary as a single flat, contiguous buffer of
// `frame_count * width * height` `f64`s (frame-major, then row-major within
// each frame) rather than an array of per-frame pointers — `crystallise_video`
// itself is inherently a batch operation (it needs the whole frame
// sequence's energy time series before any of its FFT/quantisation work can
// start), so there is no streaming/incremental shape to preserve here; a
// caller (e.g. a real-time pipeline) is expected to accumulate frames on
// its own side and call this once it has a full batch, the same way this
// crate's own `neos/media_ffi/ffi_test` C program will.

struct VideoNode {
    components: [f64; 4],
}

/// Opaque across the FFI boundary — see the module docs' safety contract.
pub struct MediaFfiVideoResult {
    nodes: Vec<VideoNode>,
    input_energy: f64,
    energy_conserving: bool,
    fundamental_hz: f64,
    error: Option<CString>,
}

impl MediaFfiVideoResult {
    fn ok(vtc: &VolumetricTimeCrystal) -> Self {
        Self {
            nodes: vtc
                .nodes()
                .iter()
                .map(|n| VideoNode { components: *n.components() })
                .collect(),
            input_energy: vtc.input_energy(),
            energy_conserving: vtc.is_energy_conserving(),
            fundamental_hz: vtc.fundamental().get(),
            error: None,
        }
    }

    fn err(message: String) -> Self {
        let error = CString::new(message)
            .unwrap_or_else(|_| CString::new("crystallisation error (message unrepresentable)").unwrap());
        Self {
            nodes: Vec::new(),
            input_energy: 0.0,
            energy_conserving: false,
            fundamental_hz: 0.0,
            error: Some(error),
        }
    }
}

fn crystallise_video(frames: &[f64], frame_count: usize, width: usize, height: usize, frame_rate: f64, tau: usize) -> MediaFfiVideoResult {
    let per_frame = width * height;
    let mut grids = Vec::with_capacity(frame_count);
    for chunk in frames.chunks_exact(per_frame).take(frame_count) {
        match PixelGrid::new(height, width, chunk.to_vec()) {
            Ok(grid) => grids.push(grid),
            Err(e) => return MediaFfiVideoResult::err(e.to_string()),
        }
    }
    match VolumetricTimeCrystal::crystallise_video(grids, frame_rate, tau) {
        Ok(vtc) => MediaFfiVideoResult::ok(&vtc),
        Err(e) => MediaFfiVideoResult::err(e.to_string()),
    }
}

/// Crystallise a real video (a flat buffer of `frame_count * width * height`
/// pixel values, frame-major then row-major) through the volumetric
/// time-crystal pipeline, returning an opaque handle.
///
/// Returns a valid handle for both success and every `crystallisation`-level
/// failure — check [`media_ffi_video_result_is_ok`] first. Returns null
/// **only** when `frames` itself is null.
///
/// # Safety
/// `frames` must be either null, or point to at least
/// `frame_count * width * height` readable `f64`s for the duration of this
/// call. The returned pointer, if non-null, must eventually be passed to
/// [`media_ffi_video_result_free`] exactly once.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_crystallise_video(
    frames: *const f64,
    frame_count: usize,
    width: usize,
    height: usize,
    frame_rate: f64,
    tau: usize,
) -> *mut MediaFfiVideoResult {
    if frames.is_null() {
        return std::ptr::null_mut();
    }
    let total = frame_count.saturating_mul(width).saturating_mul(height);
    let slice = if total == 0 { &[] } else { slice::from_raw_parts(frames, total) };
    let outcome = std::panic::catch_unwind(|| {
        crystallise_video(slice, frame_count, width, height, frame_rate, tau)
    })
    .unwrap_or_else(|payload| {
        let reason = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "crystallisation panicked with a non-string payload".to_string());
        MediaFfiVideoResult::err(format!("internal panic during video crystallisation: {reason}"))
    });
    Box::into_raw(Box::new(outcome))
}

/// `1` if `result` holds a real crystallised video, `0` if it holds an
/// error (or `result` itself is null).
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_video`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_video_result_is_ok(result: *const MediaFfiVideoResult) -> c_int {
    match result.as_ref() {
        Some(r) => c_int::from(r.error.is_none()),
        None => 0,
    }
}

/// The error message, as a NUL-terminated C string, or null if `result` is
/// ok or is itself null. Borrowed from `result` — valid until `result` is
/// freed, never freed separately.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_video`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_video_result_error_message(
    result: *const MediaFfiVideoResult,
) -> *const c_char {
    match result.as_ref().and_then(|r| r.error.as_ref()) {
        Some(msg) => msg.as_ptr(),
        None => std::ptr::null(),
    }
}

/// How many phase-space nodes `result` holds — `0` on error or a null
/// `result`.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_video`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_video_result_node_count(result: *const MediaFfiVideoResult) -> usize {
    result.as_ref().map_or(0, |r| r.nodes.len())
}

/// Read one phase-space node's four real components through the output
/// pointer. Writes `out_components[0..4]` and returns `1` on success; on
/// any failure (`result` null/an error, `index` out of range, or
/// `out_components` null) returns `0` and leaves the output untouched.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_video`] and not yet freed. `out_components`, if
/// non-null, must point to at least 4 writable `f64`s.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_video_result_node(
    result: *const MediaFfiVideoResult,
    index: usize,
    out_components: *mut f64,
) -> c_int {
    if out_components.is_null() {
        return 0;
    }
    let Some(node) = result.as_ref().and_then(|r| r.nodes.get(index)) else {
        return 0;
    };
    let dst = slice::from_raw_parts_mut(out_components, 4);
    dst.copy_from_slice(&node.components);
    1
}

/// The video's real input energy (Joules), or `NAN` if `result` is
/// null/an error.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_video`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_video_result_input_energy(result: *const MediaFfiVideoResult) -> f64 {
    match result.as_ref() {
        Some(r) if r.error.is_none() => r.input_energy,
        _ => f64::NAN,
    }
}

/// `1` if the crystallisation conserved energy within the half-quantum
/// floor, `0` otherwise (including a null/error `result`).
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_video`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_video_result_is_energy_conserving(
    result: *const MediaFfiVideoResult,
) -> c_int {
    match result.as_ref() {
        Some(r) if r.error.is_none() => c_int::from(r.energy_conserving),
        _ => 0,
    }
}

/// The video's real, Howard-Comma-quantised fundamental frequency (Hz), or
/// `NAN` if `result` is null/an error.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_video`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_video_result_fundamental_hz(result: *const MediaFfiVideoResult) -> f64 {
    match result.as_ref() {
        Some(r) if r.error.is_none() => r.fundamental_hz,
        _ => f64::NAN,
    }
}

/// Release a handle returned by [`media_ffi_crystallise_video`]. Tolerates
/// a null pointer as a no-op, matching C's own `free(NULL)` convention.
///
/// # Safety
/// `result` must either be null, or a handle returned by
/// [`media_ffi_crystallise_video`] that has not already been freed — double
/// freeing is undefined behaviour this function cannot detect or prevent.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_video_result_free(result: *mut MediaFfiVideoResult) {
    if !result.is_null() {
        drop(Box::from_raw(result));
    }
}

// ============================================================ audio bridge
//
// The identical proven shape again, for `crystallisation::takens_embed`.
// The signal crosses the boundary as a flat `f64` buffer — already the
// natural shape here, since `takens_embed` itself takes `&[f64]` with no
// batching/framing to reconcile the way video's frame buffer needed.

struct AudioNode {
    components: [f64; 4],
}

/// Opaque across the FFI boundary — see the module docs' safety contract.
pub struct MediaFfiAudioResult {
    nodes: Vec<AudioNode>,
    error: Option<CString>,
}

impl MediaFfiAudioResult {
    fn ok(nodes: &[crystallisation::PhaseSpaceVector]) -> Self {
        Self {
            nodes: nodes.iter().map(|n| AudioNode { components: *n.components() }).collect(),
            error: None,
        }
    }

    fn err(message: String) -> Self {
        let error = CString::new(message)
            .unwrap_or_else(|_| CString::new("crystallisation error (message unrepresentable)").unwrap());
        Self { nodes: Vec::new(), error: Some(error) }
    }
}

fn embed_audio(signal: &[f64], tau: usize) -> MediaFfiAudioResult {
    match takens_embed(signal, tau) {
        Ok(nodes) => MediaFfiAudioResult::ok(&nodes),
        Err(e) => MediaFfiAudioResult::err(e.to_string()),
    }
}

/// Takens-embed a real signal (`signal`, `len` samples long) at delay
/// `tau`, returning an opaque handle.
///
/// Returns a valid handle for both success and every `crystallisation`-level
/// failure — check [`media_ffi_audio_result_is_ok`] first. Returns null
/// **only** when `signal` itself is null.
///
/// # Safety
/// `signal` must be either null, or point to at least `len` readable
/// `f64`s for the duration of this call. The returned pointer, if
/// non-null, must eventually be passed to [`media_ffi_audio_result_free`]
/// exactly once.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_embed_audio(
    signal: *const f64,
    len: usize,
    tau: usize,
) -> *mut MediaFfiAudioResult {
    if signal.is_null() {
        return std::ptr::null_mut();
    }
    let slice = if len == 0 { &[] } else { slice::from_raw_parts(signal, len) };
    let outcome = std::panic::catch_unwind(|| embed_audio(slice, tau)).unwrap_or_else(|payload| {
        let reason = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "crystallisation panicked with a non-string payload".to_string());
        MediaFfiAudioResult::err(format!("internal panic during audio embedding: {reason}"))
    });
    Box::into_raw(Box::new(outcome))
}

/// `1` if `result` holds a real embedded signal, `0` if it holds an error
/// (or `result` itself is null).
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_embed_audio`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_audio_result_is_ok(result: *const MediaFfiAudioResult) -> c_int {
    match result.as_ref() {
        Some(r) => c_int::from(r.error.is_none()),
        None => 0,
    }
}

/// The error message, as a NUL-terminated C string, or null if `result` is
/// ok or is itself null. Borrowed from `result` — valid until `result` is
/// freed, never freed separately.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_embed_audio`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_audio_result_error_message(
    result: *const MediaFfiAudioResult,
) -> *const c_char {
    match result.as_ref().and_then(|r| r.error.as_ref()) {
        Some(msg) => msg.as_ptr(),
        None => std::ptr::null(),
    }
}

/// How many phase-space nodes `result` holds — `0` on error or a null
/// `result`.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_embed_audio`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_audio_result_node_count(result: *const MediaFfiAudioResult) -> usize {
    result.as_ref().map_or(0, |r| r.nodes.len())
}

/// Read one phase-space node's four real components through the output
/// pointer. Writes `out_components[0..4]` and returns `1` on success; on
/// any failure returns `0` and leaves the output untouched.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_embed_audio`] and not yet freed. `out_components`, if
/// non-null, must point to at least 4 writable `f64`s.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_audio_result_node(
    result: *const MediaFfiAudioResult,
    index: usize,
    out_components: *mut f64,
) -> c_int {
    if out_components.is_null() {
        return 0;
    }
    let Some(node) = result.as_ref().and_then(|r| r.nodes.get(index)) else {
        return 0;
    };
    let dst = slice::from_raw_parts_mut(out_components, 4);
    dst.copy_from_slice(&node.components);
    1
}

/// Release a handle returned by [`media_ffi_embed_audio`]. Tolerates a
/// null pointer as a no-op, matching C's own `free(NULL)` convention.
///
/// # Safety
/// `result` must either be null, or a handle returned by
/// [`media_ffi_embed_audio`] that has not already been freed — double
/// freeing is undefined behaviour this function cannot detect or prevent.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_audio_result_free(result: *mut MediaFfiAudioResult) {
    if !result.is_null() {
        drop(Box::from_raw(result));
    }
}

// ============================================================= text bridge
//
// The identical proven shape again, for `crystallisation::linguistic::Crystal`.
// Text crosses the boundary as raw UTF-8 bytes (`*const u8`, `len`), the
// natural shape for a string across a C ABI — validated with
// `str::from_utf8` inside the panic-guarded call, since invalid UTF-8 is a
// real, expected caller error, not something to unwrap and crash on.

struct TextNode {
    codepoint: u32,
    phase: f64,
}

/// Opaque across the FFI boundary — see the module docs' safety contract.
pub struct MediaFfiTextResult {
    nodes: Vec<TextNode>,
    bifurcations: usize,
    extent: f64,
    error: Option<CString>,
}

impl MediaFfiTextResult {
    fn ok(crystal: &Crystal) -> Self {
        Self {
            nodes: crystal
                .nodes()
                .iter()
                .map(|n| TextNode { codepoint: n.codepoint as u32, phase: n.phase })
                .collect(),
            bifurcations: crystal.bifurcations(),
            extent: crystal.extent(),
            error: None,
        }
    }

    fn err(message: String) -> Self {
        let error = CString::new(message)
            .unwrap_or_else(|_| CString::new("crystallisation error (message unrepresentable)").unwrap());
        Self { nodes: Vec::new(), bifurcations: 0, extent: f64::NAN, error: Some(error) }
    }
}

fn crystallise_text(text: &[u8]) -> MediaFfiTextResult {
    match std::str::from_utf8(text) {
        Ok(s) => match Crystal::crystallise(s) {
            Ok(c) => MediaFfiTextResult::ok(&c),
            Err(e) => MediaFfiTextResult::err(e.to_string()),
        },
        Err(e) => MediaFfiTextResult::err(format!("input is not valid UTF-8: {e}")),
    }
}

/// Crystallise real text (`text`, `len` bytes of UTF-8, not
/// NUL-terminated) through the linguistic pipeline, returning an opaque
/// handle.
///
/// Returns a valid handle for both success and every `crystallisation`-level
/// failure (invalid UTF-8, too many line breaks) — check
/// [`media_ffi_text_result_is_ok`] first. Returns null **only** when
/// `text` itself is null.
///
/// # Safety
/// `text` must be either null, or point to at least `len` readable bytes
/// for the duration of this call. The returned pointer, if non-null, must
/// eventually be passed to [`media_ffi_text_result_free`] exactly once.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_crystallise_text(text: *const u8, len: usize) -> *mut MediaFfiTextResult {
    if text.is_null() {
        return std::ptr::null_mut();
    }
    let slice = if len == 0 { &[] } else { slice::from_raw_parts(text, len) };
    let outcome = std::panic::catch_unwind(|| crystallise_text(slice)).unwrap_or_else(|payload| {
        let reason = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "crystallisation panicked with a non-string payload".to_string());
        MediaFfiTextResult::err(format!("internal panic during text crystallisation: {reason}"))
    });
    Box::into_raw(Box::new(outcome))
}

/// `1` if `result` holds a real crystallised document, `0` if it holds an
/// error (or `result` itself is null).
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_text`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_text_result_is_ok(result: *const MediaFfiTextResult) -> c_int {
    match result.as_ref() {
        Some(r) => c_int::from(r.error.is_none()),
        None => 0,
    }
}

/// The error message, as a NUL-terminated C string, or null if `result` is
/// ok or is itself null. Borrowed from `result` — valid until `result` is
/// freed, never freed separately.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_text`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_text_result_error_message(
    result: *const MediaFfiTextResult,
) -> *const c_char {
    match result.as_ref().and_then(|r| r.error.as_ref()) {
        Some(msg) => msg.as_ptr(),
        None => std::ptr::null(),
    }
}

/// How many harmonic nodes (one per non-newline character) `result` holds
/// — `0` on error or a null `result`.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_text`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_text_result_node_count(result: *const MediaFfiTextResult) -> usize {
    result.as_ref().map_or(0, |r| r.nodes.len())
}

/// Read one harmonic node's Unicode codepoint and phase through the two
/// output pointers. `index` is not returned separately — a node's index
/// is always exactly the `index` passed in, since nodes are stored in
/// sequential order. Writes `*out_codepoint`/`*out_phase` and returns `1`
/// on success; on any failure returns `0` and leaves the outputs untouched.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_text`] and not yet freed. `out_codepoint`/
/// `out_phase`, if non-null, must each point to writable memory of the
/// matching type.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_text_result_node(
    result: *const MediaFfiTextResult,
    index: usize,
    out_codepoint: *mut u32,
    out_phase: *mut f64,
) -> c_int {
    if out_codepoint.is_null() || out_phase.is_null() {
        return 0;
    }
    let Some(node) = result.as_ref().and_then(|r| r.nodes.get(index)) else {
        return 0;
    };
    *out_codepoint = node.codepoint;
    *out_phase = node.phase;
    1
}

/// How many line-break bifurcations `result`'s document had, or `0` on
/// error or a null `result`.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_text`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_text_result_bifurcations(result: *const MediaFfiTextResult) -> usize {
    result.as_ref().map_or(0, |r| r.bifurcations)
}

/// The document's real structural extent after all bifurcations (`1.0`
/// for a single-line document), or `NAN` if `result` is null/an error.
///
/// # Safety
/// `result`, if non-null, must be a handle returned by
/// [`media_ffi_crystallise_text`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_text_result_extent(result: *const MediaFfiTextResult) -> f64 {
    match result.as_ref() {
        Some(r) if r.error.is_none() => r.extent,
        _ => f64::NAN,
    }
}

/// Release a handle returned by [`media_ffi_crystallise_text`]. Tolerates
/// a null pointer as a no-op, matching C's own `free(NULL)` convention.
///
/// # Safety
/// `result` must either be null, or a handle returned by
/// [`media_ffi_crystallise_text`] that has not already been freed —
/// double freeing is undefined behaviour this function cannot detect or
/// prevent.
#[no_mangle]
pub unsafe extern "C" fn media_ffi_text_result_free(result: *mut MediaFfiTextResult) {
    if !result.is_null() {
        drop(Box::from_raw(result));
    }
}
