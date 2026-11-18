//! Forward pixel replacements for recovery. PSD/PSB checkpoints carry changes
//! to the layer layout or metadata and any sample representation this delta
//! cannot reproduce exactly.
use std::io;

use pictura_core::{BitDepth, ColorMode, Document, Plane};

use super::planes::{each_plane, each_plane_mut, hollow};

const MAX_DELTA_BYTES: usize = 32 * 1024 * 1024;
const CHUNK_SIDE: usize = 256;

fn pixel_layout_supported(doc: &Document) -> bool {
    doc.mode == ColorMode::Rgb
        && doc.depth == BitDepth::Eight
        && doc.source.planes().is_none()
        && doc.source.original_profile().is_none()
        && doc.source.working_profile().is_none()
        && doc.source.mode().is_none_or(|mode| mode == ColorMode::Rgb)
        && !doc.layers.is_empty()
}

fn pixel_planes(doc: &Document) -> Vec<(Plane<u8>, usize, usize)> {
    let mut planes = Vec::new();
    each_plane(doc, &mut |plane, width, height, _| {
        planes.push((plane.clone(), width, height));
    });
    planes
}

/// None means a checkpoint is required. An empty payload advances the commit
/// sequence for an edit with no native pixel changes.
pub(crate) fn recovery_delta(before: &Document, after: &Document) -> Option<Vec<u8>> {
    if !pixel_layout_supported(before)
        || !pixel_layout_supported(after)
        || hollow(before) != hollow(after)
    {
        return None;
    }
    let before = pixel_planes(before);
    let after = pixel_planes(after);
    if before.len() != after.len() {
        return None;
    }
    let mut output = Vec::new();
    // A layered RGB document derives the composite from the planes below.
    for (index, ((old, ow, oh), (new, width, height))) in
        before.iter().zip(&after).enumerate().skip(1)
    {
        if (*ow, *oh, old.len()) != (*width, *height, new.len()) {
            return None;
        }
        if old.ident() == new.ident() || new.is_empty() {
            continue;
        }
        if width.checked_mul(*height)? != new.len() {
            return None;
        }
        if new.tile_count() > 0 {
            for tile in 0..new.tile_count() {
                if !old.shares_tile(tile, new) {
                    let (x, y, w, h) = new.tile_rect(tile)?;
                    let mut pixels = vec![0; w.checked_mul(h)?];
                    new.read_rect_into(x..x + w, y..y + h, *width, &mut pixels);
                    push_rect(&mut output, [index, *width, *height, x, y, w, h], &pixels)?;
                }
            }
        } else {
            for y in (0..*height).step_by(CHUNK_SIDE) {
                for x in (0..*width).step_by(CHUNK_SIDE) {
                    let w = CHUNK_SIDE.min(width - x);
                    let h = CHUNK_SIDE.min(height - y);
                    let mut a = vec![0; w * h];
                    let mut b = vec![0; w * h];
                    old.read_rect_into(x..x + w, y..y + h, *width, &mut a);
                    new.read_rect_into(x..x + w, y..y + h, *width, &mut b);
                    if a != b {
                        push_rect(&mut output, [index, *width, *height, x, y, w, h], &b)?;
                    }
                }
            }
        }
    }
    Some(output)
}

fn push_rect(output: &mut Vec<u8>, fields: [usize; 7], data: &[u8]) -> Option<()> {
    if output.len().checked_add(28 + data.len())? > MAX_DELTA_BYTES {
        return None;
    }
    for field in fields {
        output.extend_from_slice(&u32::try_from(field).ok()?.to_le_bytes());
    }
    output.extend_from_slice(data);
    Some(())
}

struct Replacement<'a> {
    index: usize,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    data: &'a [u8],
}

pub(crate) fn apply_recovery_delta(doc: &mut Document, mut bytes: &[u8]) -> io::Result<()> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid recovery tile layout");
    let planes = pixel_planes(doc);
    let mut replacements = Vec::new();
    while !bytes.is_empty() {
        let (header, rest) = bytes.split_at_checked(28).ok_or_else(invalid)?;
        let offset = |i: usize| {
            let start = i * 4;
            u32::from_le_bytes(header[start..start + 4].try_into().unwrap()) as usize
        };
        let (index, width, height) = (offset(0), offset(1), offset(2));
        let (x, y, w, h) = (offset(3), offset(4), offset(5), offset(6));
        let (plane, plane_width, plane_height) = planes.get(index).ok_or_else(invalid)?;
        if index == 0
            || (*plane_width, *plane_height) != (width, height)
            || width.checked_mul(height) != Some(plane.len())
            || w == 0
            || h == 0
            || x.checked_add(w).is_none_or(|end| end > width)
            || y.checked_add(h).is_none_or(|end| end > height)
        {
            return Err(invalid());
        }
        let size = w.checked_mul(h).ok_or_else(invalid)?;
        let (data, remainder) = rest.split_at_checked(size).ok_or_else(invalid)?;
        replacements.push(Replacement {
            index,
            width,
            height,
            x,
            y,
            w,
            h,
            data,
        });
        bytes = remainder;
    }

    let mut indices: std::collections::BTreeMap<usize, Vec<Replacement<'_>>> = Default::default();
    for replacement in replacements {
        indices
            .entry(replacement.index)
            .or_default()
            .push(replacement);
    }
    let mut index = 0;
    each_plane_mut(doc, &mut |plane, _, height, _| {
        if let Some(tiles) = indices.remove(&index) {
            for tile in tiles {
                debug_assert_eq!(height, tile.height);
                plane.write_rect_from(
                    tile.x..tile.x + tile.w,
                    tile.y..tile.y + tile.h,
                    tile.width,
                    tile.data,
                );
            }
        }
        index += 1;
    });
    Ok(())
}
