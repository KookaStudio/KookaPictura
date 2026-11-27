use super::*;
use pictura_core::{path::Subpath, Layer, VectorFillRule, VectorMask};

const LABEL: &str = "Merge Shape Components";
const MASK_ROUND_TRIP_TOLERANCE: f64 = 1.0 / 256.0 - 1e-7;

pub(in crate::cxxqt_object::paths) fn path_merge_shape_components(
    mut view: Pin<&mut PictureView>,
) -> bool {
    !matches!(view.as_mut().edit(LABEL, apply_component_merge), Edit::None)
}

fn apply_component_merge(rust: &mut PictureViewRust) -> Edit {
    let layer_target = layer_target(rust);
    let Some(doc) = rust.session.doc.as_mut() else {
        return Edit::None;
    };
    if let Some(target) = layer_target {
        let (width, height) = (doc.width, doc.height);
        let Some(candidate) = pictura_render::resolve_path(doc, &target)
            .and_then(|layer| merged_layer(layer, width, height))
        else {
            return Edit::None;
        };
        let before = pictura_render::damage::content_influence(doc, &target);
        let Some(layer) = pictura_render::resolve_path_mut(doc, &target) else {
            return Edit::None;
        };
        *layer = candidate;
        rust.layer_path_editing = None;
        let after = pictura_render::damage::content_influence(doc, &target);
        return match pictura_render::damage::union(before, after) {
            Some(rect) => Edit::regions(vec![rect], true),
            None => Edit::Full,
        };
    }

    let Some(path) = active_path_mut(doc, rust.active_path) else {
        return Edit::None;
    };
    let Some(merged) =
        pictura_select::merge_shape_components(&path.subpaths, VectorFillRule::NonZero, None)
    else {
        return Edit::None;
    };
    path.subpaths = merged;
    path.finish_editing();
    Edit::Meta
}

fn merged_layer(layer: &Layer, width: u32, height: u32) -> Option<Layer> {
    if pictura_core::layer_pixel_locked(layer) {
        return None;
    }
    let mask = layer.vector_mask.as_ref()?;
    let rule = mask.subpaths.first()?.fill_rule;
    if mask
        .subpaths
        .iter()
        .any(|subpath| subpath.operation != 1 || subpath.fill_rule != rule)
    {
        return None;
    }
    let original = pictura_render::vector_mask_paths(layer, width, height)?;
    let block = layer
        .extra_block(b"vmsk")
        .or_else(|| layer.extra_block(b"vsms"))?;
    let canonical = pictura_codec::encode_vector_mask_with_rule(&original, width, height, rule);
    // The existing writer preserves header flags. Other records it cannot
    // reproduce could affect appearance in another PSD reader.
    if block.data.get(8..) != canonical.get(8..) {
        return None;
    }
    let result = pictura_select::merge_shape_components(&original, rule, Some(&mask.subpaths))?;
    let mut candidate = layer.clone();
    pictura_render::set_layer_shape_paths(&mut candidate, &result, width, height);
    let encoded = candidate.vector_mask.as_ref()?;
    if !mask_round_trip_matches(&result, &original, mask, encoded)
        || !pictura_select::mask_fills_match(&mask.subpaths, &encoded.subpaths, rule)
    {
        return None;
    }
    pictura_render::set_layer_live_shape(&mut candidate, None);
    Some(candidate)
}

fn mask_round_trip_matches(
    result: &[Subpath],
    original: &[Subpath],
    mask: &VectorMask,
    encoded: &VectorMask,
) -> bool {
    // An empty or all-open mask reveals the layer, even if the input contours
    // covered nothing because their winding counts cancelled.
    let has_closed = encoded
        .subpaths
        .iter()
        .any(|s| s.closed && s.points.len() >= 3);
    has_closed
        && result.len() == encoded.subpaths.len()
        && result.iter().zip(&encoded.subpaths).all(|(path, stored)| {
            if let Some(index) = original.iter().position(|p| p == path) {
                return stored == &mask.subpaths[index];
            }
            stored.closed
                && stored.points.len() == path.points.len() + 1
                && stored.points.windows(2).all(|edge| edge[0] != edge[1])
                && path
                    .points
                    .iter()
                    .zip(&stored.points)
                    .all(|(point, &[x, y])| {
                        let dx = point.anchor.0 - f64::from(x) / 256.0;
                        let dy = point.anchor.1 - f64::from(y) / 256.0;
                        dx.hypot(dy) <= MASK_ROUND_TRIP_TOLERANCE
                    })
        })
}

#[cfg(test)]
mod tests;
