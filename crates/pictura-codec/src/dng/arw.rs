//! Sony ARW (#1470): a little-endian TIFF whose IFD0 names the CFA sensor
//! image among its SubIFDs, stored uncompressed in 16-bit words, ARW2
//! block-compressed through a tone curve (`arw_decode`), or as tiles of
//! lossless JPEG. The levels and as-shot white balance sit in the raw IFD on
//! recent bodies and otherwise only in the enciphered SR2 private block.
//! Built from the TIFF, TIFF/EP and DNG specifications, public descriptions
//! of the Sony tags (ExifTool's documentation) and observation of real files.
//! The file has no usable colour matrix: colour comes from the fitted
//! per-camera table (#1465).

use super::arw_decode::{decipher_sr2, decode_arw2_row, tone_curve, ARW2_GROUP};
use super::levels::{self, Area, Levels};
use super::tiff::{Order, Tiff};
use super::vendor::camera_color;
use super::{
    cfa_pattern, chunk, decode_plane, default_crop, layout, ljpeg, raw_ifd, to_usize, CfaPattern,
    DngImage, Layout, Tags, BITS_PER_SAMPLE, CFA_PATTERN, COMPRESSION, DNG_VERSION, IMAGE_LENGTH,
    IMAGE_WIDTH, MAKE, ORIENTATION, PHOTOMETRIC, PHOTOMETRIC_CFA, SAMPLES_PER_PIXEL,
    STRIP_BYTE_COUNTS, STRIP_OFFSETS,
};
use rayon::prelude::*;

use crate::error::PsdError;
use crate::probe::ImageBudget;

const MODEL: u16 = 272;
const DNG_PRIVATE_DATA: u16 = 0xC634;
const SONY_TONE_CURVE: u16 = 0x7010;
const SONY_RAW_IMAGE_SIZE: u16 = 0x7038;
const BLACK_LEVEL_OLD: u16 = 0x7300;
const WB_GRBG_LEVELS: u16 = 0x7303;
const BLACK_LEVEL: u16 = 0x7310;
const WB_RGGB_LEVELS: u16 = 0x7313;
const WHITE_LEVEL_SR2: u16 = 0x787F;
const WHITE_LEVEL: u16 = 0xC61D;
const SR2_SUB_IFD_OFFSET: u16 = 0x7200;
const SR2_SUB_IFD_LENGTH: u16 = 0x7201;
const SR2_SUB_IFD_KEY: u16 = 0x7221;
const UNCOMPRESSED: u64 = 1;
const LOSSLESS_JPEG: u64 = 7;
const SONY_COMPRESSED: u64 = 32767;
/// The SR2 block sits near the start of the file, after the previews'
/// directories; a larger end is refused rather than copied.
const MAX_SR2_END: usize = 4 << 20;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("ARW: {what}"))
}

fn unsupported(what: &str) -> PsdError {
    PsdError::Unsupported(format!("ARW {what}"))
}

/// Whether the TIFF whose IFD0 sits at `first` is a Sony raw file: no
/// `DNGVersion` and a `Make` beginning with `SONY`. Reads only IFD0's entry
/// table and its `Make` value, so variants the reader refuses still reach it
/// and are refused by name.
pub(super) fn is_arw(tiff: &Tiff, first: u64) -> bool {
    !tiff.lists_tag(first, DNG_VERSION)
        && tiff
            .ascii_in(first, MAKE)
            .is_some_and(|m| m.to_uppercase().starts_with("SONY"))
}

/// The levels and white balance a Sony tag directory records: black per 2 × 2
/// position, white, and the as-shot RGGB multipliers.
#[derive(Default)]
struct SonyLevels {
    black: Option<Vec<f64>>,
    white: Option<f64>,
    wb_rggb: Option<[f64; 4]>,
}

