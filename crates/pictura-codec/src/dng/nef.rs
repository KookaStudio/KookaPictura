//! Nikon NEF (#1469): a classic TIFF whose IFD0 names the CFA sensor image
//! among its SubIFDs, stored uncompressed or Huffman-coded (`nef_decode`),
//! with a Nikon maker note holding the levels, the linearization curve and
//! the as-shot white balance. Built from the TIFF, TIFF/EP and Exif
//! specifications, public maker-note tag tables and observation of real
//! files. The file has no colour matrix: colour comes from the fitted
//! per-camera table (#1465).

use super::levels::{self, Area, Levels};
use super::nef_decode::{
    decode_huffman, linearization, unpack_lsb, unpack_msb_le_words, unpack_padded,
};
use super::tiff::{Ifd, Order, Tiff};
use super::vendor::{camera_color, maker_note_bytes};
use super::{
    cfa_pattern, decode_plane, layout, to_usize, DngCrop, DngImage, Tags, BITS_PER_SAMPLE,
    COMPRESSION, DNG_VERSION, IMAGE_LENGTH, IMAGE_WIDTH, MAKE, NEW_SUBFILE_TYPE, ORIENTATION,
    PHOTOMETRIC, PHOTOMETRIC_CFA, ROWS_PER_STRIP, STRIP_BYTE_COUNTS, STRIP_OFFSETS, SUB_IFDS,
};
use crate::error::PsdError;
use crate::probe::ImageBudget;

const MODEL: u16 = 272;
const NIKON_CODED: u64 = 34713;
const WB_RB_LEVELS: u16 = 0x000C;
const BLACK_LEVEL: u16 = 0x003D;
const NEF_COMPRESSION: u16 = 0x0093;
const LINEARIZATION: u16 = 0x0096;
const COLOR_BALANCE: u16 = 0x0097;
const MAX_SUB_IFDS: usize = 16;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("NEF: {what}"))
}

fn unsupported(what: &str) -> PsdError {
    PsdError::Unsupported(format!("NEF {what}"))
}

/// Whether the TIFF whose IFD0 sits at `first` is a NEF: no `DNGVersion`,
/// `SubIFDs` listed, and a `Make` beginning with `NIKON`. Reads only IFD0's
/// entry table and its `Make` value.
pub(super) fn is_nef(tiff: &Tiff, first: u64) -> bool {
    !tiff.lists_tag(first, DNG_VERSION)
        && tiff.lists_tag(first, SUB_IFDS)
        && tiff
            .ascii_in(first, MAKE)
            .is_some_and(|m| m.to_uppercase().starts_with("NIKON"))
}

/// The Nikon maker note: `Nikon\0`, a version, two pad bytes, then a TIFF
/// whose offsets are relative to its own header at byte 10. A maker note
/// without the `Nikon\0` prefix counts as none; a prefixed one whose TIFF
/// does not parse refuses the file, as a damaged NEF.
fn nikon_note<'a>(ifd0: &Tags<'_, 'a>) -> Result<Option<(Tiff<'a>, Ifd)>, PsdError> {
    let Some(note) = maker_note_bytes(ifd0)? else {
        return Ok(None);
    };
    if note.len() < 18 || &note[..6] != b"Nikon\0" {
        return Ok(None);
    }
    let (tiff, first) = Tiff::parse(&note[10..])?;
    let ifd = tiff.ifd(first)?;
    Ok(Some((tiff, ifd)))
}

/// The first SubIFD of IFD0 holding the full-resolution CFA image.
fn sensor_ifd(ifd0: &Tags) -> Result<Ifd, PsdError> {
    let subs = ifd0.uints(SUB_IFDS)?.unwrap_or_default();
    for &offset in subs.iter().take(MAX_SUB_IFDS) {
        let ifd = ifd0.tiff.ifd(offset)?;
        let tags = Tags {
            tiff: ifd0.tiff,
            ifd: &ifd,
        };
        if tags.uint(NEW_SUBFILE_TYPE)?.unwrap_or(0) == 0
            && tags.uint(PHOTOMETRIC)? == Some(PHOTOMETRIC_CFA)
        {
            return Ok(ifd);
        }
    }
    Err(invalid("no CFA sensor image"))
}

