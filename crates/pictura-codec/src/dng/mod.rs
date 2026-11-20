//! In-house DNG reader (#396): the container, the DNG tags, and the
//! full-resolution CFA sensor plane normalized by its levels.
//!
//! Built from the DNG 1.6 specification and ITU T.81 only. Compression 1
//! (uncompressed) and 7 (lossless JPEG) are read; deflate, PackBits and lossy
//! JPEG, linear DNGs, floating-point samples and non-RGB CFAs are refused.
//! The demosaic (#397) and the colour transform (#398) consume the result.

use rayon::prelude::*;

use crate::error::PsdError;
use crate::probe::ImageBudget;

mod levels;
mod ljpeg;
mod tiff;

use levels::{Area, Levels};
use tiff::{Ifd, Order, Tiff};

const NEW_SUBFILE_TYPE: u16 = 254;
const IMAGE_WIDTH: u16 = 256;
const IMAGE_LENGTH: u16 = 257;
const BITS_PER_SAMPLE: u16 = 258;
const COMPRESSION: u16 = 259;
const PHOTOMETRIC: u16 = 262;
const MAKE: u16 = 271;
const STRIP_OFFSETS: u16 = 273;
const ORIENTATION: u16 = 274;
const SAMPLES_PER_PIXEL: u16 = 277;
const ROWS_PER_STRIP: u16 = 278;
const STRIP_BYTE_COUNTS: u16 = 279;
const TILE_WIDTH: u16 = 322;
const TILE_LENGTH: u16 = 323;
const TILE_OFFSETS: u16 = 324;
const TILE_BYTE_COUNTS: u16 = 325;
const SUB_IFDS: u16 = 330;
const SAMPLE_FORMAT: u16 = 339;
const CFA_REPEAT_PATTERN_DIM: u16 = 33421;
const CFA_PATTERN: u16 = 33422;
const DNG_VERSION: u16 = 50706;
const UNIQUE_CAMERA_MODEL: u16 = 50708;
const CFA_PLANE_COLOR: u16 = 50710;
const CFA_LAYOUT: u16 = 50711;
const LINEARIZATION_TABLE: u16 = 50712;
const BLACK_LEVEL_REPEAT_DIM: u16 = 50713;
const BLACK_LEVEL: u16 = 50714;
const BLACK_LEVEL_DELTA_H: u16 = 50715;
const BLACK_LEVEL_DELTA_V: u16 = 50716;
const WHITE_LEVEL: u16 = 50717;
const DEFAULT_CROP_ORIGIN: u16 = 50719;
const DEFAULT_CROP_SIZE: u16 = 50720;
const COLOR_MATRIX: [u16; 2] = [50721, 50722];
const CAMERA_CALIBRATION: [u16; 2] = [50723, 50724];
const ANALOG_BALANCE: u16 = 50727;
const AS_SHOT_NEUTRAL: u16 = 50728;
const BASELINE_EXPOSURE: u16 = 50730;
const CALIBRATION_ILLUMINANT: [u16; 2] = [50778, 50779];
const ACTIVE_AREA: u16 = 50829;
const FORWARD_MATRIX: [u16; 2] = [50964, 50965];

const PHOTOMETRIC_CFA: u64 = 32803;
const PHOTOMETRIC_LINEAR_RAW: u64 = 34892;
const MAX_IFDS: usize = 64;

/// The CFA repeat pattern, measured from the active area's top-left corner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfaPattern {
    pub width: usize,
    pub height: usize,
    /// `height × width` row-major, 0 red, 1 green, 2 blue.
    pub colors: Vec<u8>,
}

/// `DefaultCropOrigin` and `DefaultCropSize`, relative to the active area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DngCrop {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The colour tags, as stored; index 0 is the `…1` tag, 1 the `…2` tag.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DngColor {
    pub color_matrix: [Option<[f64; 9]>; 2],
    pub forward_matrix: [Option<[f64; 9]>; 2],
    pub camera_calibration: [Option<[f64; 9]>; 2],
    pub calibration_illuminant: [Option<u16>; 2],
    pub as_shot_neutral: Option<[f64; 3]>,
    pub analog_balance: Option<[f64; 3]>,
    pub baseline_exposure: f64,
}

