//! Fujifilm RAF (#1473): a big-endian header naming three sections (the
//! camera JPEG, a directory of 16-bit-tagged records and the sensor section),
//! the sensor section being a TIFF whose one entry points at Fujifilm's raw
//! directory. Uncompressed 16-bit words, the 12- and 14-bit packings and
//! Fujifilm's compressed data (`raf_compressed.rs`, #1620) are read for
//! X-Trans and Bayer sensors; SuperCCD and EXR sensors (#1621) are refused.
//! Built from ExifTool's public RAF tag documentation and observation of real
//! files.

use rayon::prelude::*;

use super::levels::{self, Area, Levels};
use super::raf_compressed;
use super::tiff::{Ifd, Order, Tiff};
use super::{to_usize, CfaPattern, DngColor, DngCrop, DngImage, Tags, MAKE, ORIENTATION};
use crate::error::PsdError;
use crate::probe::ImageBudget;

const MAGIC: &[u8; 16] = b"FUJIFILMCCD-RAW ";
const MODEL: u16 = 272;
const CAMERA_NAME: std::ops::Range<usize> = 0x1C..0x3C;
const JPEG_AT: usize = 0x54;
const RECORDS_AT: usize = 0x5C;
const SENSOR_AT: usize = 0x64;
/// A second directory and sensor section: the EXR sensors' second frame.
const SECOND_FRAME_AT: usize = 0x78;
const HEADER_LEN: usize = 0x80;

/// Directory records.
const CROP_TOP_LEFT: u16 = 0x0110;
const CROP_SIZE: u16 = 0x0111;
const LAYOUT: u16 = 0x0130;
const XTRANS_LAYOUT: u16 = 0x0131;
/// The exposure the camera held back from the raw, a signed ratio.
const RAW_EXPOSURE_BIAS: u16 = 0x9650;
const LAYOUT_BAYER: [u8; 4] = [0x0A, 0x0B, 0x09, 0x08];
const LAYOUT_XTRANS: [u8; 4] = [0x0C; 4];

/// The sensor section's pointer to the raw directory, and that directory.
const RAW_IFD: u16 = 0xF000;
const RAW_WIDTH: u16 = 0xF001;
const RAW_HEIGHT: u16 = 0xF002;
const RAW_BITS: u16 = 0xF003;
const ROW_WORDS: u16 = 0xF005;
const DATA_OFFSET: u16 = 0xF007;
const DATA_BYTES: u16 = 0xF008;
const BLACK_LEVEL: u16 = 0xF00A;
const WB_GRB: u16 = 0xF00E;

const EXIF_MARKER: u8 = 0xE1;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("RAF: {what}"))
}

fn unsupported(what: &str) -> PsdError {
    PsdError::Unsupported(format!("RAF {what}"))
}

/// Whether `bytes` begin with the RAF magic.
pub(super) fn is_raf(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

fn be32(bytes: &[u8], at: usize) -> Result<usize, PsdError> {
    let b = bytes.get(at..at + 4).ok_or(PsdError::Truncated)?;
    to_usize(u32::from_be_bytes([b[0], b[1], b[2], b[3]]).into())
}

/// The section whose offset and length the header stores at `at`.
fn section(bytes: &[u8], at: usize) -> Result<&[u8], PsdError> {
    let (start, len) = (be32(bytes, at)?, be32(bytes, at + 4)?);
    start
        .checked_add(len)
        .and_then(|end| bytes.get(start..end))
        .ok_or(PsdError::Truncated)
}

/// The directory's records, tag → value bytes.
fn records(bytes: &[u8]) -> Result<Vec<(u16, &[u8])>, PsdError> {
    let dir = section(bytes, RECORDS_AT)?;
    let count = be32(dir, 0)?;
    let mut out = Vec::new();
    let mut at = 4;
    for _ in 0..count {
        let head = dir.get(at..at + 4).ok_or(PsdError::Truncated)?;
        let tag = u16::from_be_bytes([head[0], head[1]]);
        let len = u16::from_be_bytes([head[2], head[3]]) as usize;
        let value = dir.get(at + 4..at + 4 + len).ok_or(PsdError::Truncated)?;
        out.push((tag, value));
        at += 4 + len;
    }
    Ok(out)
}

fn record<'a>(records: &[(u16, &'a [u8])], tag: u16) -> Option<&'a [u8]> {
    records.iter().find(|(t, _)| *t == tag).map(|&(_, v)| v)
}

