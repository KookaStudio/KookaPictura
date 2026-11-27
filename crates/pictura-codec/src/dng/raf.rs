//! Fujifilm RAF (#1473): a big-endian header naming three sections (the
//! camera JPEG, a directory of 16-bit-tagged records and the sensor section),
//! the sensor section being a TIFF whose one entry points at Fujifilm's raw
//! directory. Uncompressed 16-bit words, the 12- and 14-bit packings and
//! Fujifilm's compressed data (`raf_compressed.rs`, #1620) are read for
//! X-Trans and Bayer sensors. Older bodies (#1621) store plain words in a
//! sensor section that is not a TIFF; SuperCCD bodies turn their lattice by
//! 45°, repacked as LibRaw does (`raf_superccd.rs`). Built from ExifTool's
//! public RAF tag documentation and observation of real files; the word
//! sections' byte order, frame layout and levels follow LibRaw 0.22.1.

use rayon::prelude::*;

use super::levels::{self, Area, Levels};
use super::raf_compressed;
use super::raf_superccd::{self, UPRIGHT_SIZE};
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
/// A second directory: LibRaw counts it as a second frame.
const SECOND_FRAME_AT: usize = 0x78;
const HEADER_LEN: usize = 0x80;

/// Directory records.
const FULL_SIZE: u16 = 0x0100;
const CROP_TOP_LEFT: u16 = 0x0110;
const CROP_SIZE: u16 = 0x0111;
const LAYOUT: u16 = 0x0130;
const XTRANS_LAYOUT: u16 = 0x0131;
/// The exposure the camera held back from the raw, a signed ratio.
const RAW_EXPOSURE_BIAS: u16 = 0x9650;
/// Word sections' black and as-shot levels, green, red, green, blue.
const WORD_BLACK: u16 = 0x4000;
const WORD_AS_SHOT: u16 = 0x2FF0;
/// Present in files of the generation LibRaw sizes from its own table.
const RAF_DATA: u16 = 0xC000;
const LAYOUT_XTRANS: [u8; 4] = [0x0C; 4];
/// LibRaw's white for little-endian word sections; big-endian ones hold
/// 12-bit samples.
const WORD_WHITE: f64 = 15872.0;
const WORD_WHITE_12: f64 = 4095.0;

/// The sensor section's pointer to the raw directory, and that directory.
const RAW_IFD: u16 = 0xF000;
const RAW_WIDTH: u16 = 0xF001;
const RAW_HEIGHT: u16 = 0xF002;
const RAW_BITS: u16 = 0xF003;
const ROW_WORDS: u16 = 0xF005;
/// 1 when packed samples run most significant bit first.
const PACKING: u16 = 0xF006;
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

/// The sensor's photosite layout.
enum Layout {
    /// An upright CFA, measured from the sensor plane's origin.
    Cfa(CfaPattern),
    /// A SuperCCD lattice; `shift` is LibRaw's `fuji_layout`.
    SuperCcd { shift: bool },
}

