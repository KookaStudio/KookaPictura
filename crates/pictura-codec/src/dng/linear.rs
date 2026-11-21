//! Linear DNGs (#402): a three-sample `LinearRaw` image read as camera RGB,
//! normalized per sample and cropped to the active area. The container walk,
//! layout and tile decode are the CFA reader's.

use rayon::prelude::*;

use super::tiff::Tiff;
use super::{
    active_area, color, decode_plane, default_crop, invalid, layout, raw_ifd, to_usize,
    unsupported, CameraRgb, DngColor, DngCrop, Tags, BITS_PER_SAMPLE, BLACK_LEVEL,
    BLACK_LEVEL_DELTA_H, BLACK_LEVEL_DELTA_V, BLACK_LEVEL_REPEAT_DIM, COMPRESSION, DNG_VERSION,
    IMAGE_LENGTH, IMAGE_WIDTH, LINEARIZATION_TABLE, MAKE, ORIENTATION, PHOTOMETRIC,
    PHOTOMETRIC_LINEAR_RAW, SAMPLES_PER_PIXEL, SAMPLE_FORMAT, UNIQUE_CAMERA_MODEL, WHITE_LEVEL,
};
use crate::error::PsdError;
use crate::probe::ImageBudget;

const SPP: usize = 3;

/// A linear DNG's camera RGB and the tags the develop needs.
#[derive(Debug, Clone)]
pub struct LinearDng {
    /// Black 0, white 65535, cropped to the active area.
    pub camera: CameraRgb,
    pub default_crop: DngCrop,
    pub orientation: u16,
    pub make: Option<String>,
    pub model: Option<String>,
    pub color: DngColor,
}

/// Whether `bytes` is a DNG whose full-resolution image is `LinearRaw`.
pub(super) fn is_linear_dng(bytes: &[u8]) -> bool {
    let Ok((tiff, first)) = Tiff::parse(bytes) else {
        return false;
    };
    let Ok(ifd0) = tiff.ifd(first) else {
        return false;
    };
    if ifd0.get(DNG_VERSION).is_none() {
        return false;
    }
    raw_ifd(&tiff, first).is_ok_and(|raw| {
        let tags = Tags {
            tiff: &tiff,
            ifd: &raw,
        };
        tags.uint(PHOTOMETRIC).ok().flatten() == Some(PHOTOMETRIC_LINEAR_RAW)
    })
}

/// One value, or one per sample.
fn per_sample(values: Option<Vec<f64>>, default: f64, what: &str) -> Result<[f64; SPP], PsdError> {
    match values.as_deref() {
        None => Ok([default; SPP]),
        Some(&[v]) => Ok([v; SPP]),
        Some(&[a, b, c]) => Ok([a, b, c]),
        Some(_) => Err(invalid(&format!("{what} count"))),
    }
}

