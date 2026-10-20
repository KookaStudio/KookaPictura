//! Layer merge and flatten. See `docs/05-layers/merge-and-flatten.md`
//! and the `layers-panel-management` design D1–D3/D11.
//!
//! Every merge reuses [`crate::composite_rgba`] on a scratch `Document` whose
//! layers are exactly the inputs in stacking order; the result is the union of
//! the inputs' content rectangles cropped out of that buffer. There is no
//! second compositor.

use std::collections::HashMap;

use pictura_core::{
    BlendMode, Channel, ColorLabel, Document, Layer, LockFlags, PixelBuffer, PsdRect,
};

use super::create::{add_raster_layer_from_rgba, next_layer_name};
use super::paths::{flatten_rows, format_segments, parse_path, resolve_path, selected_paths};
use super::properties::move_path_to;

/// Which layers a merge consumes. The app resolves panel selection to paths and
/// passes them here; `pictura-render` cannot read the panel state.
#[derive(Debug)]
pub enum MergeScope<'a> {
    /// Merge the layer at the path with the layer directly below it.
    Down(&'a str),
    /// Merge a set of selected paths (≥2 distinct, resolving paths).
    Selected(&'a [String]),
    /// Merge every eye-visible layer; `&str` is the active layer's path.
    Visible(&'a str),
    /// Collapse the clipping group containing the path into its raster base.
    ClippingMask(&'a str),
}

/// A completed merge: the new node's path and how many inputs it replaced.
#[derive(Debug, PartialEq, Eq)]
pub struct MergeOutcome {
    pub path: String,
    pub replaced: usize,
}

/// A refused merge. Nothing is mutated on any of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeError {
    NoDocument,
    NoSelection,
    NoLayerBelow,
    InvalidTarget,
    NotClippable,
}

/// A merge target must be raster: not an adjustment and not a fill-content
/// layer. Groups are raster-mergeable (they collapse their children).
pub fn can_merge_target(layer: &Layer) -> bool {
    layer.adjustment.is_none()
}

/// The panel's effective eye: the layer's own visibility and every ancestor's.
pub fn is_visible_in_panel(doc: &Document, path: &str) -> bool {
    let Some(segments) = parse_path(path) else {
        return false;
    };
    let mut layers: &[Layer] = &doc.layers;
    let mut found = false;
    for &index in &segments {
        let Some(layer) = layers.get(index) else {
            return false;
        };
        if !layer.visible {
            return false;
        }
        found = true;
        layers = &layer.children;
    }
    found
}

/// Run one merge scope against `doc`.
pub fn merge_scope(doc: &mut Document, scope: MergeScope<'_>) -> Result<MergeOutcome, MergeError> {
    if doc.layers.is_empty() {
        return Err(MergeError::NoDocument);
    }
    match scope {
        MergeScope::Down(path) => merge_down(doc, path),
        MergeScope::Selected(paths) => merge_selected(doc, paths),
        MergeScope::Visible(path) => merge_visible(doc, path),
        MergeScope::ClippingMask(path) => merge_clipping_mask(doc, path),
    }
}

/// Whether [`merge_scope`] would accept `scope`, using the same read-only guards
/// with no mutation and no composite. The UI uses it to enable/disable a merge
/// command without doing the work.
pub fn can_merge_scope(doc: &Document, scope: &MergeScope<'_>) -> bool {
    if doc.layers.is_empty() {
        return false;
    }
    match scope {
        MergeScope::Down(path) => can_merge_down(doc, path),
        MergeScope::Selected(paths) => can_merge_selected(doc, paths),
        MergeScope::Visible(path) => is_visible_in_panel(doc, path),
        MergeScope::ClippingMask(path) => clipping_plan(doc, path).is_some(),
    }
}

fn can_merge_down(doc: &Document, path: &str) -> bool {
    let Some(mut segments) = parse_path(path) else {
        return false;
    };
    let Some(index) = segments.pop() else {
        return false;
    };
    if index == 0 {
        return false;
    }
    let lower_segments: Vec<usize> = segments.iter().copied().chain([index - 1]).collect();
    let upper_segments: Vec<usize> = segments.iter().copied().chain([index]).collect();
    let Some(lower) = resolve_path(doc, &format_segments(&lower_segments)) else {
        return false;
    };
    let Some(upper) = resolve_path(doc, &format_segments(&upper_segments)) else {
        return false;
    };
    can_merge_target(lower) && can_merge_target(upper)
}

fn can_merge_selected(doc: &Document, paths: &[String]) -> bool {
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    let selected = selected_paths(doc, &refs, false);
    if selected.len() < 2 {
        return false;
    }
    let ranks = stack_ranks(doc);
    let rank_of = |path: &str| ranks.get(path).copied().unwrap_or(usize::MAX);
    selected
        .iter()
        .max_by_key(|(_, path)| rank_of(path))
        .and_then(|(_, bottom_path)| resolve_path(doc, bottom_path))
        .is_some_and(can_merge_target)
}

/// Discard hidden layers, composite the visible tree over an opaque white
/// backdrop, and replace the whole tree with one `"Background"` pixel layer.
pub fn flatten(doc: &mut Document) -> Result<MergeOutcome, MergeError> {
    if doc.width == 0 || doc.height == 0 {
        return Err(MergeError::NoDocument);
    }
    let replaced = count_layers(&doc.layers);
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };

    // White backdrop first, then the tree; `composite_rgba` skips hidden layers.
    let mut scratch = Document::new(doc.width, doc.height, doc.mode, doc.depth);
    let mut scratch_layers = Vec::with_capacity(doc.layers.len() + 1);
    scratch_layers.push(opaque_white_layer(doc.width, doc.height));
    scratch_layers.extend(doc.layers.iter().cloned());
    scratch.layers = pictura_core::LayerStack::from_layers(scratch_layers);

    let buffer = crate::composite_rgba(&scratch);
    let channels = bake(&buffer, rect);
    let mut result = pixel_result(
        "Background".to_string(),
        rect,
        BlendMode::Normal,
        255,
        channels,
    );
    result.background = true;
    doc.layers = pictura_core::LayerStack::from_layers(vec![result]);

    Ok(MergeOutcome {
        path: "0".to_string(),
        replaced,
    })
}

// --- scopes -----------------------------------------------------------------

fn merge_down(doc: &mut Document, path: &str) -> Result<MergeOutcome, MergeError> {
    let mut segments = parse_path(path).ok_or(MergeError::InvalidTarget)?;
    let index = segments.pop().ok_or(MergeError::InvalidTarget)?;
    if index == 0 {
        return Err(MergeError::NoLayerBelow);
    }

    let lower_segments: Vec<usize> = segments.iter().copied().chain([index - 1]).collect();
    let upper_segments: Vec<usize> = segments.iter().copied().chain([index]).collect();
    let lower_path = format_segments(&lower_segments);

    let lower = resolve_path(doc, &lower_path).ok_or(MergeError::NoLayerBelow)?;
    let upper = resolve_path(doc, path).ok_or(MergeError::InvalidTarget)?;
    if !can_merge_target(lower) || !can_merge_target(upper) {
        return Err(MergeError::InvalidTarget);
    }

    // Result inherits the lower (replacement-position) layer's attributes.
    let inputs = vec![lower.clone(), upper.clone()];
    bake_inputs(
        doc,
        &inputs,
        &[lower_segments.clone(), upper_segments],
        &lower_segments,
        lower.name.clone(),
        lower.blend,
        lower.opacity,
    )
}

fn merge_selected(doc: &mut Document, paths: &[String]) -> Result<MergeOutcome, MergeError> {
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    let selected = selected_paths(doc, &refs, false);
    if selected.len() < 2 {
        return Err(MergeError::NoSelection);
    }

    let ranks = stack_ranks(doc);
    let rank_of = |path: &str| ranks.get(path).copied().unwrap_or(usize::MAX);
    let anchor = selected
        .iter()
        .min_by_key(|(_, path)| rank_of(path))
        .expect("non-empty selection");
    let bottom = selected
        .iter()
        .max_by_key(|(_, path)| rank_of(path))
        .expect("non-empty selection");

    let bottom_layer = resolve_path(doc, &bottom.1).ok_or(MergeError::NoSelection)?;
    if !can_merge_target(bottom_layer) {
        return Err(MergeError::InvalidTarget);
    }

    // Bottom-first stacking order for the scratch composite.
    let mut ordered: Vec<&(Vec<usize>, String)> = selected.iter().collect();
    ordered.sort_by_key(|(_, path)| std::cmp::Reverse(rank_of(path)));
    let inputs: Vec<Layer> = ordered
        .iter()
        .filter_map(|(_, path)| resolve_path(doc, path).cloned())
        .collect();

    let name = resolve_path(doc, &anchor.1)
        .ok_or(MergeError::NoSelection)?
        .name
        .clone();
    let remove: Vec<Vec<usize>> = selected
        .iter()
        .map(|(segments, _)| segments.clone())
        .collect();
    bake_inputs(
        doc,
        &inputs,
        &remove,
        &anchor.0,
        name,
        BlendMode::Normal,
        255,
    )
}

/// The eye-visible nodes in panel (topmost-first) order, excluding any node
/// whose ancestor is also visible. `None` when the active layer is hidden or
/// nothing is visible.
fn visible_nodes(doc: &Document, active: &str) -> Option<Vec<(Vec<usize>, String)>> {
    if !is_visible_in_panel(doc, active) {
        return None;
    }
    // `flatten_rows` is topmost-first, so an ancestor always precedes its
    // descendants: keep a visible node only when no kept node contains it.
    let mut selected: Vec<(Vec<usize>, String)> = Vec::new();
    for (path, _) in flatten_rows(doc) {
        if !is_visible_in_panel(doc, &path) {
            continue;
        }
        let Some(segments) = parse_path(&path) else {
            continue;
        };
        if selected
            .iter()
            .any(|(kept, _)| is_ancestor(kept, &segments))
        {
            continue;
        }
        selected.push((segments, path));
    }
    (!selected.is_empty()).then_some(selected)
}

fn merge_visible(doc: &mut Document, active: &str) -> Result<MergeOutcome, MergeError> {
    let selected = visible_nodes(doc, active).ok_or(MergeError::NoSelection)?;

    let (anchor_segments, anchor_path) = (selected[0].0.clone(), selected[0].1.clone());
    let anchor = resolve_path(doc, &anchor_path).ok_or(MergeError::NoSelection)?;
    let name = anchor.name.clone();
    let (blend, opacity) = (anchor.blend, anchor.opacity);

    // Reverse flatten order: bottom-first for the composite.
    let mut ordered: Vec<&(Vec<usize>, String)> = selected.iter().collect();
    ordered.reverse();
    let inputs: Vec<Layer> = ordered
        .iter()
        .filter_map(|(_, path)| resolve_path(doc, path).cloned())
        .collect();
    let remove: Vec<Vec<usize>> = selected.into_iter().map(|(segments, _)| segments).collect();

    bake_inputs(
        doc,
        &inputs,
        &remove,
        &anchor_segments,
        name,
        blend,
        opacity,
    )
}

fn merge_clipping_mask(doc: &mut Document, path: &str) -> Result<MergeOutcome, MergeError> {
    let plan = clipping_plan(doc, path).ok_or(MergeError::NotClippable)?;
    let ClippingPlan {
        parent,
        base_index,
        top,
        base_segments,
    } = plan;
    let base = resolve_path(doc, &format_segments(&base_segments))
        .ok_or(MergeError::NotClippable)?
        .clone();

    let mut inputs = Vec::with_capacity(top - base_index);
    inputs.push(base.clone());
    for at in base_index + 1..top {
        let mut p = parent.clone();
        p.push(at);
        let mut clipped = resolve_path(doc, &format_segments(&p))
            .ok_or(MergeError::NotClippable)?
            .clone();
        fold_clipping(&base, &mut clipped);
        inputs.push(clipped);
    }

    let remove: Vec<Vec<usize>> = (base_index..top)
        .map(|at| {
            let mut p = parent.clone();
            p.push(at);
            p
        })
        .collect();
    bake_inputs(
        doc,
        &inputs,
        &remove,
        &base_segments,
        base.name,
        base.blend,
        base.opacity,
    )
}

/// The validated shape of a clipping-mask collapse: the parent container, the
/// raster base's index and segments, and the exclusive top of the contiguous
/// clipped run directly above it. `None` when the collapse would be refused.
struct ClippingPlan {
    parent: Vec<usize>,
    base_index: usize,
    top: usize,
    base_segments: Vec<usize>,
}

fn clipping_plan(doc: &Document, path: &str) -> Option<ClippingPlan> {
    let segments = parse_path(path)?;
    let index = *segments.last()?;
    let parent: Vec<usize> = segments[..segments.len() - 1].to_vec();

    let siblings = child_count(doc, &parent)?;
    if index >= siblings {
        return None;
    }
    let is_clipping = |at: usize| -> bool {
        let mut p = parent.clone();
        p.push(at);
        resolve_path(doc, &format_segments(&p)).is_some_and(|layer| layer.clipping)
    };

    // The base is the nearest non-clipping sibling at or below the path.
    let mut base_index = index;
    while base_index > 0 && is_clipping(base_index) {
        base_index -= 1;
    }
    if is_clipping(base_index) {
        return None;
    }
    // Clipped siblings sit contiguously directly above the base.
    let mut top = base_index + 1;
    while top < siblings && is_clipping(top) {
        top += 1;
    }
    if top == base_index + 1 {
        return None;
    }

    let mut base_segments = parent.clone();
    base_segments.push(base_index);
    let base = resolve_path(doc, &format_segments(&base_segments))?;
    if base.is_group || !can_merge_target(base) {
        return None;
    }
    Some(ClippingPlan {
        parent,
        base_index,
        top,
        base_segments,
    })
}

// --- shared machinery -------------------------------------------------------

fn bake_inputs(
    doc: &mut Document,
    inputs: &[Layer],
    remove: &[Vec<usize>],
    anchor: &[usize],
    name: String,
    blend: BlendMode,
    opacity: u8,
) -> Result<MergeOutcome, MergeError> {
    let mut scratch = Document::new(doc.width, doc.height, doc.mode, doc.depth);
    scratch.layers = pictura_core::LayerStack::from_layers(inputs.to_vec());
    let buffer = crate::composite_rgba(&scratch);

    let union = inputs.iter().filter_map(content_rect).reduce(union_rect);
    let rect = union
        .and_then(|r| clamp_rect(r, doc.width, doc.height))
        .unwrap_or(PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        });
    let channels = bake(&buffer, rect);
    let node = pixel_result(name, rect, blend, opacity, channels);

    let path = splice_result(doc, remove, anchor, node).ok_or(MergeError::InvalidTarget)?;
    Ok(MergeOutcome {
        path,
        replaced: remove.len(),
    })
}

