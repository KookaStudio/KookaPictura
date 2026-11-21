//! DNG output (#402): a mosaic or linear image, colour tags, a thumbnail, an
//! optional JPEG preview, a `crs:` XMP packet and the original raw file, in
//! one little-endian TIFF built in memory.
//!
//! Layout: IFD0 is the thumbnail and carries the DNG-wide tags; its SubIFDs
//! are the full-resolution image and the JPEG preview. The reader's
//! `raw_ifd`, `raw_xmp` and `sniff_raw_prefix` read exactly this shape.

use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use super::{
    CfaPattern, DngColor, DngCrop, ANALOG_BALANCE, AS_SHOT_NEUTRAL, BASELINE_EXPOSURE,
    BITS_PER_SAMPLE, BLACK_LEVEL, CALIBRATION_ILLUMINANT, CAMERA_CALIBRATION, CFA_LAYOUT,
    CFA_PATTERN, CFA_PLANE_COLOR, CFA_REPEAT_PATTERN_DIM, COLOR_MATRIX, COMPRESSION,
    DEFAULT_CROP_ORIGIN, DEFAULT_CROP_SIZE, DNG_VERSION, FORWARD_MATRIX, IMAGE_LENGTH, IMAGE_WIDTH,
    MAKE, NEW_SUBFILE_TYPE, ORIENTATION, PHOTOMETRIC, PHOTOMETRIC_CFA, PHOTOMETRIC_LINEAR_RAW,
    ROWS_PER_STRIP, SAMPLES_PER_PIXEL, STRIP_BYTE_COUNTS, STRIP_OFFSETS, SUB_IFDS,
    TILE_BYTE_COUNTS, TILE_LENGTH, TILE_OFFSETS, TILE_WIDTH, UNIQUE_CAMERA_MODEL, WHITE_LEVEL,
};
use crate::error::PsdError;
use tiff_out::{rational, srational, Value};

mod ljpeg_encode;
mod original;
#[cfg(test)]
mod tests;
mod tiff_out;

const MODEL: u16 = 272;
const PLANAR_CONFIGURATION: u16 = 284;
const SOFTWARE: u16 = 305;
const XMP: u16 = 700;
const DNG_BACKWARD_VERSION: u16 = 50707;
const DEFAULT_SCALE: u16 = 50718;
const ORIGINAL_RAW_FILE_NAME: u16 = 50827;
const PREVIEW_COLOR_SPACE: u16 = 50970;
const ORIGINAL_RAW_FILE_DATA: u16 = 50828;

const TILE: usize = 256;
const THUMBNAIL_EDGE: usize = 256;
const PREVIEW_QUALITY: u8 = 90;
const PHOTOMETRIC_RGB: u16 = 2;
const PHOTOMETRIC_YCBCR: u16 = 6;
const COMPRESSION_NONE: u16 = 1;
const COMPRESSION_JPEG: u16 = 7;

/// ACR's DNG compatibility levels, oldest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DngCompatibility {
    CameraRaw2_4,
    CameraRaw4_1,
    CameraRaw4_6,
    CameraRaw5_4,
    #[default]
    CameraRaw6_6,
}

impl DngCompatibility {
    /// The level at `index` in ACR's menu order (2.4 first); the default
    /// for an index out of range.
    pub fn from_index(index: i32) -> Self {
        match index {
            0 => Self::CameraRaw2_4,
            1 => Self::CameraRaw4_1,
            2 => Self::CameraRaw4_6,
            3 => Self::CameraRaw5_4,
            _ => Self::CameraRaw6_6,
        }
    }

    /// `DNGVersion` for the level. `ponytail:` an approximate mapping of
    /// ACR's levels; Adobe does not publish it.
    pub fn version(self) -> [u8; 4] {
        match self {
            Self::CameraRaw2_4 => [1, 1, 0, 0],
            Self::CameraRaw4_1 | Self::CameraRaw4_6 => [1, 2, 0, 0],
            Self::CameraRaw5_4 => [1, 3, 0, 0],
            Self::CameraRaw6_6 => [1, 4, 0, 0],
        }
    }
}

/// The full-resolution image: a CFA plane or demosaiced camera RGB, both
/// 0–65535 with black at 0.
#[derive(Debug, Clone, Copy)]
pub enum DngPixels<'a> {
    Mosaic {
        samples: &'a [u16],
        cfa: &'a CfaPattern,
    },
    /// `width × height × 3`, interleaved.
    Linear { rgb: &'a [u16] },
}

/// 8-bit sRGB pixels, `width × height × 3`.
#[derive(Debug, Clone, Copy)]
pub struct DngRgb8<'a> {
    pub width: usize,
    pub height: usize,
    pub rgb: &'a [u8],
}