/// The one strip holding a whole Nikon-coded plane.
fn strip<'a>(tags: &Tags<'_, 'a>) -> Result<&'a [u8], PsdError> {
    let first = |tag| -> Result<usize, PsdError> {
        let v = tags.uints(tag)?.and_then(|v| v.first().copied());
        to_usize(v.ok_or_else(|| invalid("missing strip"))?)
    };
    let (start, len) = (first(STRIP_OFFSETS)?, first(STRIP_BYTE_COUNTS)?);
    start
        .checked_add(len)
        .and_then(|end| tags.tiff.bytes().get(start..end))
        .ok_or(PsdError::Truncated)
}

/// Whether the first strip holds two bytes for each of its samples, so
/// samples sit in 16-bit containers whatever `BitsPerSample` says.
fn sixteen_bit_containers(tags: &Tags, width: usize, height: usize) -> Result<bool, PsdError> {
    let count = tags
        .uints(STRIP_BYTE_COUNTS)?
        .and_then(|v| v.first().copied());
    let rows = tags.uint(ROWS_PER_STRIP)?.unwrap_or(height as u64);
    let rows = to_usize(rows)?.min(height);
    Ok(count.is_some_and(|c| c as u128 >= (rows * width * 2) as u128))
}

/// The maker note's black levels, four 2 × 2 positions on a 14-bit scale,
/// brought to `bits`; zero without them.
fn black(note: Option<&Tags>, bits: u32) -> Result<Vec<f64>, PsdError> {
    let levels = note.map(|n| n.reals(BLACK_LEVEL)).transpose()?.flatten();
    Ok(match levels {
        Some(v) if v.len() >= 4 => {
            let scale = 2f64.powi(bits as i32 - 14);
            v[..4].iter().map(|b| b * scale).collect()
        }
        _ => vec![0.0; 4],
    })
}

/// The as-shot neutral from the maker note's red and blue multipliers
/// (`WB_RBLevels`), else from the unencrypted version-`0100` colour balance
/// (D100): 16-bit R, B, G, G levels at byte 72.
fn as_shot_neutral(note: Option<&Tags>) -> Result<Option<[f64; 3]>, PsdError> {
    let Some(note) = note else {
        return Ok(None);
    };
    if let Some([r, b, ..]) = note.reals(WB_RB_LEVELS)?.as_deref() {
        return Ok((*r > 0.0 && *b > 0.0).then(|| [1.0 / r, 1.0, 1.0 / b]));
    }
    let balance = note.ifd.get(COLOR_BALANCE).map(|e| note.tiff.raw(e));
    let Some(levels) = balance
        .filter(|b| b.starts_with(b"0100"))
        .and_then(|b| b.get(72..80))
    else {
        return Ok(None);
    };
    let [r, b, g1, g2] =
        [0, 1, 2, 3].map(|i| note.tiff.order.u16([levels[2 * i], levels[2 * i + 1]]) as f64);
    let g = (g1 + g2) / 2.0;
    Ok((r > 0.0 && b > 0.0 && g > 0.0).then(|| [g / r, 1.0, g / b]))
}

