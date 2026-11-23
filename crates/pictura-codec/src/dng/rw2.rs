//! Panasonic RW2, RWL and RAW (#1472): a TIFF with its own magic whose IFD0
//! describes the sensor in Panasonic's tags. Three plain codings are read,
//! 16-bit words and two packings in 16-byte groups; the others are refused
//! (#1541). Built from the TIFF specification, ExifTool's public tag
//! documentation and observation of real files.

use rayon::prelude::*;

use super::levels::{self, Area, Levels};
use super::tiff::{Order, Tiff};
use super::vendor::camera_color;
use super::{
    to_usize, CfaPattern, DngCrop, DngImage, Tags, MAKE, ORIENTATION, STRIP_BYTE_COUNTS,
    STRIP_OFFSETS,
};
use crate::error::PsdError;
use crate::probe::ImageBudget;

const MODEL: u16 = 272;
const SENSOR_WIDTH: u16 = 0x0002;
const SENSOR_HEIGHT: u16 = 0x0003;
/// Top, left, bottom, right.
const BORDERS: [u16; 4] = [0x0004, 0x0005, 0x0006, 0x0007];
const CFA: u16 = 0x0009;
const BITS: u16 = 0x000A;
const COMPRESSION: u16 = 0x000B;
const LINEARITY_LIMIT_GREEN: u16 = 0x000F;
const BLACK: [u16; 3] = [0x001C, 0x001D, 0x001E];
const WB_LEVELS: [u16; 3] = [0x0024, 0x0025, 0x0026];
const RAW_FORMAT: u16 = 0x002D;
/// Top, left, bottom, right.
const CROP: [u16; 4] = [0x002F, 0x0030, 0x0031, 0x0032];
const RAW_DATA_OFFSET: u16 = 0x0118;

const MAGIC: &[u8; 4] = b"IIU\0";
const HEADER_LEN: usize = 8;
const GROUP_BYTES: usize = 16;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("RW2: {what}"))
}

fn unsupported(what: &str) -> PsdError {
    PsdError::Unsupported(format!("RW2 {what}"))
}

