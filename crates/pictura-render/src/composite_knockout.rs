//! Knockout entry points and base threading for the CPU compositor.
//!
//! Split out of `composite.rs` (which sits at its file-size ceiling) so the
//! two-base selection lives next to the knockout helpers. `composite.rs` owns
//! the per-layer dispatch ([`composite_layer_inner`]) and the canvas model.
//!
//! A layer with a knockout composites against a base rather than the running
//! backdrop. The base depends on the mode: `Deep` reaches the document
//! background, inherited across pass-through groups and reset at an isolation
//! boundary; `Shallow` stops at the initial backdrop of the compositor applying
//! the layer (the enclosing group's entry backdrop).

use pictura_core::{Document, Knockout, Layer};

use crate::composite::{composite_layer_inner, Canvas};

/// Composite `doc.layers` (bottom-first) onto `canvas`, applying each non-bottom
/// layer's knockout against its stopping-point base.
pub(crate) fn composite_layers(canvas: &mut Canvas, doc: &Document) {
    let background = knockout_base(canvas, doc);
    crate::composite_clipping::composite_siblings(canvas, &doc.layers, doc, |canvas, i, layer| {
        // At the document root both bases are the same document background, so
        // `Shallow` and `Deep` are byte-identical there.
        let (deep, shallow) = if i == 0 {
            (None, None)
        } else {
            (background.as_ref(), background.as_ref())
        };
        composite_layer(canvas, layer, doc, deep, shallow);
    });
}

/// The document background (the bottom layer composited alone), built only when
/// a non-bottom layer knocks out. ponytail: no Background flag in the model, so
/// the bottom layer is assumed to be the background; a non-background bottom
/// resolves to its content rather than transparency.
fn knockout_base(region: &Canvas, doc: &Document) -> Option<Canvas> {
    let present = doc.layers.iter().skip(1).any(has_knockout);
    present.then(|| {
        let mut base = region.derived(region.ox, region.oy, region.w, region.h);
        base.skip_effects = region.skip_effects;
        if let Some(background) = doc.layers.first() {
            composite_layer_inner(&mut base, background, doc, None, None);
        }
        base
    })
}

/// Whether `layer` or any descendant carries a non-`None` knockout (recursive).
pub(crate) fn has_knockout(layer: &Layer) -> bool {
    layer.knockout != Knockout::None || layer.children.iter().any(has_knockout)
}

/// Composite `layer`; a knockout with an effective base routes through
/// [`composite_knockout`], everything else through the inner dispatch.
///
/// The effective base is `deep.or(shallow)` for `Deep` (falling back when the
/// document has no Background), `shallow` for `Shallow`, and `None` otherwise.
pub(crate) fn composite_layer(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    deep: Option<&Canvas>,
    shallow: Option<&Canvas>,
) {
    let base = match layer.knockout {
        Knockout::None => None,
        Knockout::Deep => deep.or(shallow),
        Knockout::Shallow => shallow,
    };
    match base {
        Some(base) => composite_knockout(canvas, layer, doc, base),
        None => composite_layer_inner(canvas, layer, doc, deep, shallow),
    }
}

/// Composite `layer` against the stopping-point `base`, then replace the running
/// backdrop only where the layer contributed, punching the layers between it and
/// the base through at those pixels.
///
/// ponytail: shape-composited-against-the-stopping-point rule from
/// `docs/05-layers/layers-overview.md:181`; clipping/Transparency-Shapes unresolved.
fn composite_knockout(canvas: &mut Canvas, layer: &Layer, doc: &Document, base: &Canvas) {
    let mut tmp = Canvas::with_cover_from(base);
    composite_layer_inner(&mut tmp, layer, doc, None, None);
    let cover = tmp.cover.take().unwrap_or_default();
    for (i, covered) in cover.iter().enumerate() {
        if *covered {
            canvas.px[i] = tmp.px[i];
        }
    }
}