/// Remove every `remove` path, then insert `node` at the anchor's position in
/// its container (adjusted for the removals in that container).
fn splice_result(
    doc: &mut Document,
    remove: &[Vec<usize>],
    anchor: &[usize],
    node: Layer,
) -> Option<String> {
    let (anchor_index, parent) = anchor.split_last()?;
    let at = doc
        .layers
        .splice(remove, parent, *anchor_index, vec![node])
        .ok()?;
    let mut path = parent.to_vec();
    path.push(at);
    Some(format_segments(&path))
}

/// The panel stacking rank of every node: 0 is the topmost.
fn stack_ranks(doc: &Document) -> HashMap<String, usize> {
    flatten_rows(doc)
        .into_iter()
        .enumerate()
        .map(|(rank, (path, _))| (path, rank))
        .collect()
}

fn is_ancestor(ancestor: &[usize], descendant: &[usize]) -> bool {
    descendant.len() > ancestor.len() && descendant.starts_with(ancestor)
}

fn count_layers(layers: &[Layer]) -> usize {
    layers
        .iter()
        .map(|layer| 1 + count_layers(&layer.children))
        .sum()
}

fn child_count(doc: &Document, parent: &[usize]) -> Option<usize> {
    let mut layers: &[Layer] = &doc.layers;
    for &index in parent {
        layers = &layers.get(index)?.children;
    }
    Some(layers.len())
}

