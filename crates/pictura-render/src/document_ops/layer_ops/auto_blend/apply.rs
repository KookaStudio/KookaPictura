//! Installing an [`AutoBlendPlan`]: masks first, then correction layers.

use pictura_core::{Document, Layer};

use super::super::layer_masks::replace_layer_mask;
use super::super::paths::{parse_path, resolve_path};
use super::{AutoBlendPlan, AUTO_BLEND_TONES};

fn is_correction(layer: &Layer) -> bool {
    layer.name == AUTO_BLEND_TONES
        && layer.clipping
        && layer.adjustment.as_ref().is_some_and(|a| a.key == *b"curv")
}

fn correction_layer(adjustment: pictura_core::AdjustmentData) -> Layer {
    Layer {
        name: AUTO_BLEND_TONES.into(),
        opacity: 255,
        fill: 255,
        visible: true,
        clipping: true,
        adjustment: Some(adjustment),
        ..Default::default()
    }
}

/// One change to a container: `net` layers more directly above `index`.
struct Shift {
    parent: Vec<usize>,
    index: usize,
    net: isize,
}

/// Where `segments` lands after `shifts`, which name pre-edit positions.
fn remap(segments: &[usize], shifts: &[Shift]) -> String {
    let moved: Vec<String> = segments
        .iter()
        .enumerate()
        .map(|(depth, &index)| {
            let net: isize = shifts
                .iter()
                .filter(|s| s.index < index && s.parent == segments[..depth])
                .map(|s| s.net)
                .sum();
            (index as isize + net).to_string()
        })
        .collect();
    moved.join("/")
}

/// Install `plan`, replacing an earlier correction directly above each layer
/// even when this run adds none. Returns each processed layer's path afterwards,
/// bottom first. Works top-down so an edit never moves a path still to come.
pub fn apply_auto_blend(doc: &mut Document, plan: AutoBlendPlan) -> Vec<String> {
    let mut ordered = Vec::with_capacity(plan.layers.len());
    for layer in plan.layers {
        let Some(segments) = parse_path(&layer.path) else {
            continue;
        };
        if replace_layer_mask(doc, &layer.path, layer.mask) {
            ordered.push((segments, layer.correction));
        }
    }
    let originals: Vec<Vec<usize>> = ordered.iter().map(|(s, _)| s.clone()).collect();
    let mut shifts = Vec::new();
    for (segments, correction) in ordered.into_iter().rev() {
        let Some((&index, parent)) = segments.split_last() else {
            continue;
        };
        let mut above = segments.clone();
        above[parent.len()] = index + 1;
        let joined = above
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("/");
        let mut net = 0isize;
        if resolve_path(doc, &joined).is_some_and(is_correction)
            && doc.layers.remove(&above).is_some()
        {
            net -= 1;
        }
        if let Some(adjustment) = correction {
            if doc
                .layers
                .insert(parent, index + 1, correction_layer(adjustment))
                .is_ok()
            {
                net += 1;
            }
        }
        if net != 0 {
            shifts.push(Shift {
                parent: parent.to_vec(),
                index,
                net,
            });
        }
    }
    originals.iter().map(|s| remap(s, &shifts)).collect()
}