/// Reads a linear DNG's full-resolution camera RGB, normalized and cropped to
/// its active area. Declared sizes are charged against `budget` first.
pub fn read_linear_dng(bytes: &[u8], budget: &ImageBudget) -> Result<LinearDng, PsdError> {
    let (tiff, first) = Tiff::parse(bytes)?;
    let ifd0 = tiff.ifd(first)?;
    let ifd0 = Tags {
        tiff: &tiff,
        ifd: &ifd0,
    };
    if ifd0.ifd.get(DNG_VERSION).is_none() {
        return Err(invalid("TIFF without DNGVersion"));
    }
    let raw = raw_ifd(&tiff, first)?;
    let tags = Tags {
        tiff: &tiff,
        ifd: &raw,
    };
    if tags.uint(PHOTOMETRIC)? != Some(PHOTOMETRIC_LINEAR_RAW) {
        return Err(unsupported("image that is not linear raw"));
    }
    if tags.uint(SAMPLES_PER_PIXEL)?.unwrap_or(1) != SPP as u64 {
        return Err(unsupported("linear raw image without three samples"));
    }
    if tags.uint(SAMPLE_FORMAT)?.unwrap_or(1) != 1 {
        return Err(unsupported("floating-point or signed samples"));
    }
    if raw.get(BLACK_LEVEL_DELTA_H).is_some() || raw.get(BLACK_LEVEL_DELTA_V).is_some() {
        return Err(unsupported("linear raw black-level deltas"));
    }
    if let Some(dim) = tags.uints(BLACK_LEVEL_REPEAT_DIM)? {
        if dim != [1, 1] {
            return Err(unsupported("linear raw black-level repeat pattern"));
        }
    }
    let compression = tags.uint(COMPRESSION)?.unwrap_or(1);
    if compression != 1 && compression != 7 {
        return Err(unsupported(&format!("compression {compression}")));
    }
    let bits = tags.uint(BITS_PER_SAMPLE)?.unwrap_or(1) as u32;
    let bits_ok = match compression {
        1 => (8..=16).contains(&bits),
        _ => (2..=16).contains(&bits),
    };
    if !bits_ok {
        return Err(unsupported(&format!("{bits}-bit samples")));
    }

    let width = to_usize(
        tags.uint(IMAGE_WIDTH)?
            .ok_or_else(|| invalid("missing ImageWidth"))?,
    )?;
    let height = to_usize(
        tags.uint(IMAGE_LENGTH)?
            .ok_or_else(|| invalid("missing ImageLength"))?,
    )?;
    let max = budget.max_dimension as usize;
    if width == 0 || height == 0 || width > max || height > max {
        return Err(invalid(&format!(
            "{width} × {height} is outside the image budget"
        )));
    }
    let area = active_area(&tags, width, height)?;
    let plane = (width * height * SPP * 2) as u64;
    let total = plane + (area.width * area.height * SPP * 2) as u64;
    if plane > budget.max_alloc_bytes || total > budget.max_document_bytes {
        return Err(invalid("image exceeds the allocation budget"));
    }
    let layout = layout(&tags, width, height, budget)?;
    let linearization: Option<Vec<u16>> = tags
        .uints(LINEARIZATION_TABLE)?
        .filter(|t| !t.is_empty())
        .map(|t| t.into_iter().map(|v| v.min(65535) as u16).collect());
    let black = per_sample(tags.reals(BLACK_LEVEL)?, 0.0, "BlackLevel")?;
    let white = per_sample(
        tags.reals(WHITE_LEVEL)?,
        ((1u32 << bits) - 1) as f64,
        "WhiteLevel",
    )?;
    let default_crop = default_crop(&tags, area)?;
    let color = color(&ifd0)?;

    let raw_plane = decode_plane(&tiff, &layout, compression, bits, (width, height), SPP)?;
    let mut rgb = vec![0u16; area.width * area.height * SPP];
    rgb.par_chunks_mut(area.width * SPP)
        .enumerate()
        .for_each(|(y, dst)| {
            let src = &raw_plane[((area.top + y) * width + area.left) * SPP..][..dst.len()];
            for (i, (d, &v)) in dst.iter_mut().zip(src).enumerate() {
                let v = match &linearization {
                    Some(table) => table[(v as usize).min(table.len() - 1)],
                    None => v,
                } as f64;
                let c = i % SPP;
                let range = white[c] - black[c];
                let n = if range > 0.0 {
                    ((v - black[c]) * 65535.0 / range).round()
                } else if v >= white[c] {
                    65535.0
                } else {
                    0.0
                };
                *d = n.clamp(0.0, 65535.0) as u16;
            }
        });
    Ok(LinearDng {
        camera: CameraRgb {
            width: area.width,
            height: area.height,
            rgb,
        },
        default_crop,
        orientation: ifd0.uint(ORIENTATION)?.unwrap_or(1).min(8) as u16,
        make: ifd0.ascii(MAKE),
        model: ifd0.ascii(UNIQUE_CAMERA_MODEL),
        color,
    })
}
