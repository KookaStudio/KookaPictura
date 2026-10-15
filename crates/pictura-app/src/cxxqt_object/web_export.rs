//! File > Save for Web & Devices: the flattened sRGB image, palette reduction,
//! and the GIF and WBMP encoders (`pictura_codec::web`). Free functions (their
//! own bridge). JPEG, PNG-8, and PNG-24 are written by the dialog through Qt.

use super::helpers_composite::{buffer_to_rgba_bytes, current_buffer};
use super::qobject::PictureView;
use cxx_qt::CxxQtType;
use ffi::WebPalette;
use pictura_codec::{ColorReduction, Dither, Indexed, PaletteOptions};
use pictura_core::{Layer, PsdRect};

#[cxx_qt::bridge]
pub mod ffi {
    /// Palette settings: `reduction` 0 Perceptual / 1 Selective / 2 Adaptive /
    /// 3 Restrictive (Web) / 4 Black and White / 5 Grayscale; `colors` 2–256;
    /// `dither` 0 None / 1 Diffusion / 2 Pattern / 3 Noise and its `amount`
    /// 0–100 %; `transparency`; the `matte` (`0xAARRGGBB`).
    #[namespace = "pictura"]
    struct WebPalette {
        reduction: i32,
        colors: i32,
        dither: i32,
        amount: i32,
        transparency: bool,
        matte: u32,
    }

    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The flattened document as sRGB RGBA8888, `document_width` x `document_height`; empty without a document.
        fn web_flattened(view: &PictureView) -> Vec<u8>;

        /// Top-level layer `index` rendered alone on transparency and cropped to its bounds: a little-endian `u32` width, a `u32` height, then RGBA8888; empty without that layer.
        fn layer_flattened(view: &PictureView, index: i32) -> Vec<u8>;

        /// Reduce RGBA8888 `width` x `height` to a palette: `[n_hi, n_lo]`, then `n` RGBA entries (alpha 0 marks the transparent entry), then one index per pixel.
        fn web_quantize(rgba: &[u8], width: i32, height: i32, palette: &WebPalette) -> Vec<u8>;

        /// Encode a `web_quantize` result as a GIF89a file, `interlaced` or not.
        fn web_encode_gif(indexed: &[u8], width: i32, height: i32, interlaced: bool) -> Vec<u8>;

        /// Encode RGBA8888 as a black-and-white WBMP with `dither` (as `WebPalette::dither`) at `amount` %.
        fn web_encode_wbmp(
            rgba: &[u8],
            width: i32,
            height: i32,
            dither: i32,
            amount: i32,
        ) -> Vec<u8>;
    }
}

fn dither_of(code: i32) -> Dither {
    match code {
        1 => Dither::Diffusion,
        2 => Dither::Pattern,
        3 => Dither::Noise,
        _ => Dither::None,
    }
}

fn size(rgba: &[u8], width: i32, height: i32) -> Option<(u32, u32)> {
    let (w, h) = (u32::try_from(width).ok()?, u32::try_from(height).ok()?);
    (w > 0 && h > 0 && rgba.len() >= (w as usize) * (h as usize) * 4).then_some((w, h))
}

fn web_flattened(view: &PictureView) -> Vec<u8> {
    let rust = view.rust();
    let Some(doc) = rust.doc.as_ref() else {
        return Vec::new();
    };
    let buffer = current_buffer(doc, rust.gpu_compute);
    buffer_to_rgba_bytes(&pictura_codec::buffer_to_srgb(doc, &buffer))
}

/// The bounds a layer's subtree covers: its own rect for a leaf, the union of
/// its children for a group (falling back to the group rect when the children
/// have no area).
fn subtree_bounds(layer: &Layer) -> PsdRect {
    if !layer.is_group {
        return layer.rect;
    }
    let mut union: Option<PsdRect> = None;
    for child in &layer.children {
        let rect = subtree_bounds(child);
        if rect.right <= rect.left || rect.bottom <= rect.top {
            continue;
        }
        union = Some(match union {
            None => rect,
            Some(u) => PsdRect {
                left: u.left.min(rect.left),
                top: u.top.min(rect.top),
                right: u.right.max(rect.right),
                bottom: u.bottom.max(rect.bottom),
            },
        });
    }
    union.unwrap_or(layer.rect)
}