/// The layout record `0x0130`. The X-Trans record lists the 6 × 6 pattern
/// from its last photosite back to its first, and an upright 2 × 2 record
/// does the same, each byte's low two bits naming the colour. LibRaw takes
/// a clear bit 3 in the second byte for a turned lattice.
fn layout(records: &[(u16, &[u8])]) -> Result<Layout, PsdError> {
    let layout = record(records, LAYOUT).ok_or_else(|| unsupported("without a sensor layout"))?;
    let &[first, second, ..] = layout else {
        return Err(invalid("layout record size"));
    };
    if layout == LAYOUT_XTRANS {
        let pattern = record(records, XTRANS_LAYOUT)
            .filter(|p| p.len() == 36)
            .ok_or_else(|| invalid("X-Trans layout"))?;
        let colors: Vec<u8> = pattern.iter().rev().copied().collect();
        if colors.iter().any(|&c| c > 2) || (0..3u8).any(|c| !colors.contains(&c)) {
            return Err(invalid("X-Trans layout colours"));
        }
        return Ok(Layout::Cfa(CfaPattern {
            width: 6,
            height: 6,
            colors,
        }));
    }
    if second & 8 == 0 {
        return Ok(Layout::SuperCcd {
            shift: first & 0x80 != 0,
        });
    }
    if layout.len() != 4 {
        return Err(unsupported("sensor layout"));
    }
    if record(records, XTRANS_LAYOUT).is_some() {
        return Err(unsupported("X-Trans pattern beside a 2 × 2 layout"));
    }
    let colors: Vec<u8> = layout
        .iter()
        .rev()
        .map(|b| match b & 3 {
            0 => 0,
            2 => 2,
            _ => 1,
        })
        .collect();
    if (0..3u8).any(|c| !colors.contains(&c)) {
        return Err(unsupported("sensor layout lacking a colour"));
    }
    Ok(Layout::Cfa(CfaPattern {
        width: 2,
        height: 2,
        colors,
    }))
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
    /// 12- or 14-bit samples, most significant bit first over little-endian
    /// 32-bit words, `stride` bytes a row.
    Msb { bits: u32, stride: usize },
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
    // Observed: 12-bit rows run least significant bit first unless
    // `0xF006` is 1 (the S1 and SL1000); 14-bit rows always run most
    // significant bit first.
    match (bits, raw.uint(PACKING)? == Some(1)) {
        (12, false) => Ok(Coding::Lsb12 { stride }),
        (12 | 14, _) => Ok(Coding::Msb { bits, stride }),
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

fn unpack_msb(src: &[u8], dst: &mut [u16], bits: u32) {
    let mask = (1u64 << bits) - 1;
    let (mut acc, mut have) = (0u64, 0u32);
    let mut words = src.as_chunks::<4>().0.iter();
    for d in dst {
        if have < bits {
            let w = words.next().map_or(0, |&w| u32::from_le_bytes(w));
            acc = acc << 32 | u64::from(w);
            have += 32;
        }
        have -= bits;
        *d = (acc >> have & mask) as u16;
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
            Coding::Msb { bits, stride } => unpack_msb(&data[y * stride..][..stride], dst, bits),
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

/// A word section's black by colour: red, green on red's rows, blue, green
/// on blue's rows, from `0x4000`'s green, red, green, blue, eight values
/// being two sets LibRaw averages.
fn word_black(records: &[(u16, &[u8])]) -> Result<[f64; 4], PsdError> {
    let Some(values) = record(records, WORD_BLACK) else {
        return Ok([0.0; 4]);
    };
    if values.len() != 8 && values.len() != 16 {
        return Err(invalid("black record size"));
    }
    let w: Vec<u32> = values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&b| u16::from_be_bytes(b).into())
        .collect();
    let grgb: [u32; 4] = std::array::from_fn(|c| match w.len() {
        8 => (w[c] + w[c + 4]) / 2,
        _ => w[c],
    });
    Ok([grgb[1], grgb[0], grgb[3], grgb[2]].map(f64::from))
}

/// A word section's as-shot neutral from `0x2FF0`'s green, red, green, blue.
fn word_neutral(records: &[(u16, &[u8])]) -> Option<[f64; 3]> {
    let v = record(records, WORD_AS_SHOT)?;
    let at = |i: usize| -> Option<f64> {
        Some(f64::from(u16::from_be_bytes([
            *v.get(2 * i)?,
            *v.get(2 * i + 1)?,
        ])))
    };
    let (g, r, b) = (at(0)?, at(1)?, at(3)?);
    (g > 0.0 && r > 0.0 && b > 0.0).then(|| [g / r, 1.0, g / b])
}

enum Black {
    /// One value, 2 × 2 or 6 × 6 values from the plane's origin, and the side.
    Positions(Vec<f64>, usize),
    /// Red, green on red's rows, blue, green on blue's rows (LibRaw's
    /// `cblack`).
    Colours([f64; 4]),
}

/// A decoded sensor plane and its levels.
struct Plane {
    width: usize,
    height: usize,
    samples: Vec<u16>,
    black: Black,
    white: f64,
    neutral: Option<[f64; 3]>,
}

fn levels(black: &Black, cfa: &CfaPattern, white: f64) -> Levels {
    let (black, rows, cols) = match black {
        Black::Positions(values, side) => (values.clone(), *side, *side),
        Black::Colours(by_colour) => (
            cfa.colors
                .chunks(cfa.width)
                .flat_map(|row| {
                    let blue_row = !row.contains(&0);
                    row.iter().map(move |&c| match c {
                        1 if blue_row => by_colour[3],
                        c => by_colour[usize::from(c)],
                    })
                })
                .collect(),
            cfa.height,
            cfa.width,
        ),
    };
    Levels {
        linearization: None,
        black,
        black_rows: rows,
        black_cols: cols,
        delta_v: Vec::new(),
        delta_h: Vec::new(),
        white,
    }
}

/// Charges a `width × height` plane of 16-bit samples, held twice while it
/// is normalized, against `budget`.
fn charge(width: usize, height: usize, budget: &ImageBudget) -> Result<(), PsdError> {
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
    Ok(())
}

/// The plane of a sensor section that is a TIFF, through its raw directory.
fn tiff_plane(
    sensor: &[u8],
    tiff: &Tiff,
    first: u64,
    cfa: &CfaPattern,
    budget: &ImageBudget,
) -> Result<Plane, PsdError> {
    let top: Ifd = tiff.ifd(first)?;
    let raw_at = top
        .get(RAW_IFD)
        .map(|e| tiff.uint(e))
        .transpose()?
        .ok_or_else(|| invalid("missing raw directory"))?;
    let raw_ifd = tiff.ifd(raw_at)?;
    let raw = Tags {
        tiff,
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
    charge(width, height, budget)?;
    let (samples, bits) = match compressed {
        Some(header) => (raf_compressed::decode(tail, &header, cfa)?, header.bits),
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
    Ok(Plane {
        width,
        height,
        samples,
        black: Black::Positions(black, side),
        white: ((1u32 << bits) - 1) as f64,
        neutral: as_shot_neutral(&raw)?,
    })
}

/// The plane of a sensor section of plain words: `0x0100`'s height by width
/// in the camera JPEG's byte order. With LibRaw's `fuji_layout` and a second
/// directory each row holds both frames, the primary first, and the data
/// outruns the section's recorded length, so it is bounded by the file.
fn word_plane(
    bytes: &[u8],
    records: &[(u16, &[u8])],
    two_frames: bool,
    big_endian: bool,
    budget: &ImageBudget,
) -> Result<Plane, PsdError> {
    let (height, width) = pair(records, FULL_SIZE)?.ok_or_else(|| invalid("missing raw size"))?;
    charge(width, height, budget)?;
    let stride = if two_frames { 2 * width } else { width };
    let data = bytes
        .get(be32(bytes, SENSOR_AT)?..)
        .and_then(|d| d.get(..stride * height * 2))
        .ok_or(PsdError::Truncated)?;
    let mut samples = vec![0u16; width * height];
    samples
        .par_chunks_mut(width)
        .zip(data.par_chunks(stride * 2))
        .for_each(|(dst, row)| {
            for (d, w) in dst.iter_mut().zip(row.as_chunks::<2>().0) {
                *d = if big_endian {
                    u16::from_be_bytes(*w)
                } else {
                    u16::from_le_bytes(*w)
                };
            }
        });
    Ok(Plane {
        width,
        height,
        samples,
        black: Black::Colours(word_black(records)?),
        white: if big_endian {
            WORD_WHITE_12
        } else {
            WORD_WHITE
        },
        neutral: word_neutral(records),
    })
}

/// Reads a RAF's sensor plane: an upright CFA normalized over the whole
/// plane, or a SuperCCD lattice repacked onto LibRaw's grid.
pub(super) fn read_raf(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
    if !is_raf(bytes) {
        return Err(invalid("not a Fujifilm raw file"));
    }
    if bytes.len() < HEADER_LEN {
        return Err(PsdError::Truncated);
    }
    let two_frames = be32(bytes, SECOND_FRAME_AT)? != 0;
    let records = records(bytes)?;
    let layout = layout(&records)?;
    let sensor = section(bytes, SENSOR_AT)?;
    let (make, model, orientation) = camera(bytes);
    let plane = match (Tiff::parse(sensor), &layout) {
        (Ok((tiff, first)), Layout::Cfa(cfa)) => tiff_plane(sensor, &tiff, first, cfa, budget)?,
        (Ok(_), Layout::SuperCcd { .. }) => {
            return Err(unsupported("SuperCCD layout in a TIFF sensor section"));
        }
        (Err(_), _) => {
            // LibRaw's `fuji_layout`, whatever the lattice.
            let shift = record(&records, LAYOUT).is_some_and(|l| l[0] & 0x80 != 0);
            let big_endian = exif(bytes).is_some_and(|e| e.starts_with(b"MM"));
            word_plane(bytes, &records, shift && two_frames, big_endian, budget)?
        }
    };
    let whole = |width, height| Area {
        top: 0,
        left: 0,
        width,
        height,
    };
    let (width, height) = (plane.width, plane.height);
    let (image_width, image_height, samples, cfa, fuji_width, default_crop, orientation) =
        match layout {
            Layout::Cfa(cfa) => {
                let levels = levels(&plane.black, &cfa, plane.white);
                let samples =
                    levels::normalize(&plane.samples, width, whole(width, height), &levels);
                let crop = default_crop(&records, (width, height))?;
                (width, height, samples, cfa, None, crop, orientation)
            }
            Layout::SuperCcd { shift } => {
                let inset = match (pair(&records, CROP_TOP_LEFT)?, pair(&records, CROP_SIZE)?) {
                    (Some((top, left)), Some((h, w))) => Some([top, left, h, w]),
                    _ => None,
                };
                let area = raf_superccd::active_area(
                    (width, height),
                    shift,
                    inset,
                    pair(&records, UPRIGHT_SIZE)?,
                    record(&records, RAF_DATA).is_some(),
                )?;
                let (fw, grid_width, grid_height) = raf_superccd::grid_size(&area, shift)?;
                charge(grid_width, grid_height, budget)?;
                let (upright_width, upright_height) =
                    super::demosaic::rotated_size(fw, grid_height)
                        .ok_or_else(|| invalid("SuperCCD grid size"))?;
                let grid = raf_superccd::repack(&plane.samples, (width, height), &area, shift)?;
                let cfa = raf_superccd::grid_cfa(fw);
                let levels = levels(&plane.black, &cfa, plane.white);
                let samples = levels::normalize(
                    &grid.samples,
                    grid.width,
                    whole(grid.width, grid.height),
                    &levels,
                );
                let crop = DngCrop {
                    x: 0.0,
                    y: 0.0,
                    width: upright_width as f64,
                    height: upright_height as f64,
                };
                let orientation = area.orientation.unwrap_or(orientation);
                (
                    grid.width,
                    grid.height,
                    samples,
                    cfa,
                    Some(grid.fuji_width),
                    crop,
                    orientation,
                )
            }
        };
    let mut color: DngColor =
        super::vendor::camera_color(make.as_deref(), model.as_deref(), plane.neutral);
    // The camera lifts its own JPEG by the bias; so does the develop.
    color.baseline_exposure = -exposure_bias(&records);
    Ok(DngImage {
        width: image_width,
        height: image_height,
        samples,
        cfa,
        fuji_width,
        default_crop,
        orientation,
        make,
        model,
        color,
    })
}