/// Reads a NEF's sensor plane, normalized; the whole plane is active.
// ponytail: white is the bit depth's ceiling, not the camera's saturation.
pub(super) fn read_nef(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
    let (tiff, first) = Tiff::parse(bytes)?;
    if !is_nef(&tiff, first) {
        return Err(invalid("not a Nikon raw file"));
    }
    let ifd0 = tiff.ifd(first)?;
    let ifd0 = Tags {
        tiff: &tiff,
        ifd: &ifd0,
    };
    let raw = sensor_ifd(&ifd0)?;
    let tags = Tags {
        tiff: &tiff,
        ifd: &raw,
    };
    let note = nikon_note(&ifd0)?;
    let note = note.as_ref().map(|(tiff, ifd)| Tags { tiff, ifd });

    match note
        .as_ref()
        .map(|n| n.uint(NEF_COMPRESSION))
        .transpose()?
        .flatten()
    {
        Some(8) => return Err(unsupported("small raw")),
        Some(13 | 14) => return Err(unsupported("high-efficiency compression")),
        _ => {}
    }
    let compression = tags.uint(COMPRESSION)?.unwrap_or(1);
    let bits = tags.uint(BITS_PER_SAMPLE)?.unwrap_or(0) as u32;
    if compression != 1 && compression != NIKON_CODED {
        return Err(unsupported(&format!("compression {compression}")));
    }
    if !(12..=16).contains(&bits) {
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
    let plane_bytes = (width * height * 2) as u64;
    if plane_bytes > budget.max_alloc_bytes || 2 * plane_bytes > budget.max_document_bytes {
        return Err(invalid("sensor plane exceeds the allocation budget"));
    }

    let table = note.as_ref().and_then(|n| {
        n.ifd
            .get(LINEARIZATION)
            .map(|e| (n.tiff.raw(e), n.tiff.order))
    });
    let plane = match (compression, table) {
        (NIKON_CODED, Some((table, order))) => {
            if bits != 12 && bits != 14 {
                return Err(unsupported(&format!("{bits}-bit Huffman data")));
            }
            let lin = linearization(table, order, bits)?;
            let data = strip(&tags)?;
            // Every sample costs at least two bits (the shortest code).
            if data.len() < (width * height).div_ceil(4) {
                return Err(PsdError::Truncated);
            }
            decode_huffman(data, &lin, bits, width, height)?
        }
        (NIKON_CODED, None) if !sixteen_bit_containers(&tags, width, height)? => {
            // The strip's exact size names its packing: LSB-first rows (D850
            // 12-bit), or 16-byte groups of ten MSB-first 12-bit samples and
            // a pad byte (D100). Anything else is refused, never guessed.
            let data = strip(&tags)?;
            if data.len() == (width * bits as usize).div_ceil(8) * height {
                unpack_lsb(data, bits, width, height)?
            } else if bits == 12 && data.len() == width.div_ceil(10) * 16 * height {
                unpack_padded(data, width, height)
            } else {
                return Err(unsupported("uncompressed sample layout"));
            }
        }
        _ if sixteen_bit_containers(&tags, width, height)? => {
            let layout = layout(&tags, width, height, budget)?;
            decode_plane(&tiff, &layout, 1, 16, (width, height), 1)?
        }
        // Packed samples run most-significant bit first through 32-bit words
        // in the file's byte order: plain packing in a big-endian file (D200),
        // word-swapped in a little-endian one (Coolpix P7100 NRW).
        _ if tiff.order == Order::Big => {
            let layout = layout(&tags, width, height, budget)?;
            decode_plane(&tiff, &layout, 1, bits, (width, height), 1)?
        }
        _ => {
            let rows = tags.uint(ROWS_PER_STRIP)?.unwrap_or(height as u64);
            if rows < height as u64 {
                return Err(unsupported("little-endian packed strips"));
            }
            unpack_msb_le_words(strip(&tags)?, bits, width, height)?
        }
    };

    let area = Area {
        top: 0,
        left: 0,
        width,
        height,
    };
    let levels = Levels {
        linearization: None,
        black: black(note.as_ref(), bits)?,
        black_rows: 2,
        black_cols: 2,
        delta_v: Vec::new(),
        delta_h: Vec::new(),
        white: ((1u32 << bits) - 1) as f64,
    };
    let samples = levels::normalize(&plane, width, area, &levels);
    let (make, model) = (ifd0.ascii(MAKE), ifd0.ascii(MODEL));
    Ok(DngImage {
        width,
        height,
        samples,
        cfa,
        default_crop: DngCrop {
            x: 0.0,
            y: 0.0,
            width: width as f64,
            height: height as f64,
        },
        orientation: ifd0.uint(ORIENTATION)?.unwrap_or(1).min(8) as u16,
        color: camera_color(
            make.as_deref(),
            model.as_deref(),
            as_shot_neutral(note.as_ref())?,
        ),
        make,
        model,
    })
}
