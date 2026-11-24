//! Canon CR3 (#1474): an ISO base media file (ISO/IEC 14496-12) of brand
//! `crx `. Each `CRAW` video track holds CRX-coded frames (`crx.rs`); the
//! largest is the full-resolution sensor plane, and its first sample the
//! main image (a raw burst's other frames and a Dual Pixel file's second
//! track are not read). Canon's `moov/uuid` box carries IFD0 (`CMT1`) and
//! the maker note (`CMT3`) as little TIFF files; the as-shot white balance
//! is in the maker note of the `CTMD` timed-metadata track's sample.
//!
//! The box walk follows the ISO base media specification and public
//! write-ups of the CR3 layout. The track choice, the `CMP1` fields and the
//! `CTMD` record layout follow LibRaw 0.22.1 (`src/metadata/cr3_parser.cpp`,
//! Copyright (C) 2019-2025 LibRaw LLC, LGPL-2.1; see `crx.rs` and
//! `assets/PROVENANCE.md`). The file has no colour matrix: colour comes from
//! the fitted per-camera table (#1465).

use super::cr2::{active_area, as_shot_neutral, mask, sensor_info};
use super::crx;
use super::levels::{self, Area, Levels};
use super::tiff::{Ifd, Tiff};
use super::vendor::{camera_color, masked_black};
use super::{CfaPattern, DngCrop, DngImage, Tags, MAKE, ORIENTATION};
use crate::error::PsdError;
use crate::probe::ImageBudget;

const MODEL: u16 = 272;
const CANON_UUID: [u8; 16] = [
    0x85, 0xc0, 0xb6, 0x87, 0x82, 0x0f, 0x11, 0xe0, 0x81, 0x11, 0xf4, 0xce, 0x46, 0x2b, 0x6a, 0x48,
];
/// Bytes of a `CRAW` sample entry before its child boxes.
const CRAW_ENTRY: usize = 82;
const MAKER_NOTE_TAG: u32 = 0x927C;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("CR3: {what}"))
}

pub(super) fn is_cr3(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && &bytes[8..12] == b"crx "
}

fn be32(b: &[u8], at: usize) -> Result<u64, PsdError> {
    let v = b.get(at..at + 4).ok_or(PsdError::Truncated)?;
    Ok(u32::from_be_bytes([v[0], v[1], v[2], v[3]]) as u64)
}

fn be64(b: &[u8], at: usize) -> Result<u64, PsdError> {
    let v = b.get(at..at + 8).ok_or(PsdError::Truncated)?;
    Ok(u64::from_be_bytes(
        v.try_into().map_err(|_| PsdError::Truncated)?,
    ))
}

fn le32(b: &[u8], at: usize) -> Option<usize> {
    let v = b.get(at..at + 4)?;
    Some(u32::from_le_bytes([v[0], v[1], v[2], v[3]]) as usize)
}

/// Boxes by name and content.
type Boxes<'a> = Vec<([u8; 4], &'a [u8])>;

/// The boxes `b` holds.
fn children(b: &[u8]) -> Result<Boxes<'_>, PsdError> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 8 <= b.len() {
        let name: [u8; 4] = b[at + 4..at + 8]
            .try_into()
            .map_err(|_| PsdError::Truncated)?;
        let (head, size) = match be32(b, at)? {
            0 => (8, (b.len() - at) as u64),
            1 => (16, be64(b, at + 8)?),
            s => (8, s),
        };
        let end = usize::try_from(size)
            .ok()
            .filter(|&s| s >= head)
            .and_then(|s| at.checked_add(s))
            .filter(|&e| e <= b.len())
            .ok_or_else(|| invalid("box size"))?;
        out.push((name, &b[at + head..end]));
        at = end;
    }
    Ok(out)
}

fn child<'a>(boxes: &[([u8; 4], &'a [u8])], name: &[u8; 4]) -> Option<&'a [u8]> {
    boxes.iter().find(|(n, _)| n == name).map(|&(_, b)| b)
}

/// What one `trak` box says: its handler, sample format, frame header and
/// where its first sample lies.
#[derive(Default)]
struct Track<'a> {
    handler: [u8; 4],
    format: [u8; 4],
    cmp1: Option<&'a [u8]>,
    first: Option<(u64, u64)>,
}

