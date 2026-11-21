//! `OriginalRawFileData`: the source raw file in the DNG specification's
//! fork layout, as ExifTool's `ProcessOriginalRaw` decodes it.
//!
//! Big-endian throughout: the data fork's length, `n + 1` offsets relative to
//! the fork's start, then `n` zlib streams of 64 KiB input each; then a zero
//! length for the (Mac OS) resource fork and zero file type and creator words.

use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use rayon::prelude::*;

pub(super) const BLOCK: usize = 65536;

pub(super) fn original_raw_data(bytes: &[u8]) -> Vec<u8> {
    let blocks: Vec<Vec<u8>> = bytes
        .par_chunks(BLOCK)
        .map(|block| {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            // Writing to a Vec cannot fail.
            encoder.write_all(block).expect("in-memory zlib");
            encoder.finish().expect("in-memory zlib")
        })
        .collect();
    let mut out = Vec::new();
    out.extend((bytes.len() as u32).to_be_bytes());
    if !bytes.is_empty() {
        let mut at = 4 + 4 * (blocks.len() as u32 + 1);
        out.extend(at.to_be_bytes());
        for block in &blocks {
            at += block.len() as u32;
            out.extend(at.to_be_bytes());
        }
        blocks.iter().for_each(|block| out.extend(block));
    }
    out.extend([0; 12]);
    out
}

/// The data fork of `OriginalRawFileData`, inflated; `None` when the layout
/// or a zlib stream is broken.
pub(super) fn inflate_original_raw(data: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    let be = |at: usize| -> Option<usize> {
        Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?) as usize)
    };
    let len = be(0)?;
    let mut out = Vec::with_capacity(len.min(data.len().saturating_mul(16)));
    for i in 0..len.div_ceil(BLOCK) {
        let (from, to) = (be(4 + 4 * i)?, be(8 + 4 * i)?);
        let block = data.get(from..to)?;
        flate2::read::ZlibDecoder::new(block)
            .take(BLOCK as u64)
            .read_to_end(&mut out)
            .ok()?;
    }
    (out.len() == len).then_some(out)
}