/// A record of two big-endian 16-bit values.
fn pair(records: &[(u16, &[u8])], tag: u16) -> Result<Option<(usize, usize)>, PsdError> {
    match record(records, tag) {
        None => Ok(None),
        Some(&[a, b, c, d]) => Ok(Some((
            u16::from_be_bytes([a, b]) as usize,
            u16::from_be_bytes([c, d]) as usize,
        ))),
        Some(_) => Err(invalid(&format!("record {tag:#06x} size"))),
    }
}

/// The CFA, measured from the sensor plane's origin. The X-Trans record
/// lists the 6 × 6 pattern from its last photosite back to its first.
fn cfa(records: &[(u16, &[u8])]) -> Result<CfaPattern, PsdError> {
    let layout = record(records, LAYOUT).ok_or_else(|| unsupported("without a sensor layout"))?;
    if layout == LAYOUT_BAYER && record(records, XTRANS_LAYOUT).is_none() {
        return Ok(CfaPattern {
            width: 2,
            height: 2,
            colors: vec![0, 1, 1, 2],
        });
    }
    if layout != LAYOUT_XTRANS {
        return Err(unsupported(
            "sensor layout (SuperCCD or EXR) is not supported yet",
        ));
    }
    let pattern = record(records, XTRANS_LAYOUT)
        .filter(|p| p.len() == 36)
        .ok_or_else(|| invalid("X-Trans layout"))?;
    let colors: Vec<u8> = pattern.iter().rev().copied().collect();
    if colors.iter().any(|&c| c > 2) || (0..3u8).any(|c| !colors.contains(&c)) {
        return Err(invalid("X-Trans layout colours"));
    }
    Ok(CfaPattern {
        width: 6,
        height: 6,
        colors,
    })
}

/// IFD0, Exif-style, of the camera JPEG's `Exif` APP1 segment.
fn jpeg_exif(jpeg: &[u8]) -> Option<&[u8]> {
    let mut at = 2;
    while jpeg.get(at) == Some(&0xFF) {
        let marker = *jpeg.get(at + 1)?;
        let len = u16::from_be_bytes([*jpeg.get(at + 2)?, *jpeg.get(at + 3)?]) as usize;
        let payload = jpeg.get(at + 4..at + 2 + len)?;
        if marker == EXIF_MARKER {
            if let Some(tiff) = payload.strip_prefix(b"Exif\0\0") {
                return Some(tiff);
            }
        }
        if marker == 0xDA {
            return None;
        }
        at += 2 + len;
    }
    None
}

/// The camera JPEG's Exif TIFF, the shooting data a RAF carries.
pub(super) fn exif(bytes: &[u8]) -> Option<&[u8]> {
    if !is_raf(bytes) {
        return None;
    }
    section(bytes, JPEG_AT).ok().and_then(jpeg_exif)
}

/// The stops the camera underexposed the raw by to keep highlights (its
/// dynamic-range modes), from `0x9650`; 0 when absent or unusable.
fn exposure_bias(records: &[(u16, &[u8])]) -> f64 {
    match record(records, RAW_EXPOSURE_BIAS) {
        Some(&[a, b, c, d]) => {
            let (num, den) = (i16::from_be_bytes([a, b]), i16::from_be_bytes([c, d]));
            if den == 0 {
                0.0
            } else {
                f64::from(num) / f64::from(den)
            }
        }
        _ => 0.0,
    }
}

