//! Clipping groups for the CPU compositor. A clipped layer (`Layer::clipping`)
//! with a non-clipped sibling below it in the same list (its base) shows only
//! where the base has content, and the group takes on the base's opacity and
//! blend mode, as CS6 documents ("the layers in a clipping mask take on the
//! opacity and mode attributes of the bottommost layer"). A clipped layer at
//! the bottom of its list has no base and composites unclipped.
//!
//! The group renders in isolation: the base at full opacity, then each clipped
//! layer composited onto a copy and blended back in by the base's coverage
//! (exact for every blend mode linear in the layer's coverage, all but
//! Dissolve); the result composites onto the backdrop with the base's opacity
//! and mode. The coverage is the base's content alone: its Fill opacity and its
//! effects do not narrow it, so a base at Fill 0 still clips.
//!
//! ponytail: isolating the group means a clipped layer's blend mode sees only
//! the base and the clipped layers below it, not the backdrop under the base.

use pictura_core::{BlendMode, Document, Layer, PsdRect};
use rayon::prelude::*;

use crate::composite::{composite_layer_inner, Canvas, Px};
use crate::composite_rows::{composite_canvas, serial};

/// Composite `layers` (bottom-first) onto `canvas` through `composite`, which
/// receives each layer's index; a base and the clipped layers above it
/// composite as one group.
pub(crate) fn composite_siblings(
    canvas: &mut Canvas,
    layers: &[Layer],
    doc: &Document,
    composite: impl FnMut(&mut Canvas, usize, &Layer),
) {
    composite_siblings_range(canvas, layers, doc, 0, layers.len(), composite);
}

/// [`composite_siblings`] over `layers[start..end]`. `start` must be a clipping
/// boundary (a non-clipped base or a run start); a resume inside a clipped run
/// is not expressible.
pub(crate) fn composite_siblings_range(
    canvas: &mut Canvas,
    layers: &[Layer],
    doc: &Document,
    start: usize,
    end: usize,
    mut composite: impl FnMut(&mut Canvas, usize, &Layer),
) {
    let end = end.min(layers.len());
    let mut i = start;
    while i < end {
        let base = &layers[i];
        let run = layers[i + 1..end].iter().take_while(|l| l.clipping).count();
        if run == 0 || !base.visible {
            // A hidden base hides the clipped layers above it.
            if base.visible || run == 0 {
                composite(canvas, i, base);
            }
            i += 1 + run;
            continue;
        }
        let bounds = PsdRect {
            top: canvas.y0(),
            left: canvas.x0(),
            bottom: canvas.y1(),
            right: canvas.x1(),
        };
        let clipped: Vec<(usize, &Layer)> =
            layers.iter().enumerate().skip(i + 1).take(run).collect();
        let group = build_sibling_group(
            bounds,
            canvas.clip(),
            canvas.effect_region,
            canvas.skip_effects,
            base,
            i,
            &clipped,
            doc,
            &mut composite,
        );
        let proxy = Layer {
            opacity: base.opacity,
            blend: base.blend,
            fill: 255,
            mask: None,
            vector_mask: None,
            blend_if: None,
            is_group: true,
            channels: Vec::new(),
            children: pictura_core::LayerStack::default(),
            ..base.clone()
        };
        composite_canvas(canvas, &proxy, doc, &group);
        i += 1 + run;
    }
}

/// Build the raw clipped group over `bounds` with the CPU sibling-group
/// algorithm, without blending it onto a parent canvas. `clip` is the parent's
/// mask-feather clip and `effect_region` its transient-pass setting. Both the CPU
/// compositor and the GPU clip dispatch call this so the two cannot drift.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_sibling_group(
    bounds: PsdRect,
    clip: PsdRect,
    effect_region: Option<PsdRect>,
    skip_effects: bool,
    base: &Layer,
    base_index: usize,
    clipped: &[(usize, &Layer)],
    doc: &Document,
    mut composite: impl FnMut(&mut Canvas, usize, &Layer),
) -> Canvas {
    let mut group = Canvas::new_region(
        bounds.left,
        bounds.top,
        bounds.width().max(0) as usize,
        bounds.height().max(0) as usize,
    );
    group.clip = clip;
    group.effect_region = effect_region;
    group.skip_effects = skip_effects;
    let opaque = Layer {
        opacity: 255,
        blend: BlendMode::Normal,
        ..base.clone()
    };
    composite(&mut group, base_index, &opaque);
    let cover = coverage(&group, base, doc);
    for &(index, layer) in clipped {
        if !layer.visible {
            continue;
        }
        let mut tmp = group.clone();
        composite(&mut tmp, index, layer);
        if serial() {
            for ((out, inside), &k) in group.px.iter_mut().zip(&tmp.px).zip(&cover) {
                lerp(out, inside, k);
            }
        } else {
            group
                .px
                .par_iter_mut()
                .zip(tmp.px.par_iter())
                .zip(cover.par_iter())
                .for_each(|((out, inside), &k)| lerp(out, inside, k));
        }
    }
    group
}

fn lerp(out: &mut Px, inside: &Px, k: f32) {
    out.r += (inside.r - out.r) * k;
    out.g += (inside.g - out.g) * k;
    out.b += (inside.b - out.b) * k;
    out.a += (inside.a - out.a) * k;
}

/// The base's coverage over `canvas`'s region: its alpha composited alone onto
/// transparency at full opacity and Fill, without effects.
fn coverage(canvas: &Canvas, base: &Layer, doc: &Document) -> Vec<f32> {
    let mut alone = canvas.derived(canvas.ox, canvas.oy, canvas.w, canvas.h);
    alone.skip_effects = true;
    let full = Layer {
        opacity: 255,
        fill: 255,
        blend: BlendMode::Normal,
        ..base.clone()
    };
    composite_layer_inner(&mut alone, &full, doc, None, None);
    if serial() {
        alone.px.iter().map(|p| p.a.clamp(0.0, 1.0)).collect()
    } else {
        alone.px.par_iter().map(|p| p.a.clamp(0.0, 1.0)).collect()
    }
}