fn track(trak: &[u8]) -> Result<Track<'_>, PsdError> {
    let mut t = Track::default();
    let Some(mdia) = child(&children(trak)?, b"mdia") else {
        return Ok(t);
    };
    let mdia = children(mdia)?;
    if let Some(hdlr) = child(&mdia, b"hdlr") {
        t.handler = hdlr
            .get(8..12)
            .and_then(|h| h.try_into().ok())
            .unwrap_or_default();
    }
    let Some(stbl) = child(&mdia, b"minf")
        .map(children)
        .transpose()?
        .and_then(|m| child(&m, b"stbl"))
    else {
        return Ok(t);
    };
    let stbl = children(stbl)?;
    if let Some(stsd) = child(&stbl, b"stsd") {
        let entries = children(stsd.get(8..).ok_or(PsdError::Truncated)?)?;
        if let Some(&(format, entry)) = entries.first() {
            t.format = format;
            if &format == b"CRAW" {
                let boxes = children(entry.get(CRAW_ENTRY..).ok_or(PsdError::Truncated)?)?;
                t.cmp1 = child(&boxes, b"CMP1");
            }
        }
    }
    let size = match child(&stbl, b"stsz") {
        Some(stsz) => match be32(stsz, 4)? {
            0 if be32(stsz, 8)? > 0 => Some(be32(stsz, 12)?),
            0 => None,
            fixed => Some(fixed),
        },
        None => None,
    };
    let offset = match child(&stbl, b"co64") {
        Some(co64) if be32(co64, 4)? > 0 => Some(be64(co64, 8)?),
        _ => None,
    };
    t.first = offset.zip(size);
    Ok(t)
}

fn sample(bytes: &[u8], (offset, size): (u64, u64)) -> Result<&[u8], PsdError> {
    let start = usize::try_from(offset).map_err(|_| PsdError::Truncated)?;
    let len = usize::try_from(size).map_err(|_| PsdError::Truncated)?;
    start
        .checked_add(len)
        .and_then(|end| bytes.get(start..end))
        .ok_or(PsdError::Truncated)
}

/// The maker notes in a `CTMD` sample: records of a little-endian size and
/// type; types 7 to 9 list tagged blocks, of which `0x927C` is a maker note
/// written as a little TIFF file.
fn ctmd_notes(ctmd: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 6 < ctmd.len() {
        let (Some(size), Some(kind)) = (le32(ctmd, at), ctmd.get(at + 4..at + 6)) else {
            break;
        };
        let end = match at.checked_add(size) {
            Some(end) if size > 0 && end <= ctmd.len() => end,
            _ => break,
        };
        if matches!(u16::from_le_bytes([kind[0], kind[1]]), 7..=9) {
            let mut inner = at + 12;
            while inner + 8 < end {
                let (Some(len), Some(tag)) = (le32(ctmd, inner), le32(ctmd, inner + 4)) else {
                    break;
                };
                if len < 8 || inner + len > end {
                    break;
                }
                if tag as u32 == MAKER_NOTE_TAG {
                    out.push(&ctmd[inner + 8..inner + len]);
                }
                inner += len;
            }
        }
        at = end;
    }
    out
}

/// IFD0 of a TIFF block, or `None` when it is not one.
fn tiff_ifd(block: &[u8]) -> Option<(Tiff<'_>, Ifd)> {
    let (tiff, first) = Tiff::parse(block).ok()?;
    let ifd = tiff.ifd(first).ok()?;
    Some((tiff, ifd))
}

/// The CFA at the active corner: the header's pattern at the plane's
/// corner, shifted by the active area's offset.
fn pattern(cfa: u32, area: Area) -> Vec<u8> {
    const CORNER: [[u8; 4]; 4] = [[0, 1, 1, 2], [1, 0, 2, 1], [1, 2, 0, 1], [2, 1, 1, 0]];
    let corner = CORNER[cfa as usize];
    (0..4)
        .map(|i| corner[((i / 2 + area.top) % 2) * 2 + (i % 2 + area.left) % 2])
        .collect()
}

