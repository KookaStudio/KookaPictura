//! A minimal PNG writer for the SVG raster fallback.
//!
//! Only the one shape the fallback needs: 8-bit RGBA (PNG colour type 6), one
//! `IDAT` with filter 0 per row, no interlacing, no ancillary chunks. This is
//! deliberately not a general encoder; the shipped raster-export path stays on
//! Qt. `flate2` (already a dependency) supplies the zlib stream.

use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;

/// Encode straight-alpha RGBA8888 to PNG bytes. `None` when the dimensions are
/// zero or `rgba` is not exactly `width * height * 4` bytes.
pub fn encode_rgba(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let stride = width as usize * 4;
    if rgba.len() != stride * height as usize {
        return None;
    }
    let mut out = Vec::with_capacity(rgba.len() + rgba.len() / 2 + 64);
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    let mut ihdr = [0u8; 13];
    ihdr[..4].copy_from_slice(&width.to_be_bytes());
    ihdr[4..8].copy_from_slice(&height.to_be_bytes());
    // bit depth 8, colour type 6 (RGBA), deflate, adaptive filtering, no interlace.
    ihdr[8..].copy_from_slice(&[8, 6, 0, 0, 0]);
    write_chunk(&mut out, b"IHDR", &ihdr);

    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for row in rgba.chunks_exact(stride) {
        raw.push(0); // filter type None
        raw.extend_from_slice(row);
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw).ok()?;
    let compressed = encoder.finish().ok()?;
    write_chunk(&mut out, b"IDAT", &compressed);
    write_chunk(&mut out, b"IEND", &[]);
    Some(out)
}

/// Encode straight-alpha RGBA8888 to a base64 PNG payload (the part after
/// `data:img/png;base64,`). `None` under the same conditions as [`encode_rgba`].
pub fn encode_base64(rgba: &[u8], width: u32, height: u32) -> Option<String> {
    Some(base64(&encode_rgba(rgba, width, height)?))
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// CRC-32 (IEEE 802.3), the PNG chunk checksum.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc = (crc >> 8) ^ CRC_TABLE[((crc ^ u32::from(byte)) & 0xFF) as usize];
    }
    !crc
}

const CRC_TABLE: [u32; 256] = build_crc_table();

const fn build_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with `=` padding.
fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64[(n >> 18 & 0x3F) as usize] as char);
        out.push(BASE64[(n >> 12 & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            BASE64[(n >> 6 & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::{probe_image, ImageBudget, ImageFormat};
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    fn pixel(rgba: [u8; 4]) -> Vec<u8> {
        rgba.to_vec()
    }

    #[test]
    fn a_one_pixel_png_has_the_expected_header() {
        let png = encode_rgba(&pixel([255, 0, 0, 255]), 1, 1).expect("encodes");
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        let probe = probe_image(&png, ImageBudget::default()).expect("probe");
        assert_eq!(probe.format, ImageFormat::Png);
        assert_eq!((probe.width, probe.height), (1, 1));
        assert_eq!(probe.bit_depth, 8);
    }

    #[test]
    fn the_idat_decompresses_to_filtered_rows() {
        // 2x1: filter byte 0, then the two RGBA pixels.
        let rgba = [1, 2, 3, 4, 5, 6, 7, 8];
        let png = encode_rgba(&rgba, 2, 1).expect("encodes");
        let idat = idat_data(&png);
        let mut decoded = Vec::new();
        ZlibDecoder::new(idat.as_slice())
            .read_to_end(&mut decoded)
            .expect("inflate");
        assert_eq!(decoded, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn dimensions_must_match_the_buffer() {
        assert!(encode_rgba(&[0; 4], 1, 1).is_some());
        assert!(encode_rgba(&[0; 3], 1, 1).is_none(), "too short");
        assert!(encode_rgba(&[0; 4], 2, 1).is_none(), "too long");
        assert!(encode_rgba(&[], 0, 0).is_none(), "zero size");
    }

    #[test]
    fn base64_matches_a_known_vector() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        // The PNG signature's base64, a stable cross-check.
        assert_eq!(
            base64(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
            "iVBORw0KGgo="
        );
    }

    #[test]
    fn crc32_matches_the_ieee_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    /// The bytes of the first `IDAT` chunk.
    fn idat_data(png: &[u8]) -> Vec<u8> {
        let mut at = 8;
        while at + 8 <= png.len() {
            let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
            let kind = &png[at + 4..at + 8];
            if kind == b"IDAT" {
                return png[at + 8..at + 8 + len].to_vec();
            }
            at += 12 + len;
        }
        Vec::new()
    }
}
