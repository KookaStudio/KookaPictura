//! The flattening conversions: `Image > Mode > Indexed Color` and `Bitmap`.
//! Both modes hold no layers in CS6, so the visible stack is flattened first
//! (hidden layers are discarded) and the result is one history state.

use pictura_codec::{quantize, PaletteOptions};
use pictura_core::{
    BitDepth, ColorMode, Document, PixelBuffer, Samples, SourceChannels, SourcePlanes,
};
use pictura_ops::OpsError;

use super::{can_convert_mode, planes};

/// How `Image > Mode > Bitmap` reduces gray to black and white.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitmapMethod {
    /// Above mid-gray (128) is white, at or below is black.
    Threshold,
    /// An 8x8 ordered (Bayer) dot pattern.
    ///
    /// ponytail: CS6's Pattern Dither geometry is unpublished; Bayer is an
    /// approximation.
    Pattern,
    /// Floyd-Steinberg error diffusion from the upper left.
    Diffusion,
}

fn refuse(doc: &Document, target: ColorMode) -> OpsError {
    OpsError::Unsupported(format!(
        "cannot convert {:?} to {target:?}",
        super::document_color_mode(doc)
    ))
}

fn flatten(doc: &mut Document) -> Result<(), OpsError> {
    crate::flatten(doc).map_err(|e| OpsError::InvalidParams(format!("{e:?}")))?;
    planes::drop_source_state(doc);
    doc.source_planes = None;
    doc.source_depth = None;
    Ok(())
}

/// `Image > Mode > Indexed Color`: flatten, build a palette of at most
/// `options.colors` entries with `options.reduction`, map every pixel to it
/// (dithered per `options.dither` / `options.amount`), and record the result as
/// an Indexed source (palette + index planes) that a save writes as header
/// mode Indexed. The working RGB becomes the palette expansion.
///
/// ponytail: Transparency/Matte and Forced colors are not offered (the
/// flattened image is opaque and the writer has no transparent-index
/// resource); System, Uniform, Master, Custom, and Previous palettes are absent.
pub fn convert_to_indexed(doc: &mut Document, options: PaletteOptions) -> Result<(), OpsError> {
    if !can_convert_mode(doc, ColorMode::Indexed) {
        return Err(refuse(doc, ColorMode::Indexed));
    }
    flatten(doc)?;
    planes::expand_gray(doc);
    let (w, h) = (doc.width, doc.height);
    let n = w as usize * h as usize;
    let layer = &mut doc.layers[0];
    let pos = planes::color_positions(layer, 3)
        .ok_or_else(|| OpsError::InvalidParams("flattened layer has no RGB planes".into()))?;
    let mut rgba = vec![255u8; n * 4];
    for (c, &i) in pos.iter().enumerate() {
        for (p, &v) in layer.channels[i].data.iter().enumerate().take(n) {
            rgba[p * 4 + c] = v;
        }
    }
    let indexed = quantize(
        &rgba,
        w,
        h,
        PaletteOptions {
            transparency: false,
            matte: [255, 255, 255],
            ..options
        },
    );
    let mut palette = [0u8; 768];
    for (i, entry) in indexed.palette.iter().enumerate().take(256) {
        palette[i] = entry[0];
        palette[256 + i] = entry[1];
        palette[512 + i] = entry[2];
    }
    let rgb = pictura_codec::indexed_to_rgb(&indexed.indices, &palette);
    for (c, &i) in pos.iter().enumerate() {
        layer.channels[i].data = rgb[c * n..(c + 1) * n].to_vec().into();
    }
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Eight,
        layer.rect,
        vec![(0, Samples::U8(indexed.indices.clone().into()))],
    ));
    doc.composite = PixelBuffer {
        width: w,
        height: h,
        channels: 3,
        data: rgb.into(),
    };
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Eight,
        width: w,
        height: h,
        samples: Samples::U8(indexed.indices.into()),
    });
    doc.source_mode = Some(ColorMode::Indexed);
    doc.source_palette = Some(palette);
    doc.document_icc = None;
    Ok(())
}

