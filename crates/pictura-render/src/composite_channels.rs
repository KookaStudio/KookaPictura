//! Channel Include (`brst`) at the layer boundary.
//!
//! A restricted channel keeps the pre-layer backdrop's premultiplied value
//! `dst * dst_alpha / out_alpha` after the layer's content and effects have
//! composited. The restore runs once around the whole contribution; applying
//! the same formula after every pass telescopes to the same result while the
//! channel is never modified in between.
//!
//! ponytail: the snapshot is a layer-region buffer (the layer rect unioned with
//! its effect-pass rects). If a profile shows many restricted layers, keep only
//! the excluded planes instead of a full RGB+alpha copy.

use pictura_core::{ColorMode, Document, Knockout, Layer, PsdRect};

use crate::composite::Canvas;
use crate::composite_plan::PlanNode;

/// The working RGB planes `layer` excludes in `doc`. RGB and Grayscale render
/// an exclusion; other colour modes keep the list for round-trip but apply no
/// exclusion.
pub(crate) fn excluded_planes(layer: &Layer, doc: &Document) -> [bool; 3] {
    let mut excluded = [false; 3];
    match doc.mode {
        ColorMode::Rgb => {
            for index in &layer.restricted_channels {
                if let Some(slot) = excluded.get_mut(*index as usize) {
                    *slot = true;
                }
            }
        }
        ColorMode::Grayscale => {
            excluded = [layer.restricted_channels.contains(&0); 3];
        }
        _ => {}
    }
    excluded
}

pub(crate) fn all_excluded(excluded: [bool; 3]) -> bool {
    excluded.iter().all(|&value| value)
}

/// The canvas region a layer can touch: its content rect plus every effect-pass
/// rect, or the whole canvas for a group, a destructive adjustment, or a
/// transparency-shapes-off knockout.
pub(crate) fn region(canvas: &Canvas, node: &PlanNode<'_>) -> Option<PsdRect> {
    let whole = || PsdRect {
        top: canvas.oy,
        left: canvas.ox,
        bottom: canvas.oy + canvas.h as i32,
        right: canvas.ox + canvas.w as i32,
    };
    let layer = node.layer;
    let destructive = !layer.is_group() && crate::layer_effects::is_destructive_adjustment(layer);
    let mut rect = if layer.is_group() || destructive {
        whole()
    } else {
        layer.rect
    };
    if !layer.is_group() && !destructive {
        if let Some(passes) = node.passes.as_deref() {
            for pass in passes.below.iter().chain(&passes.fold).chain(&passes.above) {
                rect = rect.union(pass.rect);
            }
        }
        if !layer.transparency_shapes && layer.knockout != Knockout::None {
            rect = whole();
        }
    }
    let left = rect.left.max(canvas.ox);
    let top = rect.top.max(canvas.oy);
    let right = rect.right.min(canvas.ox + canvas.w as i32);
    let bottom = rect.bottom.min(canvas.oy + canvas.h as i32);
    (right > left && bottom > top).then_some(PsdRect {
        top,
        left,
        bottom,
        right,
    })
}

/// The pre-contribution `(r, g, b, alpha)` of every pixel in `rect`.
pub(crate) fn snapshot(canvas: &Canvas, rect: PsdRect) -> Vec<[f32; 4]> {
    let mut out = Vec::with_capacity(rect.width() as usize * rect.height() as usize);
    for y in rect.top..rect.bottom {
        let row = (y - canvas.oy) as usize * canvas.w;
        for x in rect.left..rect.right {
            let px = canvas.px[row + (x - canvas.ox) as usize];
            out.push([px.r, px.g, px.b, px.a]);
        }
    }
    out
}

/// Restore the excluded channels from `snapshot` after the layer's
/// contribution, keeping the composited alpha.
pub(crate) fn restore(
    canvas: &mut Canvas,
    rect: PsdRect,
    excluded: [bool; 3],
    snapshot: &[[f32; 4]],
) {
    let mut i = 0;
    for y in rect.top..rect.bottom {
        let row = (y - canvas.oy) as usize * canvas.w;
        for x in rect.left..rect.right {
            let px = &mut canvas.px[row + (x - canvas.ox) as usize];
            if px.a > 0.0 {
                let dst = snapshot[i];
                let ratio = dst[3] / px.a;
                if excluded[0] {
                    px.r = dst[0] * ratio;
                }
                if excluded[1] {
                    px.g = dst[1] * ratio;
                }
                if excluded[2] {
                    px.b = dst[2] * ratio;
                }
            }
            i += 1;
        }
    }
}