impl SonyLevels {
    fn read(tags: &Tags) -> Result<Self, PsdError> {
        let four = |tag| -> Result<Option<[f64; 4]>, PsdError> {
            Ok(tags.reals(tag)?.and_then(|v| <[f64; 4]>::try_from(v).ok()))
        };
        let black = match four(BLACK_LEVEL)? {
            Some(b) => Some(b),
            None => four(BLACK_LEVEL_OLD)?,
        };
        let wb_rggb = match four(WB_RGGB_LEVELS)? {
            Some(wb) => Some(wb),
            None => four(WB_GRBG_LEVELS)?.map(|[g1, r, b, g2]| [r, g1, g2, b]),
        };
        let white = match tags.reals(WHITE_LEVEL)?.as_deref() {
            Some([w, ..]) => Some(*w),
            _ => tags
                .reals(WHITE_LEVEL_SR2)?
                .and_then(|v| v.into_iter().reduce(f64::min)),
        };
        Ok(Self {
            black: black.map(Vec::from),
            white,
            wb_rggb,
        })
    }

    /// `self`, with what it lacks taken from `other`.
    fn or(self, other: Self) -> Self {
        Self {
            black: self.black.or(other.black),
            white: self.white.or(other.white),
            wb_rggb: self.wb_rggb.or(other.wb_rggb),
        }
    }
}

/// A 32-bit value stored as four bytes (or one LONG) in file byte order.
fn u32_value(tags: &Tags, tag: u16) -> Result<Option<u32>, PsdError> {
    let Some(entry) = tags.ifd.get(tag) else {
        return Ok(None);
    };
    let b: [u8; 4] = tags
        .tiff
        .raw(entry)
        .try_into()
        .map_err(|_| invalid(&format!("tag {tag:#06x} size")))?;
    Ok(Some(match tags.tiff.order {
        Order::Little => u32::from_le_bytes(b),
        Order::Big => u32::from_be_bytes(b),
    }))
}

/// The levels of the SR2 private block that `DNGPrivateData` in IFD0 points
/// at: its SR2SubIFD, deciphered with the block's key, is a directory whose
/// offsets count from the file start. Without the block, none.
fn sr2_levels(tiff: &Tiff, ifd0: &Tags) -> Result<SonyLevels, PsdError> {
    let Some(private) = u32_value(ifd0, DNG_PRIVATE_DATA)? else {
        return Ok(SonyLevels::default());
    };
    let private = tiff.ifd(private as u64)?;
    let private = Tags {
        tiff,
        ifd: &private,
    };
    let (Some(at), Some(len), Some(key)) = (
        private.uint(SR2_SUB_IFD_OFFSET)?,
        private.uint(SR2_SUB_IFD_LENGTH)?,
        u32_value(&private, SR2_SUB_IFD_KEY)?,
    ) else {
        return Ok(SonyLevels::default());
    };
    let (at, len) = (to_usize(at)?, to_usize(len)?);
    let end = at
        .checked_add(len)
        .filter(|&end| end <= MAX_SR2_END)
        .ok_or_else(|| invalid("SR2 block outside the file start"))?;
    let bytes = tiff.bytes().get(..end).ok_or(PsdError::Truncated)?;
    let mut plain = bytes.to_vec();
    decipher_sr2(&mut plain[at..], key);
    let (sr2, _) = Tiff::parse(&plain)?;
    let sub = sr2.ifd(at as u64)?;
    SonyLevels::read(&Tags {
        tiff: &sr2,
        ifd: &sub,
    })
}

/// The ARW2 codes of `data` (one byte per photosite), row by row.
fn decode_arw2(data: &[u8], width: usize, height: usize) -> Result<Vec<u16>, PsdError> {
    if !width.is_multiple_of(ARW2_GROUP) {
        return Err(invalid("compressed width is not a multiple of 32"));
    }
    let mut codes = vec![0u16; width * height];
    codes
        .par_chunks_mut(width)
        .zip(data.par_chunks(width))
        .for_each(|(dst, src)| decode_arw2_row(src, dst));
    Ok(codes)
}

