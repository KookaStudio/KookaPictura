//! Olympus and OM System ORF/ORI (#1471): a TIFF with its own magic whose
//! IFD0 is the sensor image, stored in 16-bit words or in one of three 12-bit
//! packings, or Olympus-compressed, which is refused (#1535). The directory
//! calls all of them uncompressed, so the strips' size tells them apart. Built
//! from the TIFF specification and observation of real files; the maker note
//! is `orf_meta`'s.

use rayon::prelude::*;

use super::levels::{self, Area, Levels};
use super::orf_meta::OrfMeta;
use super::tiff::{Order, Tiff};
use super::{
    decode_plane, layout, to_usize, CfaPattern, DngCrop, DngImage, Tags, IMAGE_LENGTH, IMAGE_WIDTH,
    MAKE, ORIENTATION, STRIP_BYTE_COUNTS, STRIP_OFFSETS,
};
use crate::error::PsdError;
use crate::probe::ImageBudget;

const MODEL: u16 = 272;
const EXIF_IFD: u16 = 34665;
const EXIF_CFA_PATTERN: u16 = 0xA302;
const MAGICS: [&[u8; 4]; 3] = [b"IIRO", b"IIRS", b"MMOR"];
const HEADER_LEN: usize = 8;

type UnpackRow = fn(&[u8], &mut [u16]);

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("ORF: {what}"))
}

fn unsupported(what: &str) -> PsdError {
    PsdError::Unsupported(format!("ORF {what}"))
}

/// Whether `bytes` begin with one of the three ORF headers.
pub(super) fn is_orf(bytes: &[u8]) -> bool {
    bytes.len() >= HEADER_LEN && MAGICS.iter().any(|m| bytes.starts_with(*m))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Coding {
    /// Left-aligned 16-bit words in file order.
    Words,
    /// Ten samples, least significant bit first, and a pad byte in 16 bytes.
    PaddedLsb,
    /// Most significant bit first over little-endian 32-bit words.
    MsbWords,
    /// Most significant bit first over bytes, in two fields: the even rows,
    /// then the odd.
    MsbFields,
}

/// The coding of `stored` bytes in `strips` strips of a `width × height`
/// plane.
fn coding(
    bytes: &[u8],
    stored: u64,
    strips: usize,
    (width, height): (usize, usize),
) -> Result<Coding, PsdError> {
    let n = (width * height) as u64;
    if stored == 2 * n {
        Ok(Coding::Words)
    } else if stored * 5 == 8 * n && width.is_multiple_of(10) {
        Ok(Coding::PaddedLsb)
    } else if stored * 2 == 3 * n && width.is_multiple_of(2) {
        if bytes.starts_with(b"IIRS") {
            Ok(Coding::MsbFields)
        } else if !bytes.starts_with(b"IIRO") {
            Err(unsupported("packed sensor data under this header"))
        } else if width.is_multiple_of(8) {
            Ok(Coding::MsbWords)
        } else {
            Err(unsupported("packed width"))
        }
    } else if strips == 1 && stored * 2 < 3 * n {
        Err(unsupported("compressed sensor data is not supported yet"))
    } else {
        Err(unsupported("sensor data layout"))
    }
}

/// One row of `PaddedLsb`.
fn unpack_padded(src: &[u8], dst: &mut [u16]) {
    for (group, out) in src
        .as_chunks::<16>()
        .0
        .iter()
        .zip(dst.as_chunks_mut::<10>().0)
    {
        for (b, pair) in group
            .as_chunks::<3>()
            .0
            .iter()
            .zip(out.as_chunks_mut::<2>().0)
        {
            let [a, m, c] = [b[0], b[1], b[2]].map(u16::from);
            pair[0] = a | (m & 0xF) << 8;
            pair[1] = m >> 4 | c << 4;
        }
    }
}

/// The two samples of three bytes, most significant bit first.
fn msb_pair([a, m, c]: [u8; 3], pair: &mut [u16; 2]) {
    let [a, m, c] = [a, m, c].map(u16::from);
    *pair = [a << 4 | m >> 4, (m & 0xF) << 8 | c];
}

/// One row of `MsbWords`, whose length is a whole number of words.
fn unpack_msb_words(src: &[u8], dst: &mut [u16]) {
    for (i, pair) in dst.as_chunks_mut::<2>().0.iter_mut().enumerate() {
        msb_pair([0, 1, 2].map(|k| src[(3 * i + k) ^ 3]), pair);
    }
}

/// One row of `MsbFields`.
fn unpack_msb(src: &[u8], dst: &mut [u16]) {
    for (&group, pair) in src
        .as_chunks::<3>()
        .0
        .iter()
        .zip(dst.as_chunks_mut::<2>().0)
    {
        msb_pair(group, pair);
    }
}

/// The stored rows of every strip in turn. `RowsPerStrip` is not used: each
/// field of an `IIRS` file ends in its own short strip.
fn stored_rows<'a>(
    tags: &Tags<'_, 'a>,
    counts: &[u64],
    row_bytes: usize,
    height: usize,
) -> Result<Vec<&'a [u8]>, PsdError> {
    let offsets = tags
        .uints(STRIP_OFFSETS)?
        .filter(|offsets| offsets.len() == counts.len())
        .ok_or_else(|| invalid("strip offsets"))?;
    let mut rows = Vec::new();
    for (&at, &len) in offsets.iter().zip(counts) {
        let (at, len) = (to_usize(at)?, to_usize(len)?);
        if !len.is_multiple_of(row_bytes) {
            return Err(unsupported("strip holding part of a row"));
        }
        let strip = at
            .checked_add(len)
            .and_then(|end| tags.tiff.bytes().get(at..end))
            .ok_or(PsdError::Truncated)?;
        rows.extend(strip.chunks(row_bytes));
    }
    if rows.len() != height {
        return Err(invalid("strips do not hold the plane"));
    }
    Ok(rows)
}

