//! Bitmap conversion and legacy Exact eligibility for GIF save options.

use pictura_codec::{document_resolution, PatternPixels};
use pictura_core::{
    BitDepth, ColorMode, Document, PixelBuffer, PsdRect, Samples, SourcePlanes,
};
use pictura_ops::OpsError;

use super::{can_convert_mode, planes};

/// How `Image > Mode > Bitmap` reduces gray to black and white.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BitmapMethod<'a> {
    /// Above mid-gray (128) is white, at or below is black.
    Threshold,
    /// An 8x8 ordered (Bayer) dot pattern.
    ///
    /// ponytail: CS6's Pattern Dither geometry is unpublished; Bayer is an
    /// approximation.
    Pattern,
    /// Floyd-Steinberg error diffusion from the upper left.
    Diffusion,
    /// A rotated periodic screen at a physical frequency.
    HalftoneScreen(HalftoneScreen),
    /// A document-origin tile. A missing pattern is a refused conversion.
    CustomPattern(Option<&'a PatternPixels>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HalftoneScreen {
    /// Lines per inch, or lines per centimeter when `per_cm` is set.
    pub frequency: f64,
    pub per_cm: bool,
    pub angle: f64,
    pub shape: HalftoneShape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HalftoneShape {
    Round,
    Diamond,
    Ellipse,
    Line,
    Square,
    Cross,
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
    doc.source
        .replace_planes(None)
        .expect("valid source planes");
    doc.source
        .set_native(None, doc.source.planes().clone())
        .expect("valid source depth");
    Ok(())
}

/// Whether the visible image has at most 256 distinct colors, so the Indexed
/// Color dialog can offer the Exact palette.
pub fn indexed_exact_available(doc: &Document) -> bool {
    if !doc.layers.is_empty() && !crate::DocumentComposite::is_stored(doc) {
        // No stored composite (#1216): count the colours band by band.
        let mut seen = std::collections::HashSet::new();
        let whole = PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        };
        crate::stream_composite(doc, whole, 256, |_, band| {
            if seen.len() > 256 {
                return;
            }
            let n = band.width as usize * band.height as usize;
            let mut planes = [vec![0u8; n], vec![0u8; n], vec![0u8; n]];
            for (c, plane) in planes.iter_mut().enumerate() {
                band.data.read_into(c * n, plane);
            }
            let [r, g, b] = &planes;
            for ((&r, &g), &b) in r.iter().zip(g).zip(b) {
                if seen.insert([r, g, b]) && seen.len() > 256 {
                    return;
                }
            }
        });
        return seen.len() <= 256;
    }
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

const SCREEN_SIDE: usize = 128;

fn screen_thresholds(shape: HalftoneShape) -> [u8; SCREEN_SIDE * SCREEN_SIDE] {
    let mut scores: Vec<_> = (0..SCREEN_SIDE * SCREEN_SIDE)
        .map(|index| {
            let centered = |sample: usize| (2 * sample + 1).abs_diff(SCREEN_SIDE);
            let x = centered(index % SCREEN_SIDE);
            let y = centered(index / SCREEN_SIDE);
            let score = match shape {
                HalftoneShape::Round => x * x + y * y,
                HalftoneShape::Diamond => x + y,
                HalftoneShape::Ellipse => x * x + 3 * y * y,
                HalftoneShape::Line => y,
                HalftoneShape::Square => x.max(y),
                HalftoneShape::Cross => 3 * x.min(y) + x.max(y),
            };
            (score, index)
        })
        .collect();
    scores.sort_unstable_by_key(|&(score, _)| score);
    let mut thresholds = [0; SCREEN_SIDE * SCREEN_SIDE];
    let mut start = 0;
    while start < scores.len() {
        let score = scores[start].0;
        let end = start + scores[start..].partition_point(|&(other, _)| other == score);
        // Equal spots share an ink threshold. The cell's center receives ink first.
        let threshold = 255 - ((start + end) * 128 / scores.len()) as u8;
        for &(_, index) in &scores[start..end] {
            thresholds[index] = threshold;
        }
        start = end;
    }
    thresholds
}

fn screen_whites(gray: &[u8], width: usize, ppi: f64, options: HalftoneScreen) -> Vec<bool> {
    let lpi = options.frequency * if options.per_cm { 2.54 } else { 1.0 };
    let cells_per_pixel = lpi / ppi;
    let (sin, cos) = options.angle.to_radians().sin_cos();
    let thresholds = screen_thresholds(options.shape);
    let sample =
        |coord: f64| ((coord.rem_euclid(1.0) * SCREEN_SIDE as f64) as usize).min(SCREEN_SIDE - 1);
    gray.iter()
        .enumerate()
        .map(|(i, &value)| {
            let x = (i % width) as f64 + 0.5;
            let y = (i / width) as f64 + 0.5;
            let u = sample((x * cos + y * sin) * cells_per_pixel);
            let v = sample((-x * sin + y * cos) * cells_per_pixel);
            screen_white(value, thresholds[v * SCREEN_SIDE + u])
        })
        .collect()
}

fn screen_white(value: u8, threshold: u8) -> bool {
    value == 255 || value > threshold
}

fn custom_pattern_whites(gray: &[u8], width: usize, pattern: &PatternPixels) -> Vec<bool> {
    let (tile_w, tile_h) = (pattern.width as usize, pattern.height as usize);
    gray.iter()
        .enumerate()
        .map(|(i, &value)| {
            let x = i % width % tile_w;
            let y = (i / width) % tile_h;
            let offset = (y * tile_w + x) * 4;
            let rgba = &pattern.rgba[offset..offset + 4];
            let luma = u32::from(planes::luma_u8(rgba[0], rgba[1], rgba[2]));
            let alpha = u32::from(rgba[3]);
            let threshold = ((luma * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
            screen_white(value, threshold)
        })
        .collect()
}

/// Reduce a gray plane to white (`true`) / black (`false`) per validated `method`.
pub(super) fn bitmap_whites(
    gray: &[u8],
    width: usize,
    method: BitmapMethod<'_>,
    ppi: f64,
) -> Vec<bool> {
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
        BitmapMethod::HalftoneScreen(options) => screen_whites(gray, width, ppi, options),
        BitmapMethod::CustomPattern(pattern) => {
            custom_pattern_whites(gray, width, pattern.expect("validated pattern"))
        }
    }
}

/// `Image > Mode > Bitmap` from 8-bit Grayscale: flatten, reduce to black and
/// white per `method`, and record a flat depth-1 Bitmap source (packed rows,
/// MSB first, a set bit black) that a save writes as header mode Bitmap. The
/// document becomes layerless with no extra channels, as a Bitmap file reads.
///
/// ponytail: output resolution is kept (no resample). Screen spot geometry
/// models tones and shapes without an Adobe pixel oracle. A Bitmap document is
/// edited as RGB, and any edit saves the working RGB instead.
pub fn convert_to_bitmap(doc: &mut Document, method: BitmapMethod<'_>) -> Result<(), OpsError> {
    if !can_convert_mode(doc, ColorMode::Bitmap) {
        return Err(refuse(doc, ColorMode::Bitmap));
    }
    let ppi = match method {
        BitmapMethod::HalftoneScreen(options) => {
            let frequency_range = if options.per_cm {
                0.4..=400.0
            } else {
                1.0..=999.999
            };
            if !frequency_range.contains(&options.frequency)
                || !(-180.0..=180.0).contains(&options.angle)
            {
                return Err(OpsError::InvalidParams(
                    "invalid halftone screen options".into(),
                ));
            }
            document_resolution(doc).map_or(72.0, |resolution| resolution.ppi)
        }
        BitmapMethod::CustomPattern(pattern) => {
            let valid = pattern.is_some_and(|tile| {
                tile.width > 0
                    && tile.height > 0
                    && (tile.width as usize)
                        .checked_mul(tile.height as usize)
                        .and_then(|pixels| pixels.checked_mul(4))
                        == Some(tile.rgba.len())
            });
            if !valid {
                return Err(OpsError::InvalidParams(
                    "invalid or missing bitmap pattern".into(),
                ));
            }
            72.0
        }
        _ => 72.0,
    };
    flatten(doc)?;
    let (w, h) = (doc.width as usize, doc.height as usize);
    let layer = &doc.layers[0];
    let gray: Vec<u8> = match planes::color_positions(layer, 3) {
        Some(pos) => {
            let (r, g, b) = (
                &layer.channels()[pos[0]].data,
                &layer.channels()[pos[1]].data,
                &layer.channels()[pos[2]].data,
            );
            (0..w * h)
                .map(|i| planes::luma_u8(r[i], g[i], b[i]))
                .collect()
        }
        None => planes::color_positions(layer, 1)
            .map(|pos| layer.channels()[pos[0]].data.to_vec())
            .ok_or_else(|| OpsError::InvalidParams("flattened layer has no gray plane".into()))?,
    };
    let whites = bitmap_whites(&gray, w, method, ppi);
    let row = w.div_ceil(8);
    let mut packed = vec![0u8; row * h];
    for (i, &white) in whites.iter().enumerate() {
        if !white {
            packed[(i / w) * row + (i % w) / 8] |= 0x80 >> (i % w % 8);
        }
    }
    doc.layers = pictura_core::LayerStack::default();
    doc.channels.clear();
    pictura_codec::set_alpha_names(doc, &[]);
    pictura_codec::set_display_info(doc, &[]);
    doc.mode = ColorMode::Rgb;
    doc.composite
        .replace(PixelBuffer {
            width: doc.width,
            height: doc.height,
            channels: 3,
            data: pictura_codec::bitmap_rows_to_rgb(&packed, w, h).into(),
        })
        .expect("valid composite");
    doc.source
        .set_encoding(Some(ColorMode::Bitmap), None)
        .expect("valid source encoding");
    doc.source
        .replace_planes(Some(SourcePlanes {
            depth: BitDepth::One,
            width: doc.width,
            height: doc.height,
            samples: Samples::U8(packed.into()),
        }))
        .expect("valid source planes");
    doc.source.set_working_profile(None);
    Ok(())
}