/// Decodes lossless JPEG tiles whose frames are half the tile's width and
/// height in four components: each frame sample is one 2 × 2 quad of
/// photosites, its components row-major (observed on the ILCE-1 and
/// ILCE-7M4: placing them as consecutive sensor rows scrambles the CFA).
fn decode_quad_tiles(
    tiff: &Tiff,
    layout: &Layout,
    (width, height): (usize, usize),
) -> Result<Vec<u16>, PsdError> {
    let (tw, th) = (layout.tile_w, layout.tile_h);
    let tiles = layout.across * height.div_ceil(th);
    for i in 0..tiles {
        let data = chunk(tiff.bytes(), layout, i)?;
        let (_, fw, fh, components) = ljpeg::frame(data)?;
        if components != 4 || fw * 2 != tw || fh * 2 != th {
            return Err(unsupported("lossless tile layout"));
        }
        // Every sample costs at least one bit (the shortest Huffman code).
        if data.len() < (tw * th).div_ceil(8) {
            return Err(PsdError::Truncated);
        }
    }
    let row = tw * 2;
    let mut plane = vec![0u16; width * height];
    plane
        .par_chunks_mut(th * width)
        .enumerate()
        .try_for_each(|(band, dst)| {
            let rows = dst.len() / width;
            for t in 0..layout.across {
                let data = chunk(tiff.bytes(), layout, band * layout.across + t)?;
                let x0 = t * tw;
                ljpeg::decode(data, tw * th, |first, samples| {
                    for (k, &v) in samples.iter().enumerate() {
                        let (r, q) = ((first + k) / row, (first + k) % row);
                        let (y, x) = (2 * r + (q % 4) / 2, x0 + 2 * (q / 4) + q % 2);
                        if y < rows && x < width {
                            dst[y * width + x] = v;
                        }
                    }
                })?;
            }
            Ok::<(), PsdError>(())
        })?;
    Ok(plane)
}

/// The one strip of an unstriped image, at least `need` bytes long.
fn strip<'a>(bytes: &'a [u8], tags: &Tags, need: usize) -> Result<&'a [u8], PsdError> {
    let start = to_usize(
        tags.uint(STRIP_OFFSETS)?
            .ok_or_else(|| invalid("missing StripOffsets"))?,
    )?;
    let len = to_usize(
        tags.uint(STRIP_BYTE_COUNTS)?
            .ok_or_else(|| invalid("missing StripByteCounts"))?,
    )?;
    if len < need {
        return Err(PsdError::Truncated);
    }
    start
        .checked_add(need)
        .and_then(|end| bytes.get(start..end))
        .ok_or(PsdError::Truncated)
}

/// The active area: the sensor image size of tag `0x7038` inside a tiled
/// plane padded to whole tiles, else the whole plane.
fn active_area(tags: &Tags, width: usize, height: usize) -> Result<Area, PsdError> {
    let (w, h) = match tags.uints(SONY_RAW_IMAGE_SIZE)?.as_deref() {
        Some(&[w, h]) => (to_usize(w)?, to_usize(h)?),
        _ => (width, height),
    };
    if w == 0 || h == 0 || w > width || h > height {
        return Err(invalid("sensor image size outside the plane"));
    }
    Ok(Area {
        top: 0,
        left: 0,
        width: w,
        height: h,
    })
}