/// Crop interleaved RGBA `w` x `h` to `rect` (already clamped, non-empty).
fn crop_rgba(rgba: &[u8], w: u32, rect: (u32, u32, u32, u32)) -> Vec<u8> {
    let (x0, y0, cw, ch) = rect;
    let stride = cw as usize * 4;
    let mut out = vec![0u8; stride * ch as usize];
    for y in 0..ch as usize {
        let src = ((y0 as usize + y) * w as usize + x0 as usize) * 4;
        out[y * stride..y * stride + stride].copy_from_slice(&rgba[src..src + stride]);
    }
    out
}

/// The layer's bounds clamped to the document, or the whole document when the
/// subtree has no area (an adjustment layer, an empty group).
fn layer_bounds(layer: &Layer, doc_w: u32, doc_h: u32) -> (u32, u32, u32, u32) {
    let rect = subtree_bounds(layer);
    let x0 = rect.left.max(0) as u32;
    let y0 = rect.top.max(0) as u32;
    let x1 = rect.right.clamp(0, doc_w as i32) as u32;
    let y1 = rect.bottom.clamp(0, doc_h as i32) as u32;
    if x1 <= x0 || y1 <= y0 {
        (0, 0, doc_w, doc_h)
    } else {
        (x0, y0, x1 - x0, y1 - y0)
    }
}

fn layer_flattened(view: &PictureView, index: i32) -> Vec<u8> {
    let rust = view.rust();
    let Some(doc) = rust.doc.as_ref() else {
        return Vec::new();
    };
    layer_flattened_buffer(doc, index as usize, rust.gpu_compute)
}

/// The testable core of [`layer_flattened`]: composite top-level layer `index`
/// alone and crop to its bounds, prefixed with the little-endian `u32` width
/// and height (empty without that layer).
fn layer_flattened_buffer(
    doc: &pictura_core::Document,
    index: usize,
    gpu_compute: bool,
) -> Vec<u8> {
    let Some(layer) = doc.layers.get(index) else {
        return Vec::new();
    };
    let mut scratch = doc.clone();
    scratch.layers = vec![layer.clone()];
    let buffer = pictura_render::composite_active(&scratch, gpu_compute).0;
    let srgb = pictura_codec::buffer_to_srgb(&scratch, &buffer);
    let rgba = buffer_to_rgba_bytes(&srgb);
    let (x0, y0, cw, ch) = layer_bounds(layer, doc.width, doc.height);
    let cropped = crop_rgba(&rgba, srgb.width, (x0, y0, cw, ch));
    let mut out = Vec::with_capacity(8 + cropped.len());
    out.extend_from_slice(&cw.to_le_bytes());
    out.extend_from_slice(&ch.to_le_bytes());
    out.extend_from_slice(&cropped);
    out
}

fn web_quantize(rgba: &[u8], width: i32, height: i32, palette: &WebPalette) -> Vec<u8> {
    let Some((w, h)) = size(rgba, width, height) else {
        return Vec::new();
    };
    let reduction = match palette.reduction {
        0 => ColorReduction::Perceptual,
        2 => ColorReduction::Adaptive,
        3 => ColorReduction::Restrictive,
        4 => ColorReduction::BlackAndWhite,
        5 => ColorReduction::Grayscale,
        _ => ColorReduction::Selective,
    };
    let options = PaletteOptions {
        reduction,
        colors: palette.colors.clamp(2, 256) as u16,
        dither: dither_of(palette.dither),
        amount: palette.amount.clamp(0, 100) as u8,
        transparency: palette.transparency,
        matte: [
            (palette.matte >> 16) as u8,
            (palette.matte >> 8) as u8,
            palette.matte as u8,
        ],
    };
    let indexed = pictura_codec::quantize(rgba, w, h, options);
    let n = indexed.palette.len() as u16;
    let mut out = n.to_be_bytes().to_vec();
    out.extend(indexed.palette.iter().flatten());
    out.extend_from_slice(&indexed.indices);
    out
}