/// A DNG's full-resolution CFA plane and the tags the later stages need.
#[derive(Debug, Clone)]
pub struct DngImage {
    pub width: usize,
    pub height: usize,
    /// One sample per photosite, row-major over the active area: black 0,
    /// white 65535.
    pub samples: Vec<u16>,
    pub cfa: CfaPattern,
    pub default_crop: DngCrop,
    pub orientation: u16,
    pub make: Option<String>,
    pub model: Option<String>,
    pub color: DngColor,
}

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("DNG: {what}"))
}

fn unsupported(what: &str) -> PsdError {
    PsdError::Unsupported(format!("DNG {what}"))
}

struct Tags<'t, 'a> {
    tiff: &'t Tiff<'a>,
    ifd: &'t Ifd,
}

impl Tags<'_, '_> {
    fn uint(&self, tag: u16) -> Result<Option<u64>, PsdError> {
        self.ifd.get(tag).map(|e| self.tiff.uint(e)).transpose()
    }

    fn uints(&self, tag: u16) -> Result<Option<Vec<u64>>, PsdError> {
        self.ifd.get(tag).map(|e| self.tiff.uints(e)).transpose()
    }

    fn reals(&self, tag: u16) -> Result<Option<Vec<f64>>, PsdError> {
        self.ifd.get(tag).map(|e| self.tiff.reals(e)).transpose()
    }

    fn fixed<const N: usize>(&self, tag: u16) -> Result<Option<[f64; N]>, PsdError> {
        self.reals(tag)?
            .map(|v| <[f64; N]>::try_from(v).map_err(|_| invalid(&format!("tag {tag} count"))))
            .transpose()
    }

    fn ascii(&self, tag: u16) -> Option<String> {
        self.ifd.get(tag).and_then(|e| self.tiff.ascii(e))
    }
}

fn to_usize(v: u64) -> Result<usize, PsdError> {
    usize::try_from(v).map_err(|_| invalid("value out of range"))
}

/// Finds the full-resolution image: IFD0, its SubIFDs, then the chain.
fn raw_ifd(tiff: &Tiff, first: u64) -> Result<Ifd, PsdError> {
    let mut queue = vec![first];
    let mut seen = Vec::new();
    while let Some(offset) = queue.pop() {
        if offset == 0 || seen.contains(&offset) {
            continue;
        }
        seen.push(offset);
        if seen.len() > MAX_IFDS {
            return Err(invalid("too many IFDs"));
        }
        let ifd = tiff.ifd(offset)?;
        let tags = Tags { tiff, ifd: &ifd };
        if tags.uint(NEW_SUBFILE_TYPE)?.unwrap_or(0) == 0 {
            return Ok(ifd);
        }
        queue.push(ifd.next);
        let mut subs = tags.uints(SUB_IFDS)?.unwrap_or_default();
        subs.reverse();
        queue.extend(subs);
    }
    Err(invalid("no full-resolution image"))
}

fn cfa_pattern(tags: &Tags) -> Result<CfaPattern, PsdError> {
    if tags.uint(CFA_LAYOUT)?.unwrap_or(1) != 1 {
        return Err(unsupported("non-rectangular CFA layout"));
    }
    let dim = tags
        .uints(CFA_REPEAT_PATTERN_DIM)?
        .ok_or_else(|| invalid("missing CFARepeatPatternDim"))?;
    let (height, width) = match dim[..] {
        [h, w] if (1..=16).contains(&h) && (1..=16).contains(&w) => (h as usize, w as usize),
        _ => return Err(invalid("CFARepeatPatternDim")),
    };
    let pattern = tags
        .uints(CFA_PATTERN)?
        .ok_or_else(|| invalid("missing CFAPattern"))?;
    if pattern.len() != width * height {
        return Err(invalid("CFAPattern count"));
    }
    let planes = tags.uints(CFA_PLANE_COLOR)?.unwrap_or(vec![0, 1, 2]);
    let mut sorted = planes.clone();
    sorted.sort_unstable();
    if sorted != [0, 1, 2] {
        return Err(unsupported(
            "CFA with colours other than red, green and blue",
        ));
    }
    let colors = pattern
        .iter()
        .map(|&i| planes.get(i as usize).map(|&c| c as u8))
        .collect::<Option<Vec<u8>>>()
        .ok_or_else(|| invalid("CFAPattern plane index"))?;
    Ok(CfaPattern {
        width,
        height,
        colors,
    })
}

