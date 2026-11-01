//! Installing an [`AutoBlendPlan`]: masks, then the merged layer.

use pictura_core::{Document, Layer};

use super::super::layer_masks::replace_layer_mask;
use super::super::paths::parse_path;
use super::AutoBlendPlan;

/// The container of `segments`' node, top-level when `parent` is empty.
fn siblings<'a>(doc: &'a Document, parent: &[usize]) -> Option<&'a [Layer]> {
    let mut layers: &[Layer] = &doc.layers;
    for &index in parent {
        layers = &layers.get(index)?.children;
    }
    Some(layers)
}

/// Install `plan`. The merged layer goes above the topmost source and any clipped
/// run on it, replacing an earlier one of the same name found there. Nothing below
/// it moves, so returns the processed paths unchanged, bottom first.
pub fn apply_auto_blend(doc: &mut Document, plan: AutoBlendPlan) -> Vec<String> {
    let mut paths = Vec::with_capacity(plan.layers.len());
    for layer in plan.layers {
        if replace_layer_mask(doc, &layer.path, layer.mask) {
            paths.push(layer.path);
        }
    }
    let (Some(merged), Some(top)) = (plan.merged, paths.last().and_then(|p| parse_path(p))) else {
        return paths;
    };
    let Some((&index, parent)) = top.split_last() else {
        return paths;
    };
    let Some(layers) = siblings(doc, parent) else {
        return paths;
    };
    let mut slot = index + 1;
    while layers.get(slot).is_some_and(|layer| layer.clipping) {
        slot += 1;
    }
    let replaces = layers.get(slot).is_some_and(|l| l.name == merged.name);
    // Insert first: a failed insert keeps the earlier merged layer, and the masks
    // are a complete result on their own.
    if doc.layers.insert(parent, slot, merged).is_ok() && replaces {
        let mut stale = parent.to_vec();
        stale.push(slot + 1);
        doc.layers.remove(&stale);
    }
    paths
}