fn web_encode_gif(indexed: &[u8], width: i32, height: i32, interlaced: bool) -> Vec<u8> {
    let (Ok(w), Ok(h)) = (u32::try_from(width), u32::try_from(height)) else {
        return Vec::new();
    };
    let Some(n) = indexed
        .get(..2)
        .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
    else {
        return Vec::new();
    };
    let pixels = (w as usize) * (h as usize);
    if n == 0 || indexed.len() != 2 + n * 4 + pixels {
        return Vec::new();
    }
    let palette = indexed[2..2 + n * 4].as_chunks::<4>().0.to_vec();
    let image = Indexed {
        width: w,
        height: h,
        palette,
        indices: indexed[2 + n * 4..].to_vec(),
    };
    pictura_codec::encode_gif(&image, interlaced)
}

fn web_encode_wbmp(rgba: &[u8], width: i32, height: i32, dither: i32, amount: i32) -> Vec<u8> {
    let Some((w, h)) = size(rgba, width, height) else {
        return Vec::new();
    };
    pictura_codec::encode_wbmp(rgba, w, h, dither_of(dither), amount.clamp(0, 100) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{BitDepth, ColorMode, Document};

    fn at(mut layer: Layer, left: i32, top: i32) -> Layer {
        let (w, h) = (layer.rect.width(), layer.rect.height());
        layer.rect = PsdRect {
            left,
            top,
            right: left + w,
            bottom: top + h,
        };
        layer
    }

    fn px(rgba: &[u8], i: usize) -> [u8; 4] {
        [
            rgba[i * 4],
            rgba[i * 4 + 1],
            rgba[i * 4 + 2],
            rgba[i * 4 + 3],
        ]
    }

    fn dims(packed: &[u8]) -> (u32, u32) {
        (
            u32::from_le_bytes([packed[0], packed[1], packed[2], packed[3]]),
            u32::from_le_bytes([packed[4], packed[5], packed[6], packed[7]]),
        )
    }

    #[test]
    fn layer_flattened_crops_a_pixel_layer_to_its_rect() {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = vec![
            pixel_layer("base", 4, 4, (10, 20, 30)),
            at(pixel_layer("top", 2, 2, (200, 100, 50)), 1, 1),
        ];
        let rgba = layer_flattened_buffer(&doc, 1, false);
        // 2 x 2, the top layer's colour, fully opaque.
        assert_eq!(dims(&rgba), (2, 2));
        assert_eq!(rgba.len(), 8 + 2 * 2 * 4);
        let data = &rgba[8..];
        assert_eq!(px(data, 0), [200, 100, 50, 255]);
        assert_eq!(px(data, 3), [200, 100, 50, 255]);
    }

    #[test]
    fn layer_flattened_crops_a_group_to_the_child_union() {
        let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        let mut group = pixel_layer("group", 0, 0, (0, 0, 0));
        group.is_group = true;
        group.channels.clear();
        group.children = vec![
            at(pixel_layer("a", 2, 2, (1, 2, 3)), 1, 1),
            at(pixel_layer("b", 1, 1, (4, 5, 6)), 5, 5),
        ];
        doc.layers = vec![group];
        let rgba = layer_flattened_buffer(&doc, 0, false);
        // The union of (1,1)-(3,3) and (5,5)-(6,6) is (1,1)-(6,6): 5 x 5.
        assert_eq!(dims(&rgba), (5, 5));
    }

    #[test]
    fn layer_flattened_is_empty_without_that_layer() {
        let doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        assert!(layer_flattened_buffer(&doc, 0, false).is_empty());
    }
}