/// Whether `bytes` begin with the RW2 header.
pub(super) fn is_rw2(bytes: &[u8]) -> bool {
    bytes.len() >= HEADER_LEN && bytes.starts_with(MAGIC)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Coding {
    /// Left-aligned little-endian 16-bit words.
    Words,
    /// As many samples of this many bits as fit in 16 bytes, from the low bit
    /// up, then padding.
    Groups(u32),
}

impl Coding {
    fn of(tags: &Tags) -> Result<Self, PsdError> {
        let format = tags.uint(RAW_FORMAT)?;
        if let Some(n) = format.filter(|n| ![2, 7].contains(n)) {
            return Err(unsupported(&format!(
                "RawFormat {n} sensor data is not supported yet"
            )));
        }
        let compression = tags.uint(COMPRESSION)?;
        let bits = tags
            .uint(BITS)?
            .ok_or_else(|| invalid("missing BitsPerSample"))?;
        match (compression, format, bits) {
            (Some(34828 | 34830), None, 12 | 14) => Ok(Coding::Words),
            (Some(34316), Some(2), 12) => Ok(Coding::Groups(12)),
            (Some(34316), Some(7), 14) => Ok(Coding::Groups(14)),
            _ => Err(unsupported(&format!(
                "{bits}-bit sensor data with compression {compression:?} and RawFormat {format:?}"
            ))),
        }
    }

    fn row_bytes(self, width: usize) -> Result<usize, PsdError> {
        match self {
            Coding::Words => Ok(2 * width),
            Coding::Groups(bits) => {
                let per_group = (128 / bits) as usize;
                if !width.is_multiple_of(per_group) {
                    return Err(unsupported("packed width"));
                }
                Ok(width / per_group * GROUP_BYTES)
            }
        }
    }

    /// The scale of the level tags and the top of their range. The words'
    /// levels are 12-bit ones even where the file declares 14 bits.
    fn level_range(self) -> (f64, f64) {
        match self {
            Coding::Words => (16.0, 4095.0),
            Coding::Groups(bits) => (1.0, ((1u32 << bits) - 1) as f64),
        }
    }

    fn unpack_row(self, src: &[u8], dst: &mut [u16]) {
        match self {
            Coding::Words => {
                for (d, &word) in dst.iter_mut().zip(src.as_chunks::<2>().0) {
                    *d = u16::from_le_bytes(word);
                }
            }
            Coding::Groups(bits) => {
                let per_group = (128 / bits) as usize;
                let mask = (1u128 << bits) - 1;
                for (out, &group) in dst
                    .chunks_exact_mut(per_group)
                    .zip(src.as_chunks::<GROUP_BYTES>().0)
                {
                    let mut value = u128::from_le_bytes(group);
                    for d in out {
                        *d = (value & mask) as u16;
                        value >>= bits;
                    }
                }
            }
        }
    }
}

/// Four rectangle tags as `[top, left, bottom, right]`, when all are present.
fn rectangle(tags: &Tags, names: [u16; 4]) -> Result<Option<[usize; 4]>, PsdError> {
    let mut sides = [0; 4];
    for (side, tag) in sides.iter_mut().zip(names) {
        let Some(value) = tags.uint(tag)? else {
            return Ok(None);
        };
        *side = to_usize(value)?;
    }
    Ok(Some(sides))
}

fn active_area(tags: &Tags, (width, height): (usize, usize)) -> Result<Area, PsdError> {
    let [top, left, bottom, right] =
        rectangle(tags, BORDERS)?.ok_or_else(|| invalid("missing sensor borders"))?;
    if top >= bottom || left >= right || bottom > height || right > width {
        return Err(invalid("sensor borders outside the plane"));
    }
    Ok(Area {
        top,
        left,
        width: right - left,
        height: bottom - top,
    })
}

/// The colours of the area's first 2 × 2; the tag counts from the sensor's
/// corner, and the borders are often odd.
fn cfa(tags: &Tags, area: Area) -> Result<CfaPattern, PsdError> {
    let sensor: [u8; 4] = match tags.uint(CFA)? {
        Some(1) => [0, 1, 1, 2],
        Some(2) => [1, 0, 2, 1],
        Some(3) => [1, 2, 0, 1],
        Some(4) => [2, 1, 1, 0],
        _ => return Err(unsupported("CFA other than the four Bayer patterns")),
    };
    let colors = (0..4)
        .map(|p| sensor[(area.top + p / 2) % 2 * 2 + (area.left + p) % 2])
        .collect();
    Ok(CfaPattern {
        width: 2,
        height: 2,
        colors,
    })
}

// ponytail: a crop reaching outside the borders is dropped, not clipped.
fn default_crop(tags: &Tags, area: Area) -> Result<DngCrop, PsdError> {
    let inside = rectangle(tags, CROP)?.filter(|&[top, left, bottom, right]| {
        (area.top..bottom).contains(&top)
            && (area.left..right).contains(&left)
            && bottom <= area.top + area.height
            && right <= area.left + area.width
    });
    let [top, left, bottom, right] = inside.unwrap_or([
        area.top,
        area.left,
        area.top + area.height,
        area.left + area.width,
    ]);
    Ok(DngCrop {
        x: (left - area.left) as f64,
        y: (top - area.top) as f64,
        width: (right - left) as f64,
        height: (bottom - top) as f64,
    })
}

// ponytail: the green linearity limit is the white of all three colours.
fn levels(tags: &Tags, coding: Coding, cfa: &CfaPattern) -> Result<Levels, PsdError> {
    let mut black = [0.0f64; 3];
    for (value, tag) in black.iter_mut().zip(BLACK) {
        *value = tags.uint(tag)?.unwrap_or(0) as f64;
    }
    let top_black = black.iter().fold(0.0f64, |a, &b| a.max(b));
    let (scale, ceiling) = coding.level_range();
    let white = tags
        .uint(LINEARITY_LIMIT_GREEN)?
        .map(|w| w as f64)
        .filter(|&w| w > top_black && w <= ceiling)
        .unwrap_or(ceiling);
    Ok(Levels {
        linearization: None,
        black: cfa
            .colors
            .iter()
            .map(|&c| black[c as usize] * scale)
            .collect(),
        black_rows: 2,
        black_cols: 2,
        delta_v: Vec::new(),
        delta_h: Vec::new(),
        white: white * scale,
    })
}

fn as_shot_neutral(tags: &Tags) -> Result<Option<[f64; 3]>, PsdError> {
    let mut gains = [0.0f64; 3];
    for (gain, tag) in gains.iter_mut().zip(WB_LEVELS) {
        match tags.uint(tag)? {
            Some(level) if level > 0 => *gain = level as f64,
            _ => return Ok(None),
        }
    }
    Ok(Some(gains.map(|gain| gains[1] / gain)))
}

/// Reads an RW2's sensor plane, normalized and cropped to the sensor borders.
pub(super) fn read_rw2(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
    if !is_rw2(bytes) {
        return Err(invalid("not a Panasonic raw file"));
    }
    let tiff = Tiff::view(bytes, Order::Little);
    let ifd0 = tiff.ifd(tiff.first_ifd()?)?;
    let tags = Tags {
        tiff: &tiff,
        ifd: &ifd0,
    };
    let coding = Coding::of(&tags)?;

    let size = |tag, what| -> Result<usize, PsdError> {
        to_usize(tags.uint(tag)?.ok_or_else(|| invalid(what))?)
    };
    let width = size(SENSOR_WIDTH, "missing SensorWidth")?;
    let height = size(SENSOR_HEIGHT, "missing SensorHeight")?;
    let max = budget.max_dimension as usize;
    if width == 0 || height == 0 || width > max || height > max {
        return Err(invalid(&format!(
            "{width} × {height} is outside the image budget"
        )));
    }
    let area = active_area(&tags, (width, height))?;
    let plane_bytes = (width * height * 2) as u64;
    let total = plane_bytes + (area.width * area.height * 2) as u64;
    if plane_bytes > budget.max_alloc_bytes || total > budget.max_document_bytes {
        return Err(invalid("sensor plane exceeds the allocation budget"));
    }

    let row_bytes = coding.row_bytes(width)?;
    let stored = size(STRIP_BYTE_COUNTS, "missing StripByteCounts")?;
    if stored != row_bytes * height {
        return Err(unsupported("sensor data length"));
    }
    // `StripOffsets` is 0xFFFFFFFF in files that have `RawDataOffset`.
    let at = match tags.uint(RAW_DATA_OFFSET)? {
        Some(at) => to_usize(at)?,
        None => size(STRIP_OFFSETS, "missing sensor data offset")?,
    };
    let data = at
        .checked_add(stored)
        .and_then(|end| bytes.get(at..end))
        .ok_or(PsdError::Truncated)?;

    let cfa = cfa(&tags, area)?;
    let levels = levels(&tags, coding, &cfa)?;
    let default_crop = default_crop(&tags, area)?;
    let neutral = as_shot_neutral(&tags)?;

    let mut plane = vec![0u16; width * height];
    plane
        .par_chunks_mut(width)
        .zip(data.par_chunks(row_bytes))
        .for_each(|(dst, src)| coding.unpack_row(src, dst));
    let samples = levels::normalize(&plane, width, area, &levels);

    let (make, model) = (tags.ascii(MAKE), tags.ascii(MODEL));
    let color = camera_color(make.as_deref(), model.as_deref(), neutral);
    Ok(DngImage {
        width: area.width,
        height: area.height,
        samples,
        cfa,
        default_crop,
        orientation: tags.uint(ORIENTATION)?.unwrap_or(1).min(8) as u16,
        make,
        model,
        color,
    })
}
