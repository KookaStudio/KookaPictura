//! One per-composite resolution of the layer stack.
//!
//! A composite walks the tree once, resolving each layer's route, decoded
//! content, effects, passes, sources, mask gates, and native planes. Bands and
//! regions then execute the plan without repeating that work. See
//! `openspec/changes/composite-plan/design.md`.

use std::sync::Arc;

use pictura_adjust::Adjustment;
use pictura_core::{Document, Layer, PixelBuffer, PsdRect};

use crate::composite_blend::{BlendCtx, SourcePlanes};
use crate::composite_route::ContentRoute;
use crate::layer_source::{CachedBuffer, LayerPasses};

pub(crate) struct CompositePlan<'a> {
    pub(crate) roots: Vec<PlanNode<'a>>,
}

pub(crate) struct PlanNode<'a> {
    pub(crate) layer: &'a Layer,
    pub(crate) body: Body,
    pub(crate) passes: Option<Arc<LayerPasses>>,
    pub(crate) blend: Option<BlendCtx<'a>>,
    pub(crate) sources: Option<SourcePlanes<'a>>,
    pub(crate) children: Vec<PlanNode<'a>>,
}

pub(crate) enum Body {
    Hidden,
    Pixels,
    Fill(Arc<Adjustment>),
    UndecodedAdjustment,
    SmartFiltered {
        filtered: Option<CachedBuffer>,
        filter_base: Option<Arc<PixelBuffer>>,
        source: Option<Arc<PixelBuffer>>,
        type_raster: Option<Arc<PixelBuffer>>,
        has_proxy: bool,
    },
    SmartSource {
        source: Option<Arc<PixelBuffer>>,
        type_raster: Option<Arc<PixelBuffer>>,
        has_proxy: bool,
    },
    TypeSource {
        raster: Option<Arc<PixelBuffer>>,
        has_proxy: bool,
    },
    Group {
        pass_through: bool,
    },
}

impl<'a> PlanNode<'a> {
    fn placeholder(layer: &'a Layer) -> Self {
        Self {
            layer,
            body: Body::Hidden,
            passes: None,
            blend: None,
            sources: None,
            children: Vec::new(),
        }
    }
}

impl<'a> CompositePlan<'a> {
    pub(crate) fn build(doc: &'a Document, clip: PsdRect, effect_region: Option<PsdRect>) -> Self {
        Self {
            roots: plan_layers(&doc.layers, doc, clip, effect_region),
        }
    }

    /// A plan over `doc.layers[..end]`; later layers stay placeholders.
    pub(crate) fn build_prefix(
        doc: &'a Document,
        clip: PsdRect,
        effect_region: Option<PsdRect>,
        end: usize,
    ) -> Self {
        Self::build_selected(doc, clip, effect_region, |i| i < end)
    }

    /// A plan that resolves the document background and `doc.layers[start..]`;
    /// the layers between stay placeholders, so the suffix keeps its indices.
    pub(crate) fn build_suffix(
        doc: &'a Document,
        clip: PsdRect,
        effect_region: Option<PsdRect>,
        start: usize,
    ) -> Self {
        Self::build_selected(doc, clip, effect_region, |i| i == 0 || i >= start)
    }

    fn build_selected(
        doc: &'a Document,
        clip: PsdRect,
        effect_region: Option<PsdRect>,
        selected: impl Fn(usize) -> bool,
    ) -> Self {
        Self {
            roots: doc
                .layers
                .iter()
                .enumerate()
                .map(|(i, layer)| {
                    if selected(i) {
                        plan_node(layer, doc, clip, effect_region)
                    } else {
                        PlanNode::placeholder(layer)
                    }
                })
                .collect(),
        }
    }

    pub(crate) fn build_for_layers(
        layers: &[&'a Layer],
        doc: &'a Document,
        clip: PsdRect,
        effect_region: Option<PsdRect>,
    ) -> Vec<PlanNode<'a>> {
        layers
            .iter()
            .map(|layer| plan_node(layer, doc, clip, effect_region))
            .collect()
    }
}

fn plan_layers<'a>(
    layers: &'a [Layer],
    doc: &'a Document,
    clip: PsdRect,
    effect_region: Option<PsdRect>,
) -> Vec<PlanNode<'a>> {
    layers
        .iter()
        .map(|layer| plan_node(layer, doc, clip, effect_region))
        .collect()
}