/// The strip or tile grid: every chunk is `tile_w × tile_h`, strips being
/// full-width tiles.
struct Layout {
    tile_w: usize,
    tile_h: usize,
    across: usize,
    offsets: Vec<u64>,
    counts: Vec<u64>,
}

fn layout(
    tags: &Tags,
    width: usize,
    height: usize,
    budget: &ImageBudget,
) -> Result<Layout, PsdError> {
    let (tile_w, tile_h, offsets, counts) = match tags.uint(TILE_WIDTH)? {
        Some(tw) => (
            to_usize(tw)?,
            to_usize(
                tags.uint(TILE_LENGTH)?
                    .ok_or_else(|| invalid("missing TileLength"))?,
            )?,
            tags.uints(TILE_OFFSETS)?,
            tags.uints(TILE_BYTE_COUNTS)?,
        ),
        None => (
            width,
            to_usize(tags.uint(ROWS_PER_STRIP)?.unwrap_or(height as u64))?.min(height),
            tags.uints(STRIP_OFFSETS)?,
            tags.uints(STRIP_BYTE_COUNTS)?,
        ),
    };
    let max = budget.max_dimension as usize;
    if tile_w == 0 || tile_h == 0 || tile_w > max || tile_h > max {
        return Err(invalid("tile size"));
    }
    let across = width.div_ceil(tile_w);
    let chunks = across * height.div_ceil(tile_h);
    let (offsets, counts) = offsets
        .zip(counts)
        .ok_or_else(|| invalid("missing image data offsets"))?;
    if offsets.len() < chunks || counts.len() < chunks {
        return Err(invalid("too few strips or tiles"));
    }
    Ok(Layout {
        tile_w,
        tile_h,
        across,
        offsets,
        counts,
    })
}

fn chunk<'a>(bytes: &'a [u8], layout: &Layout, i: usize) -> Result<&'a [u8], PsdError> {
    let start = to_usize(layout.offsets[i])?;
    let len = to_usize(layout.counts[i])?;
    start
        .checked_add(len)
        .and_then(|end| bytes.get(start..end))
        .ok_or(PsdError::Truncated)
}

/// Unpacks `rows × cols` samples of one uncompressed chunk into `put`.
fn unpack(
    data: &[u8],
    bits: u32,
    order: Order,
    tile_w: usize,
    rows: usize,
    cols: usize,
    mut put: impl FnMut(usize, usize, u16),
) -> Result<(), PsdError> {
    let row_bytes = (tile_w * bits as usize).div_ceil(8);
    if data.len() < row_bytes * rows {
        return Err(PsdError::Truncated);
    }
    for r in 0..rows {
        let src = &data[r * row_bytes..][..row_bytes];
        match bits {
            8 => (0..cols).for_each(|c| put(r, c, src[c] as u16)),
            16 => (0..cols).for_each(|c| put(r, c, order.u16([src[2 * c], src[2 * c + 1]]))),
            _ => {
                let mut acc = 0u32;
                let mut have = 0u32;
                let mut p = 0;
                for c in 0..cols {
                    while have < bits {
                        acc = (acc << 8) | src[p] as u32;
                        p += 1;
                        have += 8;
                    }
                    have -= bits;
                    put(r, c, ((acc >> have) & ((1 << bits) - 1)) as u16);
                }
            }
        }
    }
    Ok(())
}

