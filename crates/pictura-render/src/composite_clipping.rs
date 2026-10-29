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

use pictura_core::{BlendMode, Document, Layer};
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
    mut composite: impl FnMut(&mut Canvas, usize, &Layer),
) {
    let mut i = 0;
    while i < layers.len() {
        let base = &layers[i];
        let run = layers[i + 1..].iter().take_while(|l| l.clipping).count();
        if run == 0 || !base.visible {
            // A hidden base hides the clipped layers above it.
            if base.visible || run == 0 {
                composite(canvas, i, base);
            }
            i += 1 + run;
            continue;
        }
        let mut group = canvas.derived(canvas.ox, canvas.oy, canvas.w, canvas.h);
        group.skip_effects = canvas.skip_effects;
        let opaque = Layer {
            opacity: 255,
            blend: BlendMode::Normal,
            ..base.clone()
        };
        composite(&mut group, i, &opaque);
        let cover = coverage(canvas, base, doc);
        for (j, layer) in layers.iter().enumerate().skip(i + 1).take(run) {
            let mut clipped = group.clone();
            composite(&mut clipped, j, layer);
            let lerp = |out: &mut Px, inside: &Px, k: f32| {
                out.r += (inside.r - out.r) * k;
                out.g += (inside.g - out.g) * k;
                out.b += (inside.b - out.b) * k;
                out.a += (inside.a - out.a) * k;
            };
            if serial() {
                for ((out, inside), &k) in group.px.iter_mut().zip(&clipped.px).zip(&cover) {
                    lerp(out, inside, k);
                }
            } else {
                group
                    .px
                    .par_iter_mut()
                    .zip(clipped.px.par_iter())
                    .zip(cover.par_iter())
                    .for_each(|((out, inside), &k)| lerp(out, inside, k));
            }
        }
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
