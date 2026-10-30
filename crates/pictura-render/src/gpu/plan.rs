//! One support plan per visible layer, shared by eligibility and dispatch.
//!
//! `plan_stack` walks the visible layers in stack order, groups a clipping base
//! with its clipped run, and returns the first decline. The GPU dispatcher
//! consumes the returned tree instead of re-deriving a content route, so the
//! two decisions cannot drift. A clipping run is checked for eligibility but its
//! members are composited by the CPU sibling algorithm, so no source or effect
//! artifact is built for them here.

use std::sync::Arc;

use pictura_adjust::{Adjustment, AdjustmentLut};
use pictura_core::{BlendMode, Document, Knockout, Layer, PsdRect};

use super::{adjustment_params, GpuError};
use crate::composite_route::{content_route, ContentRoute};
use crate::layer_source::{self, CachedBuffer, LayerPasses, RasterRoute};

/// The adjustment dispatch: an inline shader kind or a lookup table.
#[allow(clippy::large_enum_variant)] // one 1 KiB table per LUT layer.
pub(super) enum AdjustmentPlan {
    Inline((u32, i32, i32, i32)),
    Lut(AdjustmentLut),
}

/// What a layer's plan dispatches.
#[allow(clippy::large_enum_variant)] // one 1 KiB LUT table per planned layer.
pub(super) enum Body<'a> {
    Pixels,
    Raster(CachedBuffer),
    Fill(CachedBuffer),
    Adjustment(AdjustmentPlan),
    Group {
        pass_through: bool,
        children: Vec<LayerPlan<'a>>,
    },
    ClipGroup {
        base: &'a Layer,
        clipped: Vec<&'a Layer>,
    },
    Decline(GpuError),
}

/// One layer's plan: its source body and the cached effect passes around it.
pub(super) struct LayerPlan<'a> {
    pub(super) layer: &'a Layer,
    pub(super) effects: Option<Arc<LayerPasses>>,
    pub(super) body: Body<'a>,
    /// True when this node only inherits a descendant's decline; the descendant
    /// was already counted, so the wrapper must not count it again.
    pub(super) inherited_decline: bool,
}

/// Plan every visible layer in `doc`, or return the first decline in stack
/// order. A document with a recorded source depth declines before any layer is
/// planned.
pub(super) fn plan_stack<'a>(
    doc: &'a Document,
    region: Option<PsdRect>,
) -> Result<Vec<LayerPlan<'a>>, GpuError> {
    if doc.source_depth.is_some() {
        return Err(GpuError::UnsupportedDepth);
    }
    plan_layers(&doc.layers, doc, region)
}