/// Reads an ARW's sensor plane, normalized and cropped to its active area.
// ponytail: the black is one value per CFA position, and a file without
// white level tags takes the sample precision's ceiling.
pub(super) fn read_arw(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
    let (tiff, first) = Tiff::parse(bytes)?;
    if !is_arw(&tiff, first) {
        return Err(invalid("not a Sony raw file"));
    }
    let ifd0 = tiff.ifd(first)?;
    let ifd0 = Tags {
        tiff: &tiff,
        ifd: &ifd0,
    };
    // ARW 1.0 (DSLR-A100) points IFD0's SubIFDs at the sensor data itself.
    let raw = raw_ifd(&tiff, first).map_err(|_| unsupported("1.0 layout"))?;
    let tags = Tags {
        tiff: &tiff,
        ifd: &raw,
    };
    match tags.uint(PHOTOMETRIC)? {
        None => return Err(unsupported("1.0 layout")),
        Some(PHOTOMETRIC_CFA) if tags.uint(SAMPLES_PER_PIXEL)?.unwrap_or(1) == 1 => {}
        Some(_) => return Err(unsupported("without a CFA image (small or medium raw)")),
    }
    let compression = tags.uint(COMPRESSION)?.unwrap_or(UNCOMPRESSED);
    let bits = tags.uint(BITS_PER_SAMPLE)?.unwrap_or(16) as u32;
    if !(8..=16).contains(&bits) {
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
    let plane_bytes = (width * height * 2) as u64;
    let total = plane_bytes + (area.width * area.height * 2) as u64;
    if plane_bytes > budget.max_alloc_bytes || total > budget.max_document_bytes {
        return Err(invalid("sensor plane exceeds the allocation budget"));
    }
    let cfa = if tags.ifd.get(CFA_PATTERN).is_some() {
        cfa_pattern(&tags)?
    } else {
        CfaPattern {
            width: 2,
            height: 2,
            colors: vec![0, 1, 1, 2],
        }
    };
    if cfa.width != 2 || cfa.height != 2 {
        return Err(unsupported("CFA other than 2 × 2"));
    }

    let (plane, linearization, ceiling) = match compression {
        UNCOMPRESSED => {
            // Every body observed stores one sample per 16-bit word.
            let stored: u64 = tags
                .uints(STRIP_BYTE_COUNTS)?
                .unwrap_or_default()
                .iter()
                .sum();
            if stored < (width * height * 2) as u64 {
                return Err(unsupported("packed uncompressed samples"));
            }
            let layout = layout(&tags, width, height, budget)?;
            let plane = decode_plane(&tiff, &layout, UNCOMPRESSED, 16, (width, height), 1)?;
            (plane, None, (1u32 << bits) - 1)
        }
        SONY_COMPRESSED => {
            // ARW2 codes one photosite per byte; ARW 1.0 (DSLR-A100) packs
            // its curve-coded rows tighter.
            let len = tags.uint(STRIP_BYTE_COUNTS)?.unwrap_or(0);
            if len < (width * height) as u64 {
                return Err(unsupported("1.0 compression"));
            }
            let data = strip(bytes, &tags, width * height)?;
            let knees = tags
                .uints(SONY_TONE_CURVE)?
                .and_then(|v| <[u64; 4]>::try_from(v).ok())
                .ok_or_else(|| invalid("missing tone curve"))?;
            let curve = tone_curve(knees.map(|k| k.min(u16::MAX as u64) as u16));
            let ceiling = *curve.last().unwrap_or(&0) as u32;
            (decode_arw2(data, width, height)?, Some(curve), ceiling)
        }
        LOSSLESS_JPEG => {
            let layout = layout(&tags, width, height, budget)?;
            let plane = decode_quad_tiles(&tiff, &layout, (width, height))?;
            (plane, None, (1u32 << bits) - 1)
        }
        other => return Err(unsupported(&format!("compression {other}"))),
    };

    let sony = SonyLevels::read(&tags)?.or(sr2_levels(&tiff, &ifd0)?);
    let levels = Levels {
        linearization,
        black: sony.black.unwrap_or(vec![0.0; 4]),
        black_rows: 2,
        black_cols: 2,
        delta_v: Vec::new(),
        delta_h: Vec::new(),
        white: sony.white.unwrap_or(ceiling as f64),
    };
    let samples = levels::normalize(&plane, width, area, &levels);
    let as_shot_neutral = sony.wb_rggb.and_then(|[r, g1, g2, b]| {
        let g = (g1 + g2) / 2.0;
        (r > 0.0 && g > 0.0 && b > 0.0).then_some([g / r, 1.0, g / b])
    });
    let (make, model) = (ifd0.ascii(MAKE), ifd0.ascii(MODEL));
    Ok(DngImage {
        width: area.width,
        height: area.height,
        samples,
        cfa,
        fuji_width: None,
        default_crop: default_crop(&tags, area)?,
        orientation: ifd0.uint(ORIENTATION)?.unwrap_or(1).min(8) as u16,
        color: camera_color(make.as_deref(), model.as_deref(), as_shot_neutral),
        make,
        model,
    })
}
