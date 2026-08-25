//! A minimal, hand-written GIF89a encoder — no crate, since [`crate::render`]
//! already produces palette-indexed frames and nothing else in this
//! workspace has ever needed one.
//!
//! # Why the image data is larger than it has to be
//!
//! GIF's image data is LZW-compressed, and LZW's dictionary-building
//! compression is exactly the kind of thing this workspace's own doctrine
//! warns about: subtle, and easy to get bit-level wrong in a way that only
//! shows up against an independent decoder. The first version of this
//! module tried to dodge the compression logic entirely by emitting every
//! pixel as its own root-sized literal code and never growing the code
//! size — reasoning that since no code ever *referenced* a multi-symbol
//! dictionary entry, the table never needed to grow. That reasoning was
//! wrong, and Pillow caught it immediately ("broken data stream"): an LZW
//! *decoder* adds a new dictionary entry after every code it processes,
//! unconditionally, regardless of what the encoder intended — so the
//! decoder's expected code width grows on a schedule the encoder wasn't
//! matching, and the two desynchronised.
//!
//! The fix sidesteps that schedule entirely rather than trying to replicate
//! it from memory: a `Clear` code is emitted before **every single pixel**.
//! A decoder never adds a table entry for the first code after a `Clear`
//! (there is no previous string yet to extend), so with a `Clear` in front
//! of each pixel, every code is always "the first after a `Clear`" — the
//! dictionary never grows past its initial one-code-per-colour table, and
//! the code width never has to change. This costs roughly 2x the code
//! volume of real compression, which is why canvas size and frame count
//! ([`crate::render::ANIM_SIZE`]/[`crate::render::ANIM_FRAMES`]) are kept
//! modest — but it is provably correct rather than merely plausible, which
//! matters more here than a smaller file.
//!
//! Verified against an independent decoder (Python's Pillow: correct frame
//! count, correct per-frame duration, and pixel content that round-trips)
//! after the fix, not just re-read against this file's own logic.

use std::io::{self, Write};

struct BitWriter {
    bytes: Vec<u8>,
    acc: u32,
    nbits: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self { bytes: Vec::new(), acc: 0, nbits: 0 }
    }

    fn write_code(&mut self, code: u16, size: u8) {
        self.acc |= (code as u32) << self.nbits;
        self.nbits += size as u32;
        while self.nbits >= 8 {
            self.bytes.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.bytes.push((self.acc & 0xFF) as u8);
        }
        self.bytes
    }
}

/// GIF's own sub-block framing: length-prefixed chunks of at most 255 bytes,
/// terminated by a zero-length chunk.
fn sub_blocks(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 255 + 1);
    for chunk in data.chunks(255) {
        out.push(chunk.len() as u8);
        out.extend_from_slice(chunk);
    }
    out.push(0);
    out
}

/// The literal LZW stream described in the module doc: a `Clear` before
/// every pixel, so no decoder ever adds a table entry and the code width
/// never has to grow — see there for why that's the safe choice here.
fn lzw_literal(indices: &[u8], min_code_size: u8) -> Vec<u8> {
    let clear_code: u16 = 1 << min_code_size;
    let end_code: u16 = clear_code + 1;
    let code_size = min_code_size + 1;

    let mut bw = BitWriter::new();
    for &px in indices {
        bw.write_code(clear_code, code_size);
        bw.write_code(px as u16, code_size);
    }
    bw.write_code(end_code, code_size);
    sub_blocks(&bw.finish())
}

/// Smallest code size GIF permits (minimum 2) that can address `colors`
/// palette entries.
fn min_code_size(colors: usize) -> u8 {
    let mut bits = 2u8;
    while (1usize << bits) < colors {
        bits += 1;
    }
    bits
}

/// Write an animated GIF. `frames` are palette-index buffers, each
/// `width * height` bytes, row-major. `palette` is padded to the next GIF
/// table size with black if it doesn't already fill one exactly.
pub fn write_gif(
    path: &str,
    width: u16,
    height: u16,
    palette: &[[u8; 3]],
    frames: &[Vec<u8>],
    delay_cs: u16,
) -> io::Result<()> {
    let mcs = min_code_size(palette.len());
    let table_size = 1usize << mcs;

    let mut out = Vec::new();
    out.extend_from_slice(b"GIF89a");

    // Logical Screen Descriptor.
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    let packed = 0x80 // global colour table present
        | ((mcs - 1) << 4) // colour resolution (informational)
        | (mcs - 1); // table size field: 2^(field+1) == table_size
    out.push(packed);
    out.push(0); // background colour index
    out.push(0); // pixel aspect ratio (unused)

    // Global Colour Table.
    for i in 0..table_size {
        out.extend_from_slice(palette.get(i).unwrap_or(&[0, 0, 0]));
    }

    // NETSCAPE2.0 application extension: loop forever.
    out.extend_from_slice(&[0x21, 0xFF, 0x0B]);
    out.extend_from_slice(b"NETSCAPE2.0");
    out.extend_from_slice(&[0x03, 0x01, 0x00, 0x00, 0x00]);

    for frame in frames {
        debug_assert_eq!(frame.len(), width as usize * height as usize);

        // Graphic Control Extension: per-frame delay, disposal = "do not
        // dispose" (each frame redraws the whole opaque canvas anyway).
        out.extend_from_slice(&[0x21, 0xF9, 0x04, 0x04]);
        out.extend_from_slice(&delay_cs.to_le_bytes());
        out.push(0); // transparent colour index (unused, no transparency)
        out.push(0); // block terminator

        // Image Descriptor: full-canvas, no local colour table.
        out.push(0x2C);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.push(0x00);

        // Image Data.
        out.push(mcs);
        out.extend_from_slice(&lzw_literal(frame, mcs));
    }

    out.push(0x3B); // trailer

    std::fs::File::create(path)?.write_all(&out)
}