/// Reads a CR3's main sensor plane, normalized and cropped to its active area.
// White is the frame's precision ceiling: Canon files store clipped
// photosites there, above the white levels ColorData records, which are
// not read (#1587).
// ponytail: the black is one mean per CFA position; as for CR2.
pub(super) fn read_cr3(bytes: &[u8], budget: &ImageBudget) -> Result<DngImage, PsdError> {
    if !is_cr3(bytes) {
        return Err(invalid("not a CR3 header"));
    }
    let top = children(bytes)?;
    let moov = children(child(&top, b"moov").ok_or_else(|| invalid("no movie box"))?)?;
    let (mut cmt1, mut cmt3) = (None, None);
    let mut tracks = Vec::new();
    for &(name, body) in &moov {
        match &name {
            b"uuid" if body.get(..16) == Some(&CANON_UUID[..]) => {
                let canon = children(&body[16..])?;
                cmt1 = child(&canon, b"CMT1");
                cmt3 = child(&canon, b"CMT3");
            }
            b"trak" => tracks.push(track(body)?),
            _ => {}
        }
    }

    // The raw track: the CRAW track of the most bits per frame, the first
    // of equals (LibRaw's choice).
    let mut best: Option<(u64, crx::Header, (u64, u64))> = None;
    let mut refusal = None;
    for t in &tracks {
        let (Some(cmp1), Some(first)) = (t.cmp1, t.first) else {
            continue;
        };
        match crx::parse_header(cmp1) {
            Ok(h) => {
                let bits = h.bits as u64 * h.width as u64 * h.height as u64;
                if best.as_ref().is_none_or(|b| bits > b.0) {
                    best = Some((bits, h, first));
                }
            }
            Err(e) => refusal = refusal.or(Some(e)),
        }
    }
    let Some((_, h, first)) = best else {
        return Err(refusal.unwrap_or_else(|| invalid("no raw track")));
    };
    let (width, height) = (h.width, h.height);
    let max = budget.max_dimension as usize;
    if width > max || height > max {
        return Err(invalid(&format!(
            "{width} × {height} is outside the image budget"
        )));
    }

    let ifd0 = cmt1.and_then(tiff_ifd);
    let ifd0 = ifd0.as_ref().map(|(tiff, ifd)| Tags { tiff, ifd });
    let note = cmt3.and_then(tiff_ifd);
    let note = note.as_ref().map(|(tiff, ifd)| Tags { tiff, ifd });
    let info = sensor_info(note.as_ref())?;
    let area = active_area(info.as_deref(), width, height)?;
    let plane_bytes = (width * height * 2) as u64;
    let total = 2 * plane_bytes + (area.width * area.height * 2) as u64;
    if plane_bytes > budget.max_alloc_bytes || total > budget.max_document_bytes {
        return Err(invalid("sensor plane exceeds the allocation budget"));
    }

    let frame = sample(bytes, first)?;
    let plane = crx::decode(&h, frame)?;

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
        white: ((1u32 << h.precision()) - 1) as f64,
    };
    let samples = levels::normalize(&plane, width, area, &levels);

    let neutral = tracks
        .iter()
        .filter(|t| &t.handler == b"meta" && &t.format == b"CTMD")
        .filter_map(|t| t.first.and_then(|f| sample(bytes, f).ok()))
        .flat_map(ctmd_notes)
        .filter_map(tiff_ifd)
        .find_map(|(tiff, ifd)| {
            as_shot_neutral(Some(&Tags {
                tiff: &tiff,
                ifd: &ifd,
            }))
            .ok()
            .flatten()
        });
    let make = ifd0.as_ref().and_then(|t| t.ascii(MAKE));
    let model = ifd0.as_ref().and_then(|t| t.ascii(MODEL));
    let orientation = match &ifd0 {
        Some(t) => t.uint(ORIENTATION)?.unwrap_or(1).min(8) as u16,
        None => 1,
    };
    Ok(DngImage {
        width: area.width,
        height: area.height,
        samples,
        cfa: CfaPattern {
            width: 2,
            height: 2,
            colors: pattern(h.cfa, area),
        },
        default_crop: DngCrop {
            x: 0.0,
            y: 0.0,
            width: area.width as f64,
            height: area.height as f64,
        },
        orientation,
        color: camera_color(make.as_deref(), model.as_deref(), neutral),
        make,
        model,
    })
}
