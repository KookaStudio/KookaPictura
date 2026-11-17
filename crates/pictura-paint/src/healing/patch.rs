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
use pictura_core::{Coverage, Document, PsdRect};

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
    selection: &Coverage,
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
/// coverage inside it; `None` when the mask is the wrong size or empty. The
/// mask is read tile by tile and row by row, never materialized (#1268).
pub(super) fn selection_coverage(
    doc: &Document,
    selection: &Coverage,
) -> Option<(PsdRect, Vec<f32>)> {
    let bounds = selection.covered_bounds(PsdRect {
        left: 0,
        top: 0,
        right: doc.width as i32,
        bottom: doc.height as i32,
    })?;
    let (x0, y0, x1, y1) = (
        bounds.left as usize,
        bounds.top as usize,
        bounds.right as usize,
        bounds.bottom as usize,
    );
    let mut coverage = Vec::with_capacity((x1 - x0) * (y1 - y0));
    for y in y0..y1 {
        let row = selection.row(x0 as i32, y as i32, x1 - x0);
        coverage.extend(row.iter().map(|&v| f32::from(v) / 255.0));
    }
    let bounds = PsdRect {
        top: y0 as i32,
        left: x0 as i32,
        bottom: y1 as i32,
        right: x1 as i32,
    };
    Some((bounds, coverage))
}
