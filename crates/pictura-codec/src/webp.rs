//! Native WebP (VP8/VP8L/VP8X) decode and lossless encode.
//!
//! Qt ships its WebP reader and writer in the optional `qt6-imageformats`
//! package, so a Qt-only path silently fails to open or save WebP wherever that
//! plugin is absent. These functions let the import and export edges use the
//! engine instead, keeping WebP at parity with the formats Qt ships in `qtbase`.

use crate::probe::ImageBudget;
use image_webp::{ColorType, WebPDecoder, WebPEncoder};
use std::io::Cursor;

/// Decode a `RIFF....WEBP` container to tightly packed RGBA8888 and its
/// dimensions, or `None` for a malformed container or one over `budget`.
///
/// The declared dimensions are checked against `budget` before the decode
/// buffer is allocated. A container with no alpha channel is expanded to
/// RGBA8888 (alpha 255); an animation decodes its first frame only.
pub fn decode_webp(bytes: &[u8], budget: ImageBudget) -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = WebPDecoder::new(Cursor::new(bytes)).ok()?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    if width > budget.max_dimension || height > budget.max_dimension {
        return None;
    }
    let alloc = (width as u64).checked_mul(height as u64)?.checked_mul(4)?;
    if alloc > budget.max_alloc_bytes {
        return None;
    }
    decoder.set_memory_limit(alloc as usize);
    let mut raw = vec![0u8; decoder.output_buffer_size()?];
    decoder.read_image(&mut raw).ok()?;
    let rgba = if decoder.has_alpha() {
        raw
    } else {
        let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
        for pixel in raw.as_chunks::<3>().0 {
            rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
        }
        rgba
    };
    if rgba.len() != (width as usize) * (height as usize) * 4 {
        return None;
    }
    Some((rgba, width, height))
}

/// Encode tightly packed RGBA8888 as a lossless WebP (VP8L), or `None` when a
/// dimension is zero or `rgba` is not exactly `width * height * 4` bytes.
///
/// ponytail: lossless only, because no pure-Rust lossy WebP encoder exists; a
/// lossy export would need a `libwebp` FFI binding, so the ceiling is a larger
/// file than Photoshop's lossy WebP export produces.
pub fn encode_webp(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if rgba.len() != expected {
        return None;
    }
    let mut out = Vec::new();
    WebPEncoder::new(&mut out)
        .encode(rgba, width, height, ColorType::Rgba8)
        .ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> ImageBudget {
        ImageBudget::default()
    }

    /// An 8 x 8 field with a transparent corner and per-pixel colour, so a
    /// lossless round trip exercises every byte position including alpha.
    fn picture() -> (Vec<u8>, u32, u32) {
        let (w, h) = (8u32, 8u32);
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&[
                    (x * 31) as u8,
                    (y * 31) as u8,
                    ((x + y) * 15) as u8,
                    if x == 0 && y == 0 { 0 } else { 255 },
                ]);
            }
        }
        (rgba, w, h)
    }

    #[test]
    fn lossless_round_trip_is_byte_exact() {
        let (rgba, w, h) = picture();
        let webp = encode_webp(&rgba, w, h).expect("encode");
        assert_eq!(&webp[..4], b"RIFF");
        assert_eq!(&webp[8..12], b"WEBP");
        let (decoded, dw, dh) = decode_webp(&webp, budget()).expect("decode");
        assert_eq!((dw, dh), (w, h));
        assert_eq!(decoded, rgba);
    }

    #[test]
    fn a_container_without_alpha_decodes_to_rgba() {
        let (rgba, w, h) = picture();
        let rgb: Vec<u8> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|px| [px[0], px[1], px[2]])
            .collect();
        let mut opaque = Vec::new();
        WebPEncoder::new(&mut opaque)
            .encode(&rgb, w, h, ColorType::Rgb8)
            .expect("encode rgb8");
        let (decoded, dw, dh) = decode_webp(&opaque, budget()).expect("decode");
        assert_eq!((dw, dh), (w, h));
        assert_eq!(decoded.len(), (w * h * 4) as usize);
        assert!(decoded.as_chunks::<4>().0.iter().all(|px| px[3] == 255));
        for (src, out) in rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(decoded.as_chunks::<4>().0.iter())
        {
            assert_eq!(&src[..3], &out[..3]);
        }
    }

    #[test]
    fn a_non_webp_or_truncated_container_is_refused() {
        assert!(decode_webp(b"not a webp at all", budget()).is_none());
        assert!(decode_webp(b"RIFF\x00\x00\x00\x00WEBP", budget()).is_none());
        assert!(decode_webp(&[], budget()).is_none());
    }

    #[test]
    fn an_over_budget_webp_is_refused() {
        let (rgba, w, h) = picture();
        let webp = encode_webp(&rgba, w, h).expect("encode");
        let small_dimension = ImageBudget {
            max_dimension: 4,
            max_alloc_bytes: u64::MAX,
            ..ImageBudget::default()
        };
        assert!(decode_webp(&webp, small_dimension).is_none());
        let small_alloc = ImageBudget {
            max_dimension: 30_000,
            max_alloc_bytes: 4,
            ..ImageBudget::default()
        };
        assert!(decode_webp(&webp, small_alloc).is_none());
    }

    #[test]
    fn encode_refuses_a_zero_dimension_or_short_buffer() {
        assert!(encode_webp(&[], 0, 0).is_none());
        assert!(encode_webp(&[0, 0, 0], 2, 2).is_none());
        assert!(encode_webp(&[0; 16], 2, 2).is_some());
    }
}