fn plan_node<'a>(
    layer: &'a Layer,
    doc: &'a Document,
    clip: PsdRect,
    effect_region: Option<PsdRect>,
) -> PlanNode<'a> {
    if !layer.visible {
        return PlanNode {
            layer,
            body: Body::Hidden,
            passes: None,
            blend: None,
            sources: None,
            children: Vec::new(),
        };
    }
    #[cfg(test)]
    counters::note_plan(layer);
    let body = match crate::composite_route::content_route(layer) {
        ContentRoute::PassThroughGroup => Body::Group { pass_through: true },
        ContentRoute::IsolatedGroup => Body::Group {
            pass_through: false,
        },
        ContentRoute::Fill(adjustment) => Body::Fill(Arc::new(adjustment)),
        ContentRoute::UndecodedAdjustment => Body::UndecodedAdjustment,
        ContentRoute::SmartFiltered => {
            let filtered = crate::layer_source::filtered_source(layer, doc);
            let filter_base = match (&filtered, layer.smart_object.as_ref()) {
                (Some(cached), Some(so)) => {
                    let has_mask = so
                        .filter_mask
                        .as_ref()
                        .is_some_and(|_| so.filter_mask_enabled);
                    has_mask
                        .then(|| crate::render_smart_source(so, layer.rect, cached.rect))
                        .flatten()
                        .map(Arc::new)
                }
                _ => None,
            };
            let source = layer
                .smart_object
                .as_ref()
                .and_then(crate::layer_source::embedded_source);
            let type_raster = crate::layer_source::text_raster(layer);
            Body::SmartFiltered {
                filtered,
                filter_base,
                source,
                type_raster,
                has_proxy: crate::channel_plane(layer, 0).is_some(),
            }
        }
        ContentRoute::SmartSource => Body::SmartSource {
            source: layer
                .smart_object
                .as_ref()
                .and_then(crate::layer_source::embedded_source),
            type_raster: crate::layer_source::text_raster(layer),
            has_proxy: crate::channel_plane(layer, 0).is_some(),
        },
        ContentRoute::TypeSource => Body::TypeSource {
            raster: crate::layer_source::text_raster(layer),
            has_proxy: crate::channel_plane(layer, 0).is_some(),
        },
        ContentRoute::UndecodedVectorFill | ContentRoute::Pixels => Body::Pixels,
    };
    let effects = crate::layer_source::decoded_effects(layer);
    let fill = match &body {
        Body::Fill(adjustment) => Some(adjustment.as_ref()),
        _ => None,
    };
    let destructive = fill.is_some_and(|adjustment| !crate::fill::is_fill_kind(adjustment));
    let passes = (!layer.is_group && !destructive).then(|| {
        crate::layer_source::effect_passes_with(
            layer,
            doc,
            &effects,
            effect_region,
            destructive,
            fill,
        )
    });
    PlanNode {
        layer,
        body,
        passes,
        blend: Some(BlendCtx::new(layer, doc, clip)),
        sources: Some(SourcePlanes::resolve(layer, doc)),
        children: if layer.is_group {
            plan_layers(&layer.children, doc, clip, effect_region)
        } else {
            Vec::new()
        },
    }
}

#[cfg(test)]
pub(crate) mod counters {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    use pictura_core::{AdjustmentData, Layer};

    struct Keyed {
        map: OnceLock<Mutex<HashMap<usize, usize>>>,
    }

    impl Keyed {
        const fn new() -> Self {
            Self {
                map: OnceLock::new(),
            }
        }

        fn map(&self) -> &Mutex<HashMap<usize, usize>> {
            self.map.get_or_init(|| Mutex::new(HashMap::new()))
        }

        fn bump(&self, key: usize) {
            let mut map = self.map().lock().unwrap_or_else(|e| e.into_inner());
            *map.entry(key).or_insert(0) += 1;
        }

        fn get(&self, key: usize) -> usize {
            self.map()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&key)
                .copied()
                .unwrap_or(0)
        }

        fn reset(&self, key: usize) {
            self.map()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&key);
        }
    }

    static PLAN: Keyed = Keyed::new();
    static MASK_GATE: Keyed = Keyed::new();
    static NATIVE_PLANE: Keyed = Keyed::new();
    static ADJUSTMENT: Keyed = Keyed::new();
    static SOURCE: Keyed = Keyed::new();

    fn layer_key(layer: &Layer) -> usize {
        layer as *const Layer as usize
    }

    pub(crate) fn note_plan(layer: &Layer) {
        PLAN.bump(layer_key(layer));
    }

    pub(crate) fn plan_count(layer: &Layer) -> usize {
        PLAN.get(layer_key(layer))
    }

    pub(crate) fn reset_plan(layer: &Layer) {
        PLAN.reset(layer_key(layer));
    }

    pub(crate) fn note_mask_gate(layer: &Layer) {
        MASK_GATE.bump(layer_key(layer));
    }

    pub(crate) fn mask_gate_count(layer: &Layer) -> usize {
        MASK_GATE.get(layer_key(layer))
    }

    pub(crate) fn reset_mask_gate(layer: &Layer) {
        MASK_GATE.reset(layer_key(layer));
    }

    pub(crate) fn note_native_plane(layer: &Layer) {
        NATIVE_PLANE.bump(layer_key(layer));
    }

    pub(crate) fn native_plane_count(layer: &Layer) -> usize {
        NATIVE_PLANE.get(layer_key(layer))
    }

    pub(crate) fn reset_native_plane(layer: &Layer) {
        NATIVE_PLANE.reset(layer_key(layer));
    }

    pub(crate) fn note_adjustment(data: &AdjustmentData) {
        ADJUSTMENT.bump(data.data.ident() as usize);
    }

    pub(crate) fn adjustment_count(data: &AdjustmentData) -> usize {
        ADJUSTMENT.get(data.data.ident() as usize)
    }

    pub(crate) fn note_source_lookup(layer: &Layer) {
        SOURCE.bump(layer_key(layer));
    }

    pub(crate) fn source_lookup_count(layer: &Layer) -> usize {
        SOURCE.get(layer_key(layer))
    }

    pub(crate) fn reset_source_lookup(layer: &Layer) {
        SOURCE.reset(layer_key(layer));
    }
}
