//! Canon CR2 (#399): a little-endian TIFF whose raw IFD, named by the header,
//! holds the sensor plane as one lossless JPEG strip cut into vertical
//! slices; one frame row may carry several sensor rows (EOS 5DS). Built from the TIFF and Exif specifications, ITU T.81 and public
//! write-ups of the CR2 layout; colour is not read (the file has no matrix).

use super::levels::{self, Area, Levels};
use super::tiff::Tiff;
use super::vendor::{coord, maker_note_ifd, masked_black};
use super::{
    ljpeg, to_usize, CfaPattern, DngColor, DngCrop, DngImage, Tags, COMPRESSION, IMAGE_LENGTH,
    IMAGE_WIDTH, MAKE, ORIENTATION, STRIP_BYTE_COUNTS, STRIP_OFFSETS,
};
use crate::error::PsdError;
use crate::probe::ImageBudget;

const MODEL: u16 = 272;
const SLICES: u16 = 0xC640;
const SENSOR_INFO: u16 = 0x00E0;
const OLD_JPEG: u64 = 6;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("CR2: {what}"))
}

pub(super) fn is_cr2(bytes: &[u8]) -> bool {
    bytes.len() >= 16 && &bytes[..4] == b"II*\0" && &bytes[8..10] == b"CR" && bytes[10] == 2
}

/// Where the `p`-th decoded sample lands: `n` slices `w` wide, then one
/// `last` wide, each filled top to bottom.
#[derive(Clone, Copy)]
struct Slices {
    n: usize,
    w: usize,
    last: usize,
}

impl Slices {
    fn place(self, p: usize, height: usize) -> (usize, usize) {
        let full = self.n * self.w * height;
        if p < full {
            let q = p % (self.w * height);
            ((p / (self.w * height)) * self.w + q % self.w, q / self.w)
        } else {
            let q = p - full;
            (self.n * self.w + q % self.last, q / self.last)
        }
    }
}

fn slices(tags: &Tags, width: usize) -> Result<Slices, PsdError> {
    let Some(v) = tags.uints(SLICES)? else {
        return Ok(Slices {
            n: 0,
            w: width,
            last: width,
        });
    };
    let [n, w, last] = v[..] else {
        return Err(invalid("slice tag count"));
    };
    let [n, w, last] = [n, w, last].map(|v| v as usize);
    if last == 0
        || (n > 0 && w == 0)
        || n.checked_mul(w).and_then(|f| f.checked_add(last)) != Some(width)
    {
        return Err(invalid("slices do not add up to the sensor width"));
    }
    Ok(Slices { n, w, last })
}

/// The maker note's sensor information (Canon tag `0x00E0`): entries 1–2
/// are the sensor width and height, 5–8 the active area's inclusive left,
/// top, right and bottom.
fn sensor_info(ifd0: &Tags) -> Result<Option<Vec<f64>>, PsdError> {
    let Some(note) = maker_note_ifd(ifd0)? else {
        return Ok(None);
    };
    let note = Tags {
        tiff: ifd0.tiff,
        ifd: &note,
    };
    let info = note.reals(SENSOR_INFO)?;
    if info.as_ref().is_some_and(|v| v.len() < 9) {
        return Err(invalid("sensor information count"));
    }
    Ok(info)
}

/// The sensor plane's size: the raw IFD's `ImageWidth` and `ImageLength`,
/// else the sensor information's, else the frame's. A frame row may carry
/// several sensor rows, so only the sample count must agree.
fn plane_size(
    tags: &Tags,
    info: Option<&[f64]>,
    frame: (usize, usize),
) -> Result<(usize, usize), PsdError> {
    let (width, height) = match (tags.uint(IMAGE_WIDTH)?, tags.uint(IMAGE_LENGTH)?) {
        (Some(w), Some(h)) => (to_usize(w)?, to_usize(h)?),
        _ => match info {
            Some(v) => (coord(v[1])?, coord(v[2])?),
            None => frame,
        },
    };
    if width.checked_mul(height) != Some(frame.0 * frame.1) {
        return Err(invalid("the frame does not hold the sensor plane"));
    }
    Ok((width, height))
}

/// The active area from the sensor information; the whole plane without it.
fn active_area(info: Option<&[f64]>, width: usize, height: usize) -> Result<Area, PsdError> {
    let Some(v) = info else {
        return Ok(Area {
            top: 0,
            left: 0,
            width,
            height,
        });
    };
    let [left, top, right, bottom] = [v[5], v[6], v[7], v[8]].map(coord);
    let (left, top, right, bottom) = (left?, top?, right?, bottom?);
    if left > right || top > bottom || right >= width || bottom >= height {
        return Err(invalid("active area outside the sensor plane"));
    }
    Ok(Area {
        top,
        left,
        width: right - left + 1,
        height: bottom - top + 1,
    })
}