fn plan_layers<'a>(
    layers: &'a [Layer],
    doc: &Document,
    region: Option<PsdRect>,
) -> Result<Vec<LayerPlan<'a>>, GpuError> {
    let mut out = Vec::new();
    let mut first_err = None;
    let mut i = 0;
    while i < layers.len() {
        let base = &layers[i];
        let run = layers[i + 1..].iter().take_while(|l| l.clipping).count();
        if run == 0 || !base.visible {
            if base.visible {
                let plan = plan_layer(base, doc, region);
                record_plan(&plan, &mut first_err);
                if !matches!(plan.body, Body::Decline(_)) {
                    out.push(plan);
                }
            }
            i += 1 + run;
            continue;
        }

        // A clipping run is composited by the CPU sibling algorithm, so only its
        // eligibility is checked here. Building the members' sources/effects
        // would be discarded by `composite_clip_group`.
        let mut run_err = None;
        if let Err(e) = plan_clip_member(base, doc) {
            record_error(&e, true, &mut first_err);
            run_err = Some(e);
        }
        let mut clipped = Vec::new();
        for layer in layers.iter().skip(i + 1).take(run) {
            if !layer.visible {
                continue;
            }
            match plan_clip_member(layer, doc) {
                Ok(()) => clipped.push(layer),
                Err(e) => {
                    record_error(&e, true, &mut first_err);
                    if run_err.is_none() {
                        run_err = Some(e);
                    }
                }
            }
        }
        if run_err.is_none() {
            out.push(LayerPlan {
                layer: base,
                effects: None,
                body: Body::ClipGroup { base, clipped },
                inherited_decline: false,
            });
        }
        i += 1 + run;
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

fn plan_layer<'a>(layer: &'a Layer, doc: &Document, region: Option<PsdRect>) -> LayerPlan<'a> {
    let route = content_route(layer);

    // A group's own Dissolve/advanced-blending attributes are checked before its
    // children, matching the old document-wide walk.
    if layer.is_group {
        if let Some(e) = own_attribute_decline(layer) {
            return decline(layer, e, false);
        }
        let (pass_through, children) = match route {
            ContentRoute::PassThroughGroup => (true, plan_layers(&layer.children, doc, region)),
            ContentRoute::IsolatedGroup => (false, plan_layers(&layer.children, doc, region)),
            _ => unreachable!("content_route returns a group route for a group layer"),
        };
        return match children {
            Ok(children) => LayerPlan {
                layer,
                effects: None,
                body: Body::Group {
                    pass_through,
                    children,
                },
                inherited_decline: false,
            },
            Err(e) => decline(layer, e, true),
        };
    }

    let mut body = match route {
        ContentRoute::PassThroughGroup | ContentRoute::IsolatedGroup => {
            unreachable!("a non-group layer has a non-group content route")
        }
        ContentRoute::Fill(adjustment) => {
            if let Some(adj) = adjustment_params(&adjustment) {
                Body::Adjustment(AdjustmentPlan::Inline(adj))
            } else if let Some(lut) = pictura_adjust::adjustment_lut(&adjustment) {
                Body::Adjustment(AdjustmentPlan::Lut(lut))
            } else if let Some(cached) = crate::fill::fill_source(layer, doc) {
                Body::Fill(cached)
            } else {
                Body::Decline(GpuError::UnsupportedAdjustment)
            }
        }
        ContentRoute::UndecodedAdjustment | ContentRoute::UndecodedVectorFill => {
            Body::Decline(GpuError::UnsupportedAdjustment)
        }
        ContentRoute::SmartFiltered => {
            match layer_source::composited_source(layer, doc, RasterRoute::SmartFiltered) {
                Some(cached) => Body::Raster(cached),
                None if crate::channel_plane(layer, 0).is_some() => Body::Pixels,
                None => Body::Decline(GpuError::UnsupportedSmartObject),
            }
        }
        ContentRoute::SmartSource => {
            match layer_source::composited_source(layer, doc, RasterRoute::SmartSource) {
                Some(cached) => Body::Raster(cached),
                None if crate::channel_plane(layer, 0).is_some() => Body::Pixels,
                None => Body::Decline(GpuError::UnsupportedSmartObject),
            }
        }
        ContentRoute::TypeSource => {
            match layer_source::composited_source(layer, doc, RasterRoute::Type) {
                Some(cached) => Body::Raster(cached),
                None if crate::channel_plane(layer, 0).is_some() => Body::Pixels,
                None => Body::Decline(GpuError::UnsupportedText),
            }
        }
        ContentRoute::Pixels => Body::Pixels,
    };
    if !matches!(body, Body::Decline(_)) {
        if let Some(e) = own_attribute_decline(layer) {
            body = Body::Decline(e);
        }
    }
    if matches!(body, Body::Decline(_)) {
        return LayerPlan {
            layer,
            effects: None,
            body,
            inherited_decline: false,
        };
    }
    let effects = if !crate::layer_effects::is_destructive_adjustment(layer) {
        let decoded = layer_source::decoded_effects(layer);
        Some(layer_source::effect_passes(layer, doc, &decoded, region))
    } else {
        None
    };
    if let Some(passes) = &effects {
        if passes
            .below
            .iter()
            .chain(&passes.above)
            .any(|pass| matches!(pass.mode, BlendMode::Dissolve))
        {
            return LayerPlan {
                layer,
                effects: None,
                body: Body::Decline(GpuError::UnsupportedMode(BlendMode::Dissolve)),
                inherited_decline: false,
            };
        }
    }
    LayerPlan {
        layer,
        effects,
        body,
        inherited_decline: false,
    }
}

