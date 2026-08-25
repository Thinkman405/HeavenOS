//! Decoders for the flat media this crate crystallises.
//!
//! PRD §8 crystallises "flat media" — but until this module, nothing upstream
//! ever read one from an actual file. [`crate::PixelGrid::new`] and
//! [`crate::VolumetricTimeCrystal::crystallise`] both took pre-decoded
//! numbers; this is where a real byte stream becomes those numbers.
//!
//! ## Two formats, chosen for what they do not need
//!
//! PNG needs DEFLATE; most audio containers need a codec. Both are out of
//! scope for a from-scratch numeric kernel with no runtime dependencies —
//! pulling one in for a single format would be the same complexity
//! `CLAUDE.md`'s "closed-form... rather than runtime graph discovery" rule
//! warns against, just moved to the data layer. So the two formats decoded
//! here are the two that need only header arithmetic and a byte copy:
//!
//! - **PPM** (P5 grayscale, P6 RGB) for images — the netpbm formats, an
//!   uncompressed ASCII-header/binary-body format designed to be this simple.
//! - **WAV** (PCM, 8-bit unsigned or 16-bit signed, any channel count) for
//!   audio — a RIFF container around linear samples, no compression.
//!
//! Neither format is invented: both are real, standard, and predate this
//! project by decades. What *is* a **stated convention**, not a distillation,
//! is RGB → grayscale luma and multi-channel → mono downmix — see
//! [`decode_ppm`] and [`decode_wav`].

use crate::holographic::PixelGrid;
use crate::CrystalError;

// -------------------------------------------------------------------- PPM

/// ITU-R BT.601 luma weights — the standard grayscale reduction for
/// broadcast-range RGB. A real, external standard, not NEOS law; recorded
/// here rather than in `_mkb/` because it governs how a *third-party file
/// format* is read, not a physical quantity this project defines.
const LUMA_R: f64 = 0.299;
const LUMA_G: f64 = 0.587;
const LUMA_B: f64 = 0.114;

fn skip_ws_and_comments(b: &[u8], mut i: usize) -> usize {
    loop {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < b.len() && b[i] == b'#' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else {
            break;
        }
    }
    i
}

fn read_uint(b: &[u8], i: usize) -> Result<(usize, usize), CrystalError> {
    let start = i;
    let mut j = i;
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
    }
    if j == start {
        return Err(CrystalError::UnrecognisedFormat);
    }
    let v = std::str::from_utf8(&b[start..j])
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or(CrystalError::UnrecognisedFormat)?;
    Ok((v, j))
}

/// Decode a binary PPM (`P5` grayscale or `P6` RGB) into a [`PixelGrid`].
///
/// Pixel values are the file's raw sample magnitudes (`0..=maxval`), **not**
/// normalised to `[0, 1]` — only relative structure drives every downstream
/// computation in this crate (energy ratios, Parseval, `xi`-style
/// corrections elsewhere in NEOS), so there is no reason to invent a target
/// scale the format itself does not specify.
///
/// RGB is reduced to grayscale by ITU-R BT.601 luma
/// (`0.299 R + 0.587 G + 0.114 B`) — a stated, standard convention, not a
/// derived one. A caller that wants the raw channels should not call this;
/// there is no channel-preserving path here, matching [`PixelGrid`]'s single
/// scalar per pixel.
///
/// Supports 1-byte (`maxval < 256`) and 2-byte big-endian (`maxval` up to
/// `65535`) samples, per the PPM specification.
///
/// # Errors
/// [`CrystalError::UnrecognisedFormat`] if the magic bytes are not `P5`/`P6`
/// or the header cannot be parsed. [`CrystalError::TruncatedMedia`] if the
/// body is shorter than the header declares. Otherwise whatever
/// [`PixelGrid::new`] returns, which validates the decoded count directly —
/// one home for that check rather than a second one here.
pub fn decode_ppm(bytes: &[u8]) -> Result<PixelGrid, CrystalError> {
    if bytes.len() < 2 || bytes[0] != b'P' {
        return Err(CrystalError::UnrecognisedFormat);
    }
    let rgb = match bytes[1] {
        b'5' => false,
        b'6' => true,
        _ => return Err(CrystalError::UnrecognisedFormat),
    };

    let mut i = 2;
    i = skip_ws_and_comments(bytes, i);
    let (width, ni) = read_uint(bytes, i)?;
    i = skip_ws_and_comments(bytes, ni);
    let (height, ni) = read_uint(bytes, i)?;
    i = skip_ws_and_comments(bytes, ni);
    let (maxval, ni) = read_uint(bytes, i)?;
    i = ni;
    if maxval == 0 || maxval > 65535 || i >= bytes.len() || !bytes[i].is_ascii_whitespace() {
        return Err(CrystalError::UnrecognisedFormat);
    }
    i += 1; // exactly one whitespace byte terminates the header, per spec

    let bytes_per_sample = if maxval < 256 { 1 } else { 2 };
    let channels = if rgb { 3 } else { 1 };
    let n_pixels = width.saturating_mul(height);
    let needed = n_pixels
        .saturating_mul(channels)
        .saturating_mul(bytes_per_sample);
    let body = &bytes[i..];
    if body.len() < needed {
        return Err(CrystalError::TruncatedMedia {
            expected: needed,
            got: body.len(),
        });
    }

    let sample = |off: usize| -> f64 {
        if bytes_per_sample == 1 {
            body[off] as f64
        } else {
            (u16::from(body[off]) << 8 | u16::from(body[off + 1])) as f64
        }
    };

    let mut pixels = Vec::with_capacity(n_pixels);
    for p in 0..n_pixels {
        if rgb {
            let base = p * 3 * bytes_per_sample;
            let r = sample(base);
            let g = sample(base + bytes_per_sample);
            let b = sample(base + 2 * bytes_per_sample);
            pixels.push(LUMA_R * r + LUMA_G * g + LUMA_B * b);
        } else {
            pixels.push(sample(p * bytes_per_sample));
        }
    }

    PixelGrid::new(height, width, pixels)
}