/// Everything `write_dng` writes.
#[derive(Debug, Clone, Copy)]
pub struct DngWrite<'a> {
    pub width: usize,
    pub height: usize,
    pub pixels: DngPixels<'a>,
    pub color: &'a DngColor,
    pub crop: DngCrop,
    pub orientation: u16,
    pub make: Option<&'a str>,
    pub model: Option<&'a str>,
    /// The developed look; IFD0's thumbnail is scaled from it.
    pub look: DngRgb8<'a>,
    /// Written as a JPEG preview IFD at its own size.
    pub preview: Option<DngRgb8<'a>>,
    pub xmp: Option<&'a [u8]>,
    /// The original raw file's name and bytes.
    pub original: Option<(&'a str, &'a [u8])>,
    pub compatibility: DngCompatibility,
}

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("DNG output: {what}"))
}

fn check_rgb8(image: &DngRgb8, what: &str) -> Result<(), PsdError> {
    if image.width == 0 || image.height == 0 || image.rgb.len() != image.width * image.height * 3 {
        return Err(invalid(&format!("{what} pixels do not match its size")));
    }
    Ok(())
}

fn check(request: &DngWrite) -> Result<(), PsdError> {
    let (w, h) = (request.width, request.height);
    if w == 0 || h == 0 || w > u32::MAX as usize || h > u32::MAX as usize {
        return Err(invalid("the image is empty or too large"));
    }
    let ok = match request.pixels {
        DngPixels::Mosaic { samples, cfa } => {
            samples.len() == w * h
                && cfa.width * cfa.height == cfa.colors.len()
                && !cfa.colors.is_empty()
        }
        DngPixels::Linear { rgb } => rgb.len() == w * h * 3,
    };
    if !ok {
        return Err(invalid("the samples do not match the image size"));
    }
    check_rgb8(&request.look, "the look")?;
    if let Some(preview) = &request.preview {
        check_rgb8(preview, "the preview")?;
    }
    Ok(())
}

/// One `TILE × TILE` tile, edges padded by repeating the last row and column,
/// as interleaved samples.
fn tile_samples(
    plane: &[u16],
    width: usize,
    height: usize,
    spp: usize,
    tx: usize,
    ty: usize,
) -> Vec<u16> {
    let mut out = Vec::with_capacity(TILE * TILE * spp);
    for r in 0..TILE {
        let y = (ty * TILE + r).min(height - 1);
        let row = &plane[y * width * spp..][..width * spp];
        for c in 0..TILE {
            let x = (tx * TILE + c).min(width - 1);
            out.extend_from_slice(&row[x * spp..][..spp]);
        }
    }
    out
}

/// Lossless-JPEG tiles of the full image, in parallel. A mosaic row is coded
/// as two components of half width, so the predictor sees the same CFA
/// colour; the decoder's sample order is the same either way.
fn encode_tiles(
    request: &DngWrite,
    progress: &(dyn Fn(f32) + Sync),
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Option<Vec<Vec<u8>>> {
    let (plane, spp, frame_width, components) = match request.pixels {
        DngPixels::Mosaic { samples, .. } => (samples, 1, TILE / 2, 2),
        DngPixels::Linear { rgb } => (rgb, 3, TILE, 3),
    };
    let across = request.width.div_ceil(TILE);
    let total = across * request.height.div_ceil(TILE);
    let done = AtomicUsize::new(0);
    (0..total)
        .into_par_iter()
        .map(|i| {
            if cancelled() {
                return None;
            }
            let tile = tile_samples(
                plane,
                request.width,
                request.height,
                spp,
                i % across,
                i / across,
            );
            let coded = ljpeg_encode::encode(&tile, frame_width, TILE, components);
            progress((done.fetch_add(1, Ordering::Relaxed) + 1) as f32 / total as f32);
            Some(coded)
        })
        .collect()
}

/// A box-averaged copy whose long side is at most `edge`.
fn thumbnail(look: &DngRgb8, edge: usize) -> (usize, usize, Vec<u8>) {
    let (w, h) = (look.width, look.height);
    let long = w.max(h);
    if long <= edge {
        return (w, h, look.rgb.to_vec());
    }
    let (ow, oh) = ((w * edge / long).max(1), (h * edge / long).max(1));
    let mut out = Vec::with_capacity(ow * oh * 3);
    for oy in 0..oh {
        let (y0, y1) = (oy * h / oh, ((oy + 1) * h / oh).max(oy * h / oh + 1));
        for ox in 0..ow {
            let (x0, x1) = (ox * w / ow, ((ox + 1) * w / ow).max(ox * w / ow + 1));
            let mut sum = [0u32; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let px = &look.rgb[(y * w + x) * 3..][..3];
                    sum.iter_mut().zip(px).for_each(|(s, &v)| *s += v as u32);
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            out.extend(sum.map(|s| ((s + n / 2) / n) as u8));
        }
    }
    (ow, oh, out)
}

fn jpeg(preview: &DngRgb8) -> Result<Vec<u8>, PsdError> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, PREVIEW_QUALITY)
        .encode(
            preview.rgb,
            preview.width as u32,
            preview.height as u32,
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| invalid(&format!("JPEG preview: {error}")))?;
    Ok(out)
}