/// Where row `y` of the plane lies among the rows stored as two fields.
fn field_row(y: usize, height: usize) -> usize {
    y / 2 + (y % 2) * height.div_ceil(2)
}

/// Unpacks the packed codings.
fn decode_packed(
    tags: &Tags,
    counts: &[u64],
    coding: Coding,
    (width, height): (usize, usize),
) -> Result<Vec<u16>, PsdError> {
    let (row_bytes, unpack): (usize, UnpackRow) = match coding {
        Coding::PaddedLsb => (width / 5 * 8, unpack_padded),
        Coding::MsbWords => (width / 2 * 3, unpack_msb_words),
        Coding::MsbFields => (width / 2 * 3, unpack_msb),
        Coding::Words => unreachable!("read as TIFF strips"),
    };
    let rows = stored_rows(tags, counts, row_bytes, height)?;
    let mut plane = vec![0u16; width * height];
    plane
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(y, dst)| {
            let stored = match coding {
                Coding::MsbFields => field_row(y, height),
                _ => y,
            };
            unpack(rows[stored], dst);
        });
    Ok(plane)
}

/// Exif `CFAPattern`: the two repeat sizes in file order, then the colours.
fn cfa(tiff: &Tiff, ifd0: &Tags) -> Result<CfaPattern, PsdError> {
    let at = ifd0
        .uint(EXIF_IFD)?
        .ok_or_else(|| invalid("missing Exif directory"))?;
    let exif = tiff.ifd(at)?;
    let entry = exif
        .get(EXIF_CFA_PATTERN)
        .ok_or_else(|| unsupported("without a CFA pattern"))?;
    let &[w0, w1, h0, h1, a, b, c, d] = tiff.raw(entry) else {
        return Err(unsupported("CFA other than 2 × 2"));
    };
    let size = [tiff.order.u16([w0, w1]), tiff.order.u16([h0, h1])];
    // Green on one diagonal, red and blue on the other.
    let bayer = matches!(
        [a, b, c, d],
        [1, 0, 2, 1] | [1, 2, 0, 1] | [0, 1, 1, 2] | [2, 1, 1, 0]
    );
    if size != [2, 2] || !bayer {
        return Err(unsupported("CFA other than 2 × 2 red, green, green, blue"));
    }
    Ok(CfaPattern {
        width: 2,
        height: 2,
        colors: vec![a, b, c, d],
    })
}