/// Decodes every strip or tile into a `width × height` raw plane, one band
/// of tile rows per rayon task.
fn decode_plane(
    tiff: &Tiff,
    layout: &Layout,
    compression: u64,
    bits: u32,
    width: usize,
    height: usize,
) -> Result<Vec<u16>, PsdError> {
    let bytes = tiff.bytes();
    let (tw, th) = (layout.tile_w, layout.tile_h);
    // Every chunk must hold its samples before the plane is allocated: a row
    // of uncompressed bytes per row, or at least one bit per sample (the
    // shortest Huffman code) for lossless JPEG.
    for i in 0..layout.across * height.div_ceil(th) {
        let data = chunk(bytes, layout, i)?;
        let rows = th.min(height - (i / layout.across) * th);
        let need = match compression {
            1 => (tw * bits as usize).div_ceil(8) * rows,
            _ => (tw.min(width - (i % layout.across) * tw) * rows).div_ceil(8),
        };
        if data.len() < need {
            return Err(PsdError::Truncated);
        }
    }
    let mut raw = vec![0u16; width * height];
    raw.par_chunks_mut(th * width)
        .enumerate()
        .try_for_each(|(band, dst)| {
            let rows = dst.len() / width;
            for t in 0..layout.across {
                let data = chunk(bytes, layout, band * layout.across + t)?;
                let x0 = t * tw;
                let cols = tw.min(width - x0);
                let mut put = |r: usize, c: usize, v: u16| {
                    if r < rows && c < cols {
                        dst[r * width + x0 + c] = v;
                    }
                };
                match compression {
                    1 => unpack(data, bits, tiff.order, tw, rows.min(th), cols, put)?,
                    7 => ljpeg::decode(data, tw * th, |first, samples| {
                        for (k, &v) in samples.iter().enumerate() {
                            let i = first + k;
                            put(i / tw, i % tw, v);
                        }
                    })?,
                    _ => unreachable!(),
                }
            }
            Ok::<(), PsdError>(())
        })?;
    Ok(raw)
}

fn active_area(tags: &Tags, width: usize, height: usize) -> Result<Area, PsdError> {
    let Some(a) = tags.uints(ACTIVE_AREA)? else {
        return Ok(Area {
            top: 0,
            left: 0,
            width,
            height,
        });
    };
    let [top, left, bottom, right] = a[..] else {
        return Err(invalid("ActiveArea count"));
    };
    let [top, left, bottom, right] = [top, left, bottom, right].map(|v| v as usize);
    if top >= bottom || left >= right || bottom > height || right > width {
        return Err(invalid("ActiveArea outside the image"));
    }
    Ok(Area {
        top,
        left,
        width: right - left,
        height: bottom - top,
    })
}

fn levels(tags: &Tags, bits: u32, area: Area) -> Result<Levels, PsdError> {
    let linearization = tags
        .uints(LINEARIZATION_TABLE)?
        .filter(|t| !t.is_empty())
        .map(|t| t.into_iter().map(|v| v.min(65535) as u16).collect());
    let (black_rows, black_cols) = match tags.uints(BLACK_LEVEL_REPEAT_DIM)?.as_deref() {
        None => (1, 1),
        Some(&[r, c]) if (1..=16).contains(&r) && (1..=16).contains(&c) => (r as usize, c as usize),
        Some(_) => return Err(invalid("BlackLevelRepeatDim")),
    };
    let black = match tags.reals(BLACK_LEVEL)? {
        None => vec![0.0; black_rows * black_cols],
        Some(v) if v.len() == black_rows * black_cols => v,
        Some(v) if v.len() == 1 => vec![v[0]; black_rows * black_cols],
        Some(_) => return Err(invalid("BlackLevel count")),
    };
    let delta = |tag, len| -> Result<Vec<f64>, PsdError> {
        match tags.reals(tag)? {
            None => Ok(Vec::new()),
            Some(v) if v.len() >= len => Ok(v),
            Some(_) => Err(invalid(&format!("tag {tag} count"))),
        }
    };
    let white = match tags.reals(WHITE_LEVEL)? {
        Some(v) => *v.first().ok_or_else(|| invalid("WhiteLevel"))?,
        None => ((1u32 << bits) - 1) as f64,
    };
    Ok(Levels {
        linearization,
        black,
        black_rows,
        black_cols,
        delta_v: delta(BLACK_LEVEL_DELTA_V, area.height)?,
        delta_h: delta(BLACK_LEVEL_DELTA_H, area.width)?,
        white,
    })
}