/// Whether the visible image has at most 256 distinct colors, so the Indexed
/// Color dialog can offer the Exact palette.
pub fn indexed_exact_available(doc: &Document) -> bool {
    let plane = doc.width as usize * doc.height as usize;
    let data = &doc.composite.data;
    let colors = doc.composite.channels.min(3) as usize;
    if plane == 0 || data.len() < colors * plane {
        return false;
    }
    let mut seen = std::collections::HashSet::new();
    for i in 0..plane {
        let mut key = [0u8; 3];
        for (c, slot) in key.iter_mut().enumerate() {
            *slot = data[(c % colors) * plane + i];
        }
        if seen.insert(key) && seen.len() > 256 {
            return false;
        }
    }
    true
}

const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Reduce a gray plane to white (`true`) / black (`false`) per `method`.
pub(super) fn bitmap_whites(gray: &[u8], width: usize, method: BitmapMethod) -> Vec<bool> {
    match method {
        BitmapMethod::Threshold => gray.iter().map(|&v| v > 128).collect(),
        BitmapMethod::Pattern => gray
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let t = BAYER8[(i / width) % 8][(i % width) % 8] as u32 * 4 + 2;
                v as u32 > t
            })
            .collect(),
        BitmapMethod::Diffusion => {
            let height = gray.len() / width.max(1);
            let mut work: Vec<f32> = gray.iter().map(|&v| v as f32).collect();
            let mut out = vec![false; gray.len()];
            for y in 0..height {
                for x in 0..width {
                    let i = y * width + x;
                    let white = work[i] > 128.0;
                    out[i] = white;
                    let err = work[i] - if white { 255.0 } else { 0.0 };
                    for (dx, dy, wt) in [(1i64, 0i64, 7.0), (-1, 1, 3.0), (0, 1, 5.0), (1, 1, 1.0)]
                    {
                        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                        if nx >= 0 && nx < width as i64 && ny < height as i64 {
                            work[ny as usize * width + nx as usize] += err * wt / 16.0;
                        }
                    }
                }
            }
            out
        }
    }
}

/// `Image > Mode > Bitmap` from 8-bit Grayscale: flatten, reduce to black and
/// white per `method`, and record a flat depth-1 Bitmap source (packed rows,
/// MSB first, a set bit black) that a save writes as header mode Bitmap. The
/// document becomes layerless with no extra channels, as a Bitmap file reads.
///
/// ponytail: Output resolution is kept (no resample) and Halftone Screen /
/// Custom Pattern are absent; a Bitmap document is edited as RGB, and any edit
/// saves the working RGB instead.
pub fn convert_to_bitmap(doc: &mut Document, method: BitmapMethod) -> Result<(), OpsError> {
    if !can_convert_mode(doc, ColorMode::Bitmap) {
        return Err(refuse(doc, ColorMode::Bitmap));
    }
    flatten(doc)?;
    let (w, h) = (doc.width as usize, doc.height as usize);
    let layer = &doc.layers[0];
    let gray: Vec<u8> = match planes::color_positions(layer, 3) {
        Some(pos) => {
            let (r, g, b) = (
                &layer.channels[pos[0]].data,
                &layer.channels[pos[1]].data,
                &layer.channels[pos[2]].data,
            );
            (0..w * h)
                .map(|i| planes::luma_u8(r[i], g[i], b[i]))
                .collect()
        }
        None => planes::color_positions(layer, 1)
            .map(|pos| layer.channels[pos[0]].data.to_vec())
            .ok_or_else(|| OpsError::InvalidParams("flattened layer has no gray plane".into()))?,
    };
    let whites = bitmap_whites(&gray, w, method);
    let row = w.div_ceil(8);
    let mut packed = vec![0u8; row * h];
    for (i, &white) in whites.iter().enumerate() {
        if !white {
            packed[(i / w) * row + (i % w) / 8] |= 0x80 >> (i % w % 8);
        }
    }
    doc.layers = pictura_core::LayerStack::default();
    doc.channels.clear();
    doc.mode = ColorMode::Rgb;
    doc.composite = PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: 3,
        data: pictura_codec::bitmap_rows_to_rgb(&packed, w, h).into(),
    };
    doc.source_mode = Some(ColorMode::Bitmap);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::One,
        width: doc.width,
        height: doc.height,
        samples: Samples::U8(packed.into()),
    });
    doc.document_icc = None;
    Ok(())
}