/// Eligibility for a member of a clipping run. The member is composited by the
/// CPU sibling algorithm, so this checks its route and attributes without
/// building a source raster or effect passes.
fn plan_clip_member(layer: &Layer, doc: &Document) -> Result<(), GpuError> {
    if let Some(e) = own_attribute_decline(layer) {
        crate::metrics::count_layer_decline(&e);
        return Err(e);
    }
    let decline = |e: GpuError| {
        crate::metrics::count_layer_decline(&e);
        Err(e)
    };
    match content_route(layer) {
        ContentRoute::PassThroughGroup | ContentRoute::IsolatedGroup => {
            plan_clip_children(&layer.children, doc)
        }
        ContentRoute::Fill(adjustment) => {
            let fill = matches!(
                crate::fill::decode_layer_fill(layer),
                Some(
                    Adjustment::SolidFill(_)
                        | Adjustment::GradientFill(_)
                        | Adjustment::PatternFill(_)
                )
            ) && layer.rect.width() > 0
                && layer.rect.height() > 0;
            if adjustment_params(&adjustment).is_some()
                || pictura_adjust::adjustment_lut(&adjustment).is_some()
                || fill
            {
                Ok(())
            } else {
                decline(GpuError::UnsupportedAdjustment)
            }
        }
        ContentRoute::UndecodedAdjustment | ContentRoute::UndecodedVectorFill => {
            decline(GpuError::UnsupportedAdjustment)
        }
        ContentRoute::SmartFiltered => {
            if layer_source::filtered_source(layer, doc).is_some()
                || crate::channel_plane(layer, 0).is_some()
            {
                Ok(())
            } else {
                decline(GpuError::UnsupportedSmartObject)
            }
        }
        ContentRoute::SmartSource => {
            let embedded = layer
                .smart_object
                .as_ref()
                .and_then(layer_source::embedded_source)
                .is_some();
            if embedded || crate::channel_plane(layer, 0).is_some() {
                Ok(())
            } else {
                decline(GpuError::UnsupportedSmartObject)
            }
        }
        ContentRoute::TypeSource => {
            if layer_source::text_raster(layer).is_some()
                || crate::channel_plane(layer, 0).is_some()
            {
                Ok(())
            } else {
                decline(GpuError::UnsupportedText)
            }
        }
        ContentRoute::Pixels => Ok(()),
    }
}

/// Check a group's children for a clip member, honoring the same hidden-base
/// suppression as the top-level walk.
fn plan_clip_children(children: &[Layer], doc: &Document) -> Result<(), GpuError> {
    let mut i = 0;
    while i < children.len() {
        let base = &children[i];
        let run = children[i + 1..].iter().take_while(|l| l.clipping).count();
        if run == 0 || !base.visible {
            if base.visible {
                plan_clip_member(base, doc)?;
            }
            i += 1 + run;
            continue;
        }
        plan_clip_member(base, doc)?;
        for child in children.iter().skip(i + 1).take(run) {
            if child.visible {
                plan_clip_member(child, doc)?;
            }
        }
        i += 1 + run;
    }
    Ok(())
}

fn own_attribute_decline(layer: &Layer) -> Option<GpuError> {
    if matches!(layer.blend, BlendMode::Dissolve) {
        Some(GpuError::UnsupportedMode(layer.blend))
    } else if layer.blend_if.as_ref().is_some_and(|v| !v.is_default())
        || layer.knockout != Knockout::None
    {
        Some(GpuError::UnsupportedAdvancedBlending)
    } else {
        None
    }
}

fn record_plan(plan: &LayerPlan<'_>, first_err: &mut Option<GpuError>) {
    if let Body::Decline(e) = &plan.body {
        record_error(e, plan.inherited_decline, first_err);
    }
}

fn record_error(error: &GpuError, inherited: bool, first_err: &mut Option<GpuError>) {
    if !inherited {
        crate::metrics::count_layer_decline(error);
    }
    if first_err.is_none() {
        *first_err = Some(error.clone());
    }
}

fn decline<'a>(layer: &'a Layer, error: GpuError, inherited: bool) -> LayerPlan<'a> {
    LayerPlan {
        layer,
        effects: None,
        body: Body::Decline(error),
        inherited_decline: inherited,
    }
}