/// The bounding rectangle of a layer's raster content, recursing into groups.
/// Adjustments own no pixels and contribute nothing.
fn content_rect(layer: &Layer) -> Option<PsdRect> {
    if layer.is_group {
        return layer
            .children
            .iter()
            .filter_map(content_rect)
            .reduce(union_rect);
    }
    if layer.adjustment.is_some() {
        return None;
    }
    if layer.rect.width() > 0 && layer.rect.height() > 0 {
        Some(layer.rect)
    } else {
        None
    }
}

fn union_rect(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        top: a.top.min(b.top),
        left: a.left.min(b.left),
        bottom: a.bottom.max(b.bottom),
        right: a.right.max(b.right),
    }
}

fn clamp_rect(rect: PsdRect, width: u32, height: u32) -> Option<PsdRect> {
    let clamped = PsdRect {
        top: rect.top.max(0),
        left: rect.left.max(0),
        bottom: rect.bottom.min(height as i32),
        right: rect.right.min(width as i32),
    };
    (clamped.right > clamped.left && clamped.bottom > clamped.top).then_some(clamped)
}

/// Crop the full-document planar RGBA `buffer` to `rect` as `0/1/2/-1` channels.
fn bake(buffer: &PixelBuffer, rect: PsdRect) -> Vec<Channel> {
    let w = rect.width().max(0) as usize;
    let h = rect.height().max(0) as usize;
    let n = w * h;
    let plane = buffer.width as usize * buffer.height as usize;
    let mut channels = Vec::with_capacity(4);
    for (id, plane_index) in [(0i16, 0usize), (1, 1), (2, 2), (-1, 3)] {
        let mut data = vec![0u8; n];
        for y in 0..h {
            for x in 0..w {
                let sx = rect.left as usize + x;
                let sy = rect.top as usize + y;
                let source = sy * buffer.width as usize + sx;
                data[y * w + x] = buffer.data[plane_index * plane + source];
            }
        }
        channels.push(Channel {
            id,
            data: data.into(),
        });
    }
    channels
}

