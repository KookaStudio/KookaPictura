//! The Patch tool: repair the selection from a dragged-to area (Source), apply
//! the selection's content where it was dragged (Destination), or rebuild the
//! selection from its surroundings (Content-Aware). Every mode is one heal over
//! the selection's coverage, so it shares [`clone_region`] / [`heal_region`]
//! with the brushes.
//!
//! Ported from photorust's `Document::patch_selection`
//! (<https://github.com/perfecto25/photorust>).
//!
//! [`clone_region`]: super::clone_region
//! [`heal_region`]: super::heal_region

use super::layer::{heal_layer, layer_rect, HealError};
use super::{clamped_shift, clone_region, heal_region, HealMode, Transfer, BORDER};
use pictura_core::{Document, PsdRect};
use rayon::prelude::*;

/// The Patch options bar, plus the drag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PatchOptions {
    /// The drag, in document pixels.
    pub dx: i32,
    pub dy: i32,
    /// Rebuild the selection from its surroundings and ignore the drag.
    pub content_aware: bool,
    /// The selection is good material applied where it was dragged, rather
    /// than the flaw repaired from there.
    pub destination: bool,
    /// Transfer texture only, keeping the patched area's own colour.
    pub transparent: bool,
}

/// Patch the pixel layer at `path` through `selection`, a document-sized
/// coverage mask (`doc.width × doc.height`, row-major, `0..=255`). Returns the
/// document rectangle changed, or `None` for an empty selection, a zero drag
/// outside Content-Aware, or a selection with no boundary to heal against.
pub fn patch_layer(
    doc: &mut Document,
    path: &str,
    selection: &[u8],
    options: PatchOptions,
) -> Result<Option<PsdRect>, HealError> {
    let Some((bounds, coverage)) = selection_coverage(doc, selection) else {
        return Ok(None);
    };
    let transfer = if options.transparent {
        Transfer::TextureOnly
    } else {
        Transfer::Full
    };
    let (dx, dy) = (options.dx, options.dy);
    if options.content_aware {
        let window = bounds.grow(BORDER);
        return heal_layer(
            doc,
            path,
            bounds,
            &coverage,
            Some(window),
            |img, local, cov| heal_region(img, local, cov, HealMode::ContentAware),
        );
    }
    let region = if options.destination {
        // The selection's shape lands at the drag target and samples back from
        // where the selection sits.
        PsdRect {
            top: bounds.top + dy,
            left: bounds.left + dx,
            bottom: bounds.bottom + dy,
            right: bounds.right + dx,
        }
    } else {
        bounds
    };
    let read = if options.destination {
        (-dx, -dy)
    } else {
        (dx, dy)
    };
    let window = layer_rect(doc, path).map(|canvas| {
        let source = clamped_shift(region, read.0, read.1, canvas);
        region.union(source).grow(BORDER)
    });
    heal_layer(doc, path, region, &coverage, window, |img, local, cov| {
        clone_region(img, local, cov, read, transfer)
    })
}

/// A document-sized selection mask as its bounding box and the `0..=1`
/// coverage inside it; `None` when the mask is the wrong size or empty.
pub(super) fn selection_coverage(doc: &Document, selection: &[u8]) -> Option<(PsdRect, Vec<f32>)> {
    let (w, h) = (doc.width as i32, doc.height as i32);
    if selection.len() != (w.max(0) * h.max(0)) as usize {
        return None;
    }
    let bounds = bounds(selection, w, h)?;
    let coverage = (bounds.top..bounds.bottom)
        .flat_map(|y| (bounds.left..bounds.right).map(move |x| (y * w + x) as usize))
        .map(|i| f32::from(selection[i]) / 255.0)
        .collect();
    Some((bounds, coverage))
}

/// The bounding box of the non-zero coverage, or `None` when there is none.
/// Rows are scanned in parallel; the fold keeps scan order.
fn bounds(selection: &[u8], w: i32, h: i32) -> Option<PsdRect> {
    let width = w.max(0) as usize;
    let rows = h.max(0) as usize;
    if width == 0 {
        return None;
    }
    let spans: Vec<Option<(i32, i32)>> = selection
        .par_chunks(width)
        .take(rows)
        .enumerate()
        .map(|(_, row)| {
            let first = row.iter().position(|&c| c > 0)?;
            let last = row.iter().rposition(|&c| c > 0)?;
            Some((first as i32, last as i32 + 1))
        })
        .collect();
    let mut r: Option<PsdRect> = None;
    for (y, span) in spans.into_iter().enumerate() {
        let Some((left, right)) = span else {
            continue;
        };
        let y = y as i32;
        r = Some(match r {
            None => PsdRect {
                top: y,
                left,
                bottom: y + 1,
                right,
            },
            Some(r) => PsdRect {
                top: r.top,
                left: r.left.min(left),
                bottom: y + 1,
                right: r.right.max(right),
            },
        });
    }
    r
}