// -------------------------------------------------------------------- WAV

fn le_u32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}
fn le_u16(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

/// Decoded PCM audio: samples, sample rate, and the original channel count.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioSamples {
    samples: Vec<f64>,
    sample_rate: f64,
    channels: u16,
}

impl AudioSamples {
    /// The mono signal — see [`decode_wav`] for the downmix convention.
    pub fn samples(&self) -> &[f64] {
        &self.samples
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Channel count of the *source* file. `samples()` is always mono
    /// regardless of this value.
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

/// Decode a PCM WAV file (RIFF/WAVE, 8-bit unsigned or 16-bit signed) into
/// [`AudioSamples`].
///
/// Multi-channel files are downmixed to mono by averaging every channel's
/// sample within each frame — a stated convention, the common one, not a
/// derived one. A caller that wants channels kept apart should not call
/// this.
///
/// Chunks are walked generically (`fmt ` and `data` are read wherever they
/// appear; anything else — `LIST`, `fact`, ... — is skipped by its declared
/// size, with the RIFF odd-length pad byte honoured), so extra chunks before
/// `data` do not break decoding.
///
/// # Errors
/// [`CrystalError::UnrecognisedFormat`] if the file is not `RIFF`/`WAVE`, the
/// audio format is not PCM (`1`), the bit depth is not `8` or `16`, or no
/// `fmt `/`data` chunk is found. [`CrystalError::TruncatedMedia`] if the
/// `data` chunk claims more bytes than the file actually holds.
pub fn decode_wav(bytes: &[u8]) -> Result<AudioSamples, CrystalError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(CrystalError::UnrecognisedFormat);
    }

    let mut i = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None; // format, channels, rate, bits
    let mut data: Option<(usize, usize)> = None; // offset, length

    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let size = le_u32(bytes, i + 4).ok_or(CrystalError::UnrecognisedFormat)? as usize;
        let body = i + 8;

        if id == b"fmt " {
            if body + 16 > bytes.len() {
                return Err(CrystalError::UnrecognisedFormat);
            }
            let audio_format = le_u16(bytes, body).ok_or(CrystalError::UnrecognisedFormat)?;
            let channels = le_u16(bytes, body + 2).ok_or(CrystalError::UnrecognisedFormat)?;
            let sample_rate = le_u32(bytes, body + 4).ok_or(CrystalError::UnrecognisedFormat)?;
            let bits = le_u16(bytes, body + 14).ok_or(CrystalError::UnrecognisedFormat)?;
            fmt = Some((audio_format, channels, sample_rate, bits));
        } else if id == b"data" {
            data = Some((body, size));
        }

        let padded = size + (size % 2);
        i = body
            .checked_add(padded)
            .ok_or(CrystalError::UnrecognisedFormat)?;
    }

    let (audio_format, channels, sample_rate, bits) =
        fmt.ok_or(CrystalError::UnrecognisedFormat)?;
    if audio_format != 1 || channels == 0 || (bits != 8 && bits != 16) {
        return Err(CrystalError::UnrecognisedFormat);
    }
    let (offset, len) = data.ok_or(CrystalError::UnrecognisedFormat)?;

    let bytes_per_sample = usize::from(bits / 8);
    let frame_size = bytes_per_sample * usize::from(channels);
    if frame_size == 0 {
        return Err(CrystalError::UnrecognisedFormat);
    }
    let available = bytes.len().saturating_sub(offset);
    if available < len {
        return Err(CrystalError::TruncatedMedia {
            expected: len,
            got: available,
        });
    }
    let region = &bytes[offset..offset + len];
    let n_frames = region.len() / frame_size;

    let mut samples = Vec::with_capacity(n_frames);
    for f in 0..n_frames {
        let mut sum = 0.0;
        for c in 0..usize::from(channels) {
            let so = f * frame_size + c * bytes_per_sample;
            let v = if bits == 8 {
                f64::from(region[so]) - 128.0
            } else {
                f64::from(i16::from_le_bytes([region[so], region[so + 1]]))
            };
            sum += v;
        }
        samples.push(sum / f64::from(channels));
    }

    Ok(AudioSamples {
        samples,
        sample_rate: f64::from(sample_rate),
        channels,
    })
}