fn default_crop(meta: &OrfMeta, area: Area) -> Result<DngCrop, PsdError> {
    let (width, height) = (area.width as f64, area.height as f64);
    match meta.crop {
        None => Ok(DngCrop {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }),
        Some(c)
            if c.x >= 0.0 && c.y >= 0.0 && c.x + c.width <= width && c.y + c.height <= height =>
        {
            Ok(c)
        }
        Some(_) => Err(invalid("crop outside the plane")),
    }
}

/// Reads an ORF's sensor plane, normalized over the whole plane.
// ponytail: the black is one value per 2 × 2 position, in stored order.
pub(super) fn read_orf(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
    if !is_orf(bytes) {
        return Err(invalid("not an Olympus raw file"));
    }
    let order = if bytes.starts_with(b"MM") {
        Order::Big
    } else {
        Order::Little
    };
    let tiff = Tiff::view(bytes, order);
    let ifd0 = tiff.ifd(tiff.first_ifd()?)?;
    let tags = Tags {
        tiff: &tiff,
        ifd: &ifd0,
    };
    let size = |tag, what| -> Result<usize, PsdError> {
        to_usize(tags.uint(tag)?.ok_or_else(|| invalid(what))?)
    };
    let width = size(IMAGE_WIDTH, "missing ImageWidth")?;
    let height = size(IMAGE_LENGTH, "missing ImageLength")?;
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
    let counts = tags
        .uints(STRIP_BYTE_COUNTS)?
        .ok_or_else(|| invalid("missing StripByteCounts"))?;
    let stored = counts.iter().fold(0u64, |sum, &c| sum.saturating_add(c));
    if stored > bytes.len() as u64 {
        return Err(PsdError::Truncated);
    }
    let coding = coding(bytes, stored, counts.len(), (width, height))?;

    let meta = OrfMeta::read(&tags)?;
    let bits = meta.bits.unwrap_or(12);
    let bits_ok = match coding {
        Coding::Words => (8..=16).contains(&bits),
        _ => bits == 12,
    };
    if !bits_ok {
        return Err(unsupported(&format!("{bits}-bit samples")));
    }
    let cfa = cfa(&tiff, &tags)?;
    let area = Area {
        top: 0,
        left: 0,
        width,
        height,
    };
    let crop = default_crop(&meta, area)?;

    let dims = (width, height);
    let plane = match coding {
        Coding::Words => {
            let layout = layout(&tags, width, height, budget)?;
            decode_plane(&tiff, &layout, 1, 16, dims, 1)?
        }
        _ => decode_packed(&tags, &counts, coding, dims)?,
    };

    let scale = match coding {
        Coding::Words => (1u32 << (16 - bits)) as f64,
        _ => 1.0,
    };
    let black = meta.black.unwrap_or([0.0; 4]);
    let top_black = black.iter().fold(0.0f64, |a, &b| a.max(b));
    let ceiling = ((1u32 << bits) - 1) as f64;
    let white = meta
        .white
        .filter(|&w| w > top_black && w <= ceiling)
        .unwrap_or(ceiling);
    let levels = Levels {
        linearization: None,
        black: black.map(|b| b * scale).to_vec(),
        black_rows: 2,
        black_cols: 2,
        delta_v: Vec::new(),
        delta_h: Vec::new(),
        white: white * scale,
    };
    let samples = levels::normalize(&plane, width, area, &levels);
    Ok(DngImage {
        width,
        height,
        samples,
        cfa,
        default_crop: crop,
        orientation: tags.uint(ORIENTATION)?.unwrap_or(1).min(8) as u16,
        make: tags.ascii(MAKE),
        model: tags.ascii(MODEL),
        color: meta.color(),
    })
}