fn matrix(m: &[f64; 9]) -> Value {
    Value::SRational(m.iter().map(|&v| srational(v)).collect())
}

fn long(v: usize) -> Value {
    Value::Long(vec![v as u32])
}

fn short(v: u16) -> Value {
    Value::Short(vec![v])
}

/// The DNG-wide tags IFD0 carries besides the thumbnail.
fn dng_tags(request: &DngWrite, newer: bool) -> Vec<(u16, Value)> {
    let version = request.compatibility.version().to_vec();
    let model = request
        .model
        .or(request.make)
        .filter(|m| !m.trim().is_empty())
        .unwrap_or("Unknown camera");
    let mut tags = vec![
        (DNG_VERSION, Value::Byte(version.clone())),
        (DNG_BACKWARD_VERSION, Value::Byte(version)),
        (UNIQUE_CAMERA_MODEL, Value::Ascii(model.to_string())),
        (SOFTWARE, Value::Ascii("Kooka Pictura".to_string())),
        (ORIENTATION, short(request.orientation.clamp(1, 8))),
    ];
    if let Some(make) = request.make {
        tags.push((MAKE, Value::Ascii(make.to_string())));
    }
    if let Some(model) = request.model {
        tags.push((MODEL, Value::Ascii(model.to_string())));
    }
    let color = request.color;
    for i in 0..2 {
        if let Some(m) = &color.color_matrix[i] {
            tags.push((COLOR_MATRIX[i], matrix(m)));
        }
        if let Some(m) = &color.camera_calibration[i] {
            tags.push((CAMERA_CALIBRATION[i], matrix(m)));
        }
        if let (Some(m), true) = (&color.forward_matrix[i], newer) {
            tags.push((FORWARD_MATRIX[i], matrix(m)));
        }
        if let Some(illuminant) = color.calibration_illuminant[i] {
            tags.push((CALIBRATION_ILLUMINANT[i], short(illuminant)));
        }
    }
    let triple = |v: &[f64; 3]| Value::Rational(v.iter().map(|&x| rational(x)).collect());
    if let Some(v) = &color.analog_balance {
        tags.push((ANALOG_BALANCE, triple(v)));
    }
    if let Some(v) = &color.as_shot_neutral {
        tags.push((AS_SHOT_NEUTRAL, triple(v)));
    }
    if color.baseline_exposure != 0.0 {
        tags.push((
            BASELINE_EXPOSURE,
            Value::SRational(vec![srational(color.baseline_exposure)]),
        ));
    }
    if let Some(xmp) = request.xmp {
        tags.push((XMP, Value::Byte(xmp.to_vec())));
    }
    if let Some((name, bytes)) = request.original {
        tags.push((ORIGINAL_RAW_FILE_NAME, Value::Ascii(name.to_string())));
        tags.push((
            ORIGINAL_RAW_FILE_DATA,
            Value::Undefined(original::original_raw_data(bytes)),
        ));
    }
    tags
}

fn image_tags(request: &DngWrite) -> Vec<(u16, Value)> {
    let crop = &request.crop;
    let mut tags = vec![
        (NEW_SUBFILE_TYPE, long(0)),
        (IMAGE_WIDTH, long(request.width)),
        (IMAGE_LENGTH, long(request.height)),
        (COMPRESSION, short(COMPRESSION_JPEG)),
        (PLANAR_CONFIGURATION, short(1)),
        (TILE_WIDTH, long(TILE)),
        (TILE_LENGTH, long(TILE)),
        (
            DEFAULT_SCALE,
            Value::Rational(vec![rational(1.0), rational(1.0)]),
        ),
        (
            DEFAULT_CROP_ORIGIN,
            Value::Rational(vec![rational(crop.x), rational(crop.y)]),
        ),
        (
            DEFAULT_CROP_SIZE,
            Value::Rational(vec![rational(crop.width), rational(crop.height)]),
        ),
    ];
    let spp = match request.pixels {
        DngPixels::Mosaic { cfa, .. } => {
            tags.extend([
                (PHOTOMETRIC, short(PHOTOMETRIC_CFA as u16)),
                (
                    CFA_REPEAT_PATTERN_DIM,
                    Value::Short(vec![cfa.height as u16, cfa.width as u16]),
                ),
                (CFA_PATTERN, Value::Byte(cfa.colors.clone())),
                (CFA_PLANE_COLOR, Value::Byte(vec![0, 1, 2])),
                (CFA_LAYOUT, short(1)),
            ]);
            1
        }
        DngPixels::Linear { .. } => {
            tags.push((PHOTOMETRIC, short(PHOTOMETRIC_LINEAR_RAW as u16)));
            3
        }
    };
    tags.extend([
        (SAMPLES_PER_PIXEL, short(spp as u16)),
        (BITS_PER_SAMPLE, Value::Short(vec![16; spp])),
        (BLACK_LEVEL, Value::Long(vec![0; spp])),
        (WHITE_LEVEL, Value::Long(vec![65535; spp])),
    ]);
    tags
}