fn color(ifd0: &Tags) -> Result<DngColor, PsdError> {
    let pair = |tags: [u16; 2]| -> Result<[Option<[f64; 9]>; 2], PsdError> {
        Ok([ifd0.fixed(tags[0])?, ifd0.fixed(tags[1])?])
    };
    let illuminant = |tag| -> Result<Option<u16>, PsdError> {
        Ok(ifd0.uint(tag)?.map(|v| v.min(u16::MAX as u64) as u16))
    };
    Ok(DngColor {
        color_matrix: pair(COLOR_MATRIX)?,
        forward_matrix: pair(FORWARD_MATRIX)?,
        camera_calibration: pair(CAMERA_CALIBRATION)?,
        calibration_illuminant: [
            illuminant(CALIBRATION_ILLUMINANT[0])?,
            illuminant(CALIBRATION_ILLUMINANT[1])?,
        ],
        as_shot_neutral: ifd0.fixed(AS_SHOT_NEUTRAL)?,
        analog_balance: ifd0.fixed(ANALOG_BALANCE)?,
        baseline_exposure: ifd0
            .reals(BASELINE_EXPOSURE)?
            .and_then(|v| v.first().copied())
            .unwrap_or(0.0),
    })
}

fn default_crop(tags: &Tags, area: Area) -> Result<DngCrop, PsdError> {
    let origin = tags.fixed::<2>(DEFAULT_CROP_ORIGIN)?.unwrap_or([0.0, 0.0]);
    let size = tags
        .fixed::<2>(DEFAULT_CROP_SIZE)?
        .unwrap_or([area.width as f64, area.height as f64]);
    Ok(DngCrop {
        x: origin[0],
        y: origin[1],
        width: size[0],
        height: size[1],
    })
}

/// Reads a DNG's full-resolution CFA plane, normalized and cropped to its
/// active area. Declared sizes are charged against `budget` first.
pub fn read_dng(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
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

    match tags.uint(PHOTOMETRIC)? {
        Some(PHOTOMETRIC_CFA) => {}
        Some(PHOTOMETRIC_LINEAR_RAW) => return Err(unsupported("linear raw image")),
        other => {
            return Err(unsupported(&format!(
                "photometric interpretation {other:?}"
            )))
        }
    }
    if tags.uint(SAMPLES_PER_PIXEL)?.unwrap_or(1) != 1 {
        return Err(unsupported("CFA with several samples per pixel"));
    }
    if tags.uint(SAMPLE_FORMAT)?.unwrap_or(1) != 1 {
        return Err(unsupported("floating-point or signed samples"));
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
    let cfa = cfa_pattern(&tags)?;

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
    let plane = (width * height * 2) as u64;
    let total = plane + (area.width * area.height * 2) as u64;
    if plane > budget.max_alloc_bytes || total > budget.max_document_bytes {
        return Err(invalid("sensor plane exceeds the allocation budget"));
    }
    let layout = layout(&tags, width, height, budget)?;
    let levels = levels(&tags, bits, area)?;
    let default_crop = default_crop(&tags, area)?;
    let color = color(&ifd0)?;

    let raw_plane = decode_plane(&tiff, &layout, compression, bits, width, height)?;
    let samples = levels::normalize(&raw_plane, width, area, &levels);
    Ok(DngImage {
        width: area.width,
        height: area.height,
        samples,
        cfa,
        default_crop,
        orientation: ifd0.uint(ORIENTATION)?.unwrap_or(1).min(8) as u16,
        make: ifd0.ascii(MAKE),
        model: ifd0.ascii(UNIQUE_CAMERA_MODEL),
        color,
    })
}
