//! The Content-Aware Move tool: relocate the selection's pixels by a drag and
//! rebuild the hole they leave from its surroundings (Move), or copy them and
//! leave the original in place (Extend).
//!
//! The moved pixels are copied, not re-solved: a Poisson blend keeps only a
//! region's gradients and rebuilds its interior from the boundary, which is
//! right for a blemish but smears anything larger. Only the hole is
//! synthesised, at the options bar's [`Adaptation`].
//!
//! Ported from photorust's `move_region` / `Document::content_aware_move`
//! (<https://github.com/perfecto25/photorust>), without its later-CC Structure
//! and Color controls: CS6 exposes only Mode and Adaptation.

use super::layer::{heal_layer, HealError};
use super::patch::selection_coverage;
use super::{heal_region_adapted, Adaptation, HealMode, RgbaImage};
use pictura_core::{Document, PsdRect};

/// The Content-Aware Move options bar, plus the drag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoveOptions {
    /// The drag, in document pixels.
    pub dx: i32,
    pub dy: i32,
    /// CS6's Extend mode: copy the selection and leave the original in place.
    pub extend: bool,
    pub adaptation: Adaptation,
}

/// Move the selection's pixels on the pixel layer at `path` by the drag.
/// `selection` is a document-sized coverage mask (`doc.width × doc.height`,
/// row-major, `0..=255`). Returns the document rectangle changed, or `None`
/// for an empty selection, a zero drag, or a destination off the layer.
pub fn move_layer(
    doc: &mut Document,
    path: &str,
    selection: &[u8],
    options: MoveOptions,
) -> Result<Option<PsdRect>, HealError> {
    let Some((bounds, coverage)) = selection_coverage(doc, selection) else {
        return Ok(None);
    };
    heal_layer(doc, path, bounds, &coverage, |img, local, cov| {
        move_region(img, local, cov, options)
    })
}

fn move_region(
    img: &mut RgbaImage,
    region: PsdRect,
    coverage: &[f32],
    options: MoveOptions,
) -> Option<PsdRect> {
    let (dx, dy) = (options.dx, options.dy);
    if (dx, dy) == (0, 0) {
        return None;
    }
    let destination = PsdRect {
        top: region.top + dy,
        left: region.left + dx,
        bottom: region.bottom + dy,
        right: region.right + dx,
    };
    let visible = destination.intersect(img.rect())?;
    let source = img.clone();
    // Close the hole first, while its surroundings are untouched, so the moved
    // pixels are not sampled as if they belonged where the subject was.
    let filled = if options.extend {
        None
    } else {
        heal_region_adapted(
            img,
            region,
            coverage,
            HealMode::ContentAware,
            options.adaptation,
        )
    };
    let rw = region.width() as usize;
    let canvas = img.rect();
    for y in visible.top..visible.bottom {
        for x in visible.left..visible.right {
            let (sx, sy) = (x - dx, y - dy);
            if sx < canvas.left || sy < canvas.top || sx >= canvas.right || sy >= canvas.bottom {
                continue;
            }
            let t = coverage[(sy - region.top) as usize * rw + (sx - region.left) as usize]
                .clamp(0.0, 1.0);
            if t <= 0.0 {
                continue;
            }
            let (a, b) = (img.get(x, y), source.get(sx, sy));
            let mix = |c: usize| {
                (f32::from(a[c]) + (f32::from(b[c]) - f32::from(a[c])) * t).round() as u8
            };
            img.set(x, y, [mix(0), mix(1), mix(2), mix(3)]);
        }
    }
    Some(match filled {
        None => visible,
        Some(f) => PsdRect {
            top: f.top.min(visible.top),
            left: f.left.min(visible.left),
            bottom: f.bottom.max(visible.bottom),
            right: f.right.max(visible.right),
        },
    })
}