/// Make, model and orientation from the camera JPEG's Exif IFD0; the
/// header's camera name stands in for a missing model.
fn camera(bytes: &[u8]) -> (Option<String>, Option<String>, u16) {
    let header_name = bytes
        .get(CAMERA_NAME)
        .map(|b| b.split(|&c| c == 0).next().unwrap_or(b))
        .and_then(|b| std::str::from_utf8(b).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let exif = exif(bytes);
    let ifd0 = exif.and_then(|exif| {
        let (tiff, first) = Tiff::parse(exif).ok()?;
        let ifd = tiff.ifd(first).ok()?;
        Some((tiff, ifd))
    });
    let Some((tiff, ifd)) = ifd0 else {
        return (Some("FUJIFILM".into()), header_name, 1);
    };
    let tags = Tags {
        tiff: &tiff,
        ifd: &ifd,
    };
    let orientation = tags
        .uint(ORIENTATION)
        .ok()
        .flatten()
        .unwrap_or(1)
        .clamp(1, 8) as u16;
    (
        tags.ascii(MAKE).or(Some("FUJIFILM".into())),
        tags.ascii(MODEL).or(header_name),
        orientation,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Coding {
    /// 16-bit words in the sensor section's byte order.
    Words,
    /// 12-bit samples, least significant bit first, `stride` bytes a row.
    Lsb12 { stride: usize },
    /// 14-bit samples, most significant bit first over little-endian 32-bit
    /// words, `stride` bytes a row.
    Msb14 { stride: usize },
}

fn coding(
    raw: &Tags,
    stored: usize,
    bits: u32,
    (width, height): (usize, usize),
) -> Result<Coding, PsdError> {
    let n = width * height;
    // Word files record a row length of 0 (or none); packed files their row
    // length, which may total two bytes a photosite once padded.
    let stride = match raw.uint(ROW_WORDS)? {
        None | Some(0) => None,
        Some(words) => Some(
            to_usize(words)?
                .checked_mul(4)
                .ok_or_else(|| invalid("row length"))?,
        ),
    };
    if stored == 2 * n && stride.is_none() {
        return Ok(Coding::Words);
    }
    let packed = (n * bits as usize).div_ceil(8);
    if stored < packed {
        return Err(unsupported(
            "sensor data shorter than its packed size has no compressed-data header",
        ));
    }
    let stride = stride.ok_or_else(|| unsupported("sensor data layout"))?;
    if stride < (width * bits as usize).div_ceil(8) || stride.checked_mul(height) != Some(stored) {
        return Err(unsupported("sensor data layout"));
    }
    match bits {
        12 => Ok(Coding::Lsb12 { stride }),
        14 => Ok(Coding::Msb14 { stride }),
        _ => Err(unsupported(&format!("{bits}-bit packed samples"))),
    }
}

fn unpack_lsb12(src: &[u8], dst: &mut [u16]) {
    for (b, pair) in src
        .as_chunks::<3>()
        .0
        .iter()
        .zip(dst.as_chunks_mut::<2>().0)
    {
        let [a, m, c] = b.map(u16::from);
        pair[0] = a | (m & 0xF) << 8;
        pair[1] = m >> 4 | c << 4;
    }
    if dst.len() % 2 == 1 {
        let i = dst.len() - 1;
        let at = i / 2 * 3;
        dst[i] = u16::from(src[at]) | u16::from(src[at + 1] & 0xF) << 8;
    }
}

fn unpack_msb14(src: &[u8], dst: &mut [u16]) {
    let (mut acc, mut have) = (0u64, 0u32);
    let mut words = src.as_chunks::<4>().0.iter();
    for d in dst {
        if have < 14 {
            let w = words.next().map_or(0, |&w| u32::from_le_bytes(w));
            acc = acc << 32 | u64::from(w);
            have += 32;
        }
        have -= 14;
        *d = (acc >> have) as u16 & 0x3FFF;
    }
}

fn decode(
    data: &[u8],
    coding: Coding,
    big_endian: bool,
    (width, height): (usize, usize),
) -> Vec<u16> {
    let mut plane = vec![0u16; width * height];
    plane
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(y, dst)| match coding {
            Coding::Words => {
                let row = &data[y * width * 2..][..width * 2];
                for (d, w) in dst.iter_mut().zip(row.as_chunks::<2>().0) {
                    *d = if big_endian {
                        u16::from_be_bytes(*w)
                    } else {
                        u16::from_le_bytes(*w)
                    };
                }
            }
            Coding::Lsb12 { stride } => unpack_lsb12(&data[y * stride..][..stride], dst),
            Coding::Msb14 { stride } => unpack_msb14(&data[y * stride..][..stride], dst),
        });
    plane
}

/// The default crop: the directory's crop when it lies in the plane.
fn default_crop(
    records: &[(u16, &[u8])],
    (width, height): (usize, usize),
) -> Result<DngCrop, PsdError> {
    let whole = DngCrop {
        x: 0.0,
        y: 0.0,
        width: width as f64,
        height: height as f64,
    };
    match (pair(records, CROP_TOP_LEFT)?, pair(records, CROP_SIZE)?) {
        (Some((top, left)), Some((h, w))) if h > 0 && w > 0 => {
            if left + w > width || top + h > height {
                return Err(invalid("crop outside the plane"));
            }
            Ok(DngCrop {
                x: left as f64,
                y: top as f64,
                width: w as f64,
                height: h as f64,
            })
        }
        _ => Ok(whole),
    }
}

/// `BlackLevel`: one value, one per 2 × 2 position, or one per 6 × 6.
fn black(raw: &Tags) -> Result<(Vec<f64>, usize), PsdError> {
    let Some(values) = raw.uints(BLACK_LEVEL)? else {
        return Ok((vec![0.0], 1));
    };
    let side = match values.len() {
        1 => 1,
        4 => 2,
        36 => 6,
        _ => return Err(invalid("black level count")),
    };
    Ok((values.into_iter().map(|v| v as f64).collect(), side))
}

/// The as-shot neutral from the green, red and blue levels.
fn as_shot_neutral(raw: &Tags) -> Result<Option<[f64; 3]>, PsdError> {
    Ok(match raw.uints(WB_GRB)?.as_deref() {
        Some(&[g, r, b, ..]) if g > 0 && r > 0 && b > 0 => {
            Some([g as f64 / r as f64, 1.0, g as f64 / b as f64])
        }
        _ => None,
    })
}

/// Reads a RAF's sensor plane, normalized over the whole plane.
pub(super) fn read_raf(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
    if !is_raf(bytes) {
        return Err(invalid("not a Fujifilm raw file"));
    }
    if bytes.len() < HEADER_LEN {
        return Err(PsdError::Truncated);
    }
    if be32(bytes, SECOND_FRAME_AT)? != 0 {
        return Err(unsupported(
            "with two sensor frames (EXR) is not supported yet",
        ));
    }
    let records = records(bytes)?;
    let cfa = cfa(&records)?;
    let sensor = section(bytes, SENSOR_AT)?;
    let Ok((tiff, first)) = Tiff::parse(sensor) else {
        return Err(unsupported(
            "without a raw directory (SuperCCD) is not supported yet",
        ));
    };
    let top: Ifd = tiff.ifd(first)?;
    let raw_at = top
        .get(RAW_IFD)
        .map(|e| tiff.uint(e))
        .transpose()?
        .ok_or_else(|| invalid("missing raw directory"))?;
    let raw_ifd = tiff.ifd(raw_at)?;
    let raw = Tags {
        tiff: &tiff,
        ifd: &raw_ifd,
    };
    let size = |tag, what| -> Result<usize, PsdError> {
        to_usize(raw.uint(tag)?.ok_or_else(|| invalid(what))?)
    };
    let width = size(RAW_WIDTH, "missing raw width")?;
    let height = size(RAW_HEIGHT, "missing raw height")?;
    let start = size(DATA_OFFSET, "missing data offset")?;
    let stored = size(DATA_BYTES, "missing data size")?;
    // Compressed data may run past its recorded size (LibRaw notes the
    // X-H2S), so its blocks are bounded by the section instead.
    let tail = sensor.get(start..).ok_or(PsdError::Truncated)?;
    let compressed = match width.checked_mul(height) {
        Some(n) if n.checked_mul(2) == Some(stored) => None,
        _ => raf_compressed::header(tail).transpose()?,
    };
    let (width, height) = compressed.map_or((width, height), |h| (h.width, h.height));
    let max = budget.max_dimension as usize;
    if width == 0 || height == 0 || width > max || height > max {
        return Err(invalid(&format!(
            "{width} × {height} is outside the image budget"
        )));
    }
    let plane_bytes = (width * height * 2) as u64;
    if plane_bytes > budget.max_alloc_bytes || 2 * plane_bytes > budget.max_document_bytes {
        return Err(invalid("sensor plane exceeds the allocation budget"));
    }
    let (plane, bits) = match compressed {
        Some(header) => (raf_compressed::decode(tail, &header, &cfa)?, header.bits),
        None => {
            let bits = raw.uint(RAW_BITS)?.unwrap_or(14);
            if !(8..=16).contains(&bits) {
                return Err(unsupported(&format!("{bits}-bit samples")));
            }
            let bits = bits as u32;
            let coding = coding(&raw, stored, bits, (width, height))?;
            let data = tail.get(..stored).ok_or(PsdError::Truncated)?;
            let big_endian = tiff.order == Order::Big;
            (decode(data, coding, big_endian, (width, height)), bits)
        }
    };

    let (black, side) = black(&raw)?;
    let ceiling = ((1u32 << bits) - 1) as f64;
    let levels = Levels {
        linearization: None,
        black,
        black_rows: side,
        black_cols: side,
        delta_v: Vec::new(),
        delta_h: Vec::new(),
        white: ceiling,
    };
    let area = Area {
        top: 0,
        left: 0,
        width,
        height,
    };
    let samples = levels::normalize(&plane, width, area, &levels);
    let default_crop = default_crop(&records, (width, height))?;
    let (make, model, orientation) = camera(bytes);
    let mut color: DngColor =
        super::vendor::camera_color(make.as_deref(), model.as_deref(), as_shot_neutral(&raw)?);
    // The camera lifts its own JPEG by the bias; so does the develop.
    color.baseline_exposure = -exposure_bias(&records);
    Ok(DngImage {
        width,
        height,
        samples,
        cfa,
        default_crop,
        orientation,
        make,
        model,
        color,
    })
}