/// The masked border left of the active area, else above it.
fn mask(area: Area) -> Option<Area> {
    if area.left > 0 {
        Some(Area {
            top: area.top,
            left: 0,
            width: area.left,
            height: area.height,
        })
    } else if area.top > 0 {
        Some(Area {
            top: 0,
            left: area.left,
            width: area.width,
            height: area.top,
        })
    } else {
        None
    }
}

/// The CFA at the active corner: RGGB at the sensor plane's corner, shifted
/// by the active area's offset (observed: the EOS 450D's odd top border gives
/// GBRG, the EOS 5DS's even borders RGGB).
fn rggb_from_sensor_corner(area: Area) -> Vec<u8> {
    const RGGB: [u8; 4] = [0, 1, 1, 2];
    (0..4)
        .map(|i| RGGB[((i / 2 + area.top) % 2) * 2 + (i % 2 + area.left) % 2])
        .collect()
}

/// Reads a CR2's sensor plane, normalized and cropped to its active area.
// ponytail: white is the frame's precision ceiling, not the camera's
// saturation (ColorData's per-version white levels are not read), and the
// black is one mean per CFA position.
pub(super) fn read_cr2(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
    if !is_cr2(bytes) {
        return Err(invalid("not a CR2 header"));
    }
    let (tiff, first) = Tiff::parse(bytes)?;
    let ifd0 = tiff.ifd(first)?;
    let ifd0 = Tags {
        tiff: &tiff,
        ifd: &ifd0,
    };
    let raw_at = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let raw = tiff.ifd(raw_at as u64)?;
    let tags = Tags {
        tiff: &tiff,
        ifd: &raw,
    };
    let compression = tags.uint(COMPRESSION)?.unwrap_or(1);
    if compression != OLD_JPEG {
        return Err(PsdError::Unsupported(format!(
            "CR2 compression {compression}"
        )));
    }
    let start = to_usize(
        tags.uint(STRIP_OFFSETS)?
            .ok_or_else(|| invalid("missing StripOffsets"))?,
    )?;
    let len = to_usize(
        tags.uint(STRIP_BYTE_COUNTS)?
            .ok_or_else(|| invalid("missing StripByteCounts"))?,
    )?;
    let data = start
        .checked_add(len)
        .and_then(|end| bytes.get(start..end))
        .ok_or(PsdError::Truncated)?;

    let (precision, frame_w, frame_h, components) = ljpeg::frame(data)?;
    let info = sensor_info(&ifd0)?;
    let (width, height) = plane_size(&tags, info.as_deref(), (frame_w * components, frame_h))?;
    let max = budget.max_dimension as usize;
    if width > max || height > max {
        return Err(invalid(&format!(
            "{width} × {height} is outside the image budget"
        )));
    }
    let slices = slices(&tags, width)?;
    let area = active_area(info.as_deref(), width, height)?;
    let plane_bytes = (width * height * 2) as u64;
    let total = plane_bytes + (area.width * area.height * 2) as u64;
    if plane_bytes > budget.max_alloc_bytes || total > budget.max_document_bytes {
        return Err(invalid("sensor plane exceeds the allocation budget"));
    }
    // Every sample costs at least one bit (the shortest Huffman code).
    if data.len() < (width * height).div_ceil(8) {
        return Err(PsdError::Truncated);
    }

    let mut plane = vec![0u16; width * height];
    ljpeg::decode(data, width * height, |first, samples| {
        for (k, &v) in samples.iter().enumerate() {
            let (x, y) = slices.place(first + k, height);
            plane[y * width + x] = v;
        }
    })?;

    let black = match mask(area) {
        Some(m) => masked_black(&plane, width, m, (area.left, area.top)),
        None => vec![0.0; 4],
    };
    let levels = Levels {
        linearization: None,
        black,
        black_rows: 2,
        black_cols: 2,
        delta_v: Vec::new(),
        delta_h: Vec::new(),
        white: ((1u32 << precision) - 1) as f64,
    };
    let samples = levels::normalize(&plane, width, area, &levels);
    Ok(DngImage {
        width: area.width,
        height: area.height,
        samples,
        cfa: CfaPattern {
            width: 2,
            height: 2,
            colors: rggb_from_sensor_corner(area),
        },
        default_crop: DngCrop {
            x: 0.0,
            y: 0.0,
            width: area.width as f64,
            height: area.height as f64,
        },
        orientation: ifd0.uint(ORIENTATION)?.unwrap_or(1).min(8) as u16,
        make: ifd0.ascii(MAKE),
        model: ifd0.ascii(MODEL),
        color: DngColor::default(),
    })
}