/// One-strip IFD tags for 8-bit, three-sample `width × height` pixels.
fn strip_tags(
    subfile: usize,
    width: usize,
    height: usize,
    compression: u16,
    photometric: u16,
    strip: (u32, u32),
) -> Vec<(u16, Value)> {
    vec![
        (NEW_SUBFILE_TYPE, long(subfile)),
        (IMAGE_WIDTH, long(width)),
        (IMAGE_LENGTH, long(height)),
        (BITS_PER_SAMPLE, Value::Short(vec![8; 3])),
        (COMPRESSION, short(compression)),
        (PHOTOMETRIC, short(photometric)),
        (STRIP_OFFSETS, Value::Long(vec![strip.0])),
        (SAMPLES_PER_PIXEL, short(3)),
        (ROWS_PER_STRIP, long(height)),
        (STRIP_BYTE_COUNTS, Value::Long(vec![strip.1])),
        (PLANAR_CONFIGURATION, short(1)),
    ]
}

/// The original raw file a DNG embeds (`OriginalRawFileData` in IFD0),
/// inflated; `None` when it has none or it is damaged.
pub fn read_original_raw(bytes: &[u8]) -> Option<Vec<u8>> {
    let (tiff, first) = super::tiff::Tiff::parse(bytes).ok()?;
    let ifd0 = tiff.ifd(first).ok()?;
    original::inflate_original_raw(tiff.raw(ifd0.get(ORIGINAL_RAW_FILE_DATA)?))
}

/// Writes `request` as a DNG. `progress` gets 0–1 as image tiles finish;
/// `None` when `cancelled` turned true.
pub fn write_dng(
    request: &DngWrite,
    progress: &(dyn Fn(f32) + Sync),
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<Option<Vec<u8>>, PsdError> {
    check(request)?;
    if cancelled() {
        return Ok(None);
    }
    let newer = request.compatibility.version()[1] >= 2;
    let Some(tiles) = encode_tiles(request, progress, cancelled) else {
        return Ok(None);
    };
    let preview = request.preview.as_ref().map(jpeg).transpose()?;
    if cancelled() {
        return Ok(None);
    }

    let mut out = tiff_out::header();
    let (offsets, counts) = tiff_out::append_chunks(&mut out, &tiles)?;
    drop(tiles);
    let mut image = image_tags(request);
    image.push((TILE_OFFSETS, Value::Long(offsets)));
    image.push((TILE_BYTE_COUNTS, Value::Long(counts)));
    let mut subs = vec![tiff_out::append_ifd(&mut out, image)?];

    if let (Some(jpeg), Some(size)) = (preview, request.preview.as_ref()) {
        let (offsets, counts) = tiff_out::append_chunks(&mut out, &[jpeg])?;
        let mut tags = strip_tags(
            1,
            size.width,
            size.height,
            COMPRESSION_JPEG,
            PHOTOMETRIC_YCBCR,
            (offsets[0], counts[0]),
        );
        if newer {
            tags.push((PREVIEW_COLOR_SPACE, long(2)));
        }
        subs.push(tiff_out::append_ifd(&mut out, tags)?);
    }

    let (tw, th, thumb) = thumbnail(&request.look, THUMBNAIL_EDGE);
    let (offsets, counts) = tiff_out::append_chunks(&mut out, &[thumb])?;
    let mut ifd0 = strip_tags(
        1,
        tw,
        th,
        COMPRESSION_NONE,
        PHOTOMETRIC_RGB,
        (offsets[0], counts[0]),
    );
    ifd0.push((SUB_IFDS, Value::Long(subs)));
    ifd0.extend(dng_tags(request, newer));
    let first = tiff_out::append_ifd(&mut out, ifd0)?;
    tiff_out::finish(&mut out, first);
    Ok(Some(out))
}