fn pixel_result(
    name: String,
    rect: PsdRect,
    blend: BlendMode,
    opacity: u8,
    channels: Vec<Channel>,
) -> Layer {
    Layer {
        name,
        rect,
        blend,
        opacity,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels,
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

fn opaque_white_layer(width: u32, height: u32) -> Layer {
    let n = width as usize * height as usize;
    let white = vec![255u8; n];
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    };
    Layer {
        name: "Backdrop".to_string(),
        rect,
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: white.clone().into(),
            },
            Channel {
                id: 1,
                data: white.clone().into(),
            },
            Channel {
                id: 2,
                data: white.clone().into(),
            },
            Channel {
                id: -1,
                data: white.into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

/// Mask a clipped sibling by the base's alpha (intersection). The compositor
/// does not render the `clipping` flag yet; this merge-local fold is the
/// documented gap.
//
// ponytail: compositor has no clipping pass; delete this fold when it does.
fn fold_clipping(base: &Layer, sibling: &mut Layer) {
    let bw = base.rect.width();
    let bh = base.rect.height();
    let sw = sibling.rect.width();
    let sh = sibling.rect.height();
    if sw <= 0 || sh <= 0 {
        return;
    }
    let base_alpha = crate::channel(base, -1);
    let alpha_index = match sibling.channels.iter().position(|c| c.id == -1) {
        Some(index) => index,
        None => {
            sibling.channels.push(Channel {
                id: -1,
                data: vec![255; (sw * sh) as usize].into(),
            });
            sibling.channels.len() - 1
        }
    };
    for y in 0..sh {
        for x in 0..sw {
            let bx = sibling.rect.left + x - base.rect.left;
            let by = sibling.rect.top + y - base.rect.top;
            let base_a = if bx >= 0 && by >= 0 && bx < bw && by < bh {
                base_alpha
                    .and_then(|data| data.get(by as usize * bw as usize + bx as usize))
                    .copied()
                    .unwrap_or(255)
            } else {
                0
            };
            let index = (y * sw + x) as usize;
            let current = sibling.channels[alpha_index].data[index];
            sibling.channels[alpha_index].data[index] =
                ((current as u32 * base_a as u32) / 255) as u8;
        }
    }
}

// --- stamping ---------------------------------------------------------------

/// Which layers [`stamp_scope`] flattens into a new layer. `Visible` anchors on
/// the active layer's panel visibility; `Selected` uses the panel selection.
#[derive(Debug)]
pub enum StampScope<'a> {
    Visible(&'a str),
    Selected(&'a [String]),
}

/// Composite the chosen layers and insert the result as a NEW raster layer
/// directly above `anchor`, leaving every original intact. `Visible` uses the
/// same eye-visible selection as [`merge_scope`]'s `Visible` scope; `Selected`
/// uses the given paths. Returns the new layer's path, or `None` when the
/// document is empty, the anchor does not resolve, or no input is eligible.
pub fn stamp_scope(doc: &mut Document, scope: StampScope<'_>, anchor: &str) -> Option<String> {
    if doc.width == 0 || doc.height == 0 || resolve_path(doc, anchor).is_none() {
        return None;
    }
    // Bottom-first stacking order for the scratch composite.
    let mut inputs: Vec<Layer> = match scope {
        StampScope::Visible(active) => {
            let mut nodes = visible_nodes(doc, active)?;
            nodes.reverse();
            nodes
                .iter()
                .filter_map(|(_, path)| resolve_path(doc, path).cloned())
                .collect()
        }
        StampScope::Selected(paths) => {
            let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
            let selected = selected_paths(doc, &refs, false);
            if selected.is_empty() {
                return None;
            }
            let mut nodes: Vec<Layer> = selected
                .iter()
                .filter_map(|(_, path)| resolve_path(doc, path).cloned())
                .collect();
            nodes.reverse();
            nodes
        }
    };
    if inputs.is_empty() {
        return None;
    }

    let mut scratch = Document::new(doc.width, doc.height, doc.mode, doc.depth);
    scratch.layers = pictura_core::LayerStack::from_layers(std::mem::take(&mut inputs));
    let buffer = crate::composite_rgba(&scratch);

    let name = next_layer_name(doc, "Stamp");
    let path = add_raster_layer_from_rgba(doc, &name, doc.width, doc.height, &packed_rgba(&buffer));
    if path.is_empty() {
        return None;
    }
    if !move_path_to(doc, &path, anchor, 0) {
        if let Some(segments) = parse_path(&path) {
            doc.layers.remove(&segments);
        }
        return None;
    }
    Some(path)
}

/// Pack a planar RGBA `PixelBuffer` (planes R, G, B, A) into RGBA8888 bytes.
fn packed_rgba(buffer: &PixelBuffer) -> Vec<u8> {
    let plane = buffer.width as usize * buffer.height as usize;
    let mut out = Vec::with_capacity(plane * 4);
    for index in 0..plane {
        out.push(buffer.data[index]);
        out.push(buffer.data[plane + index]);
        out.push(buffer.data[2 * plane + index]);
        out.push(buffer.data[3 * plane + index]);
    }
    out
}
