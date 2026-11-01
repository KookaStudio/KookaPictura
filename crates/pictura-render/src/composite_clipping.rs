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
use crate::composite_blend::BlendOverride;
use crate::composite_plan::PlanNode;
use crate::composite_rows::{composite_canvas, serial};

/// Composite `layers` (bottom-first) onto `canvas` through `composite`, which
/// receives each layer's index; a base and the clipped layers above it
/// composite as one group.
pub(crate) fn composite_siblings<'a>(
    canvas: &mut Canvas,
    layers: &'a [Layer],
    doc: &'a Document,
    plan: &[PlanNode<'a>],
    composite: impl FnMut(&mut Canvas, usize, &PlanNode<'a>, Option<&BlendOverride>),
) {
    composite_siblings_range(canvas, layers, doc, plan, 0, layers.len(), composite);
}

/// [`composite_siblings`] over `layers[start..end]`. `start` must be a clipping
/// boundary (a non-clipped base or a run start); a resume inside a clipped run
/// is not expressible.
pub(crate) fn composite_siblings_range<'a>(
    canvas: &mut Canvas,
    layers: &'a [Layer],
    doc: &'a Document,
    plan: &[PlanNode<'a>],
    start: usize,
    end: usize,
    mut composite: impl FnMut(&mut Canvas, usize, &PlanNode<'a>, Option<&BlendOverride>),
) {
    let end = end.min(layers.len());
    let mut i = start;
    while i < end {
        let base = &layers[i];
        let run = layers[i + 1..end].iter().take_while(|l| l.clipping).count();
        if run == 0 || !base.visible {
            // A hidden base hides the clipped layers above it.
            if base.visible || run == 0 {
                composite(canvas, i, &plan[i], None);
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
            i,
            &clipped,
            doc,
            plan,
            &mut composite,
        );
        let proxy_over = BlendOverride {
            fill: Some(255),
            no_mask: true,
            no_blend_if: true,
            ..Default::default()
        };
        let proxy_ctx = plan[i]
            .blend
            .as_ref()
            .expect("a visible base resolves a blend context")
            .with_override(&proxy_over);
        composite_canvas(
            canvas,
            plan[i].layer,
            doc,
            &group,
            &proxy_ctx,
            Some(&proxy_over),
        );
        i += 1 + run;
    }
}

/// Build the raw clipped group over `bounds` with the CPU sibling-group
/// algorithm, without blending it onto a parent canvas. `clip` is the parent's
/// mask-feather clip and `effect_region` its transient-pass setting. Both the CPU
/// compositor and the GPU clip dispatch call this so the two cannot drift.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_sibling_group<'a>(
    bounds: PsdRect,
    clip: PsdRect,
    effect_region: Option<PsdRect>,
    skip_effects: bool,
    base_index: usize,
    clipped: &[(usize, &Layer)],
    doc: &Document,
    plan: &[PlanNode<'a>],
    mut composite: impl FnMut(&mut Canvas, usize, &PlanNode<'a>, Option<&BlendOverride>),
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
    let opaque_over = BlendOverride {
        opacity: Some(255),
        mode: Some(BlendMode::Normal),
        isolated_group: Some(true),
        ..Default::default()
    };
    composite(
        &mut group,
        base_index,
        &plan[base_index],
        Some(&opaque_over),
    );
    let cover = coverage(&group, &plan[base_index], doc);
    for &(index, layer) in clipped {
        if !layer.visible {
            continue;
        }
        if crate::layer_effects::is_destructive_adjustment(layer) {
            composite_clipped_adjustment(&mut group, &cover, index, layer, &mut composite);
            continue;
        }
        let mut tmp = group.clone();
        composite(&mut tmp, index, &plan[index], None);
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

/// A clipped adjustment recolors the group and adds no coverage. It runs over an
/// opaque copy so its source alpha cannot raise the group's alpha or dilute its
/// colour, and the result keeps the group's alpha. Coverage still gates it, so
/// a base's effects outside its content stay unadjusted.
fn composite_clipped_adjustment(
    group: &mut Canvas,
    cover: &[f32],
    index: usize,
    layer: &Layer,
    composite: &mut impl FnMut(&mut Canvas, usize, &Layer),
) {
    let mut opaque = group.clone();
    opaque.px.iter_mut().for_each(|p| p.a = 1.0);
    composite(&mut opaque, index, layer);
    let recolor = |((out, adjusted), &k): ((&mut Px, &Px), &f32)| {
        if k > 0.0 {
            out.r = adjusted.r;
            out.g = adjusted.g;
            out.b = adjusted.b;
        }
    };
    if serial() {
        group
            .px
            .iter_mut()
            .zip(&opaque.px)
            .zip(cover)
            .for_each(recolor);
    } else {
        group
            .px
            .par_iter_mut()
            .zip(opaque.px.par_iter())
            .zip(cover.par_iter())
            .for_each(recolor);
    }
}

fn lerp(out: &mut Px, inside: &Px, k: f32) {
    out.r += (inside.r - out.r) * k;
    out.g += (inside.g - out.g) * k;
    out.b += (inside.b - out.b) * k;
    out.a += (inside.a - out.a) * k;
}

/// The base's coverage over `canvas`'s region: its alpha composited alone onto
/// transparency at full opacity and Fill, without effects.
fn coverage(canvas: &Canvas, base: &PlanNode<'_>, doc: &Document) -> Vec<f32> {
    let mut alone = canvas.derived(canvas.ox, canvas.oy, canvas.w, canvas.h);
    alone.skip_effects = true;
    let full_over = BlendOverride {
        opacity: Some(255),
        fill: Some(255),
        mode: Some(BlendMode::Normal),
        isolated_group: Some(true),
        ..Default::default()
    };
    composite_layer_inner(&mut alone, base, doc, None, None, Some(&full_over));
    if serial() {
        alone.px.iter().map(|p| p.a.clamp(0.0, 1.0)).collect()
    } else {
        alone.px.par_iter().map(|p| p.a.clamp(0.0, 1.0)).collect()
    }
}
