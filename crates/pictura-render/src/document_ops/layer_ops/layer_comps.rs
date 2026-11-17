//! Layer comp capture, apply, update, and validation.
//!
//! A comp stores only the recorded classes. Entries are keyed by the layer's
//! stable `lyid`; the document allocator assigns missing ids on first use.

use std::collections::{BTreeSet, HashSet};

use pictura_core::layer_comp::{
    ensure_layer_ids, layer_anchor, layer_ids, layer_paths_by_id, set_layer_anchor, BlendKey,
    CapturedState, CompEntry, CompRecord, LastDocumentState, LayerComp, PositionAnchor,
};
use pictura_core::{layer_move_locked, Document, Layer, LayerStack};

use super::layer_style::{layer_style_value, set_layer_style_value};
use super::paths::{resolve_path, resolve_path_mut};

/// The result of applying a comp or Last Document State.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ApplyOutcome {
    pub changed: usize,
    pub missing: usize,
}

impl ApplyOutcome {
    pub fn is_noop(self) -> bool {
        self.changed == 0
    }
}

fn anchor_for(doc: &Document, layer: &Layer) -> PositionAnchor {
    if crate::is_shape_layer(layer) {
        return crate::shape_bounds(layer, doc.width, doc.height)
            .map(|(left, top, _, _)| PositionAnchor { x: left, y: top })
            .unwrap_or_else(|| layer_anchor(layer));
    }
    layer_anchor(layer)
}

fn captured_state(doc: &Document, layer: &Layer, record: CompRecord) -> CapturedState {
    let visible = record.visibility.then_some(layer.visible);
    let position = record.position.then(|| anchor_for(doc, layer));
    let blend = record.appearance.then(|| BlendKey::from_layer(layer));
    let style_visible = record.appearance
        && layer.extra_block(b"lfx2").is_some()
        && layer_style_value(layer, "fx.visible").is_some();
    let style_visible = style_visible.then(|| layer_style_value(layer, "fx.visible") != Some(0.0));
    CapturedState {
        visible,
        position,
        blend,
        style_visible,
    }
}

fn capture_entries(doc: &Document, record: CompRecord) -> Vec<CompEntry> {
    fn walk(
        doc: &Document,
        layers: &LayerStack,
        prefix: &str,
        record: CompRecord,
        out: &mut Vec<CompEntry>,
    ) {
        for (index, layer) in layers.iter().enumerate() {
            let path = if prefix.is_empty() {
                index.to_string()
            } else {
                format!("{prefix}/{index}")
            };
            if let Some(layer_id) = pictura_core::layer_comp::layer_id(layer) {
                out.push(CompEntry {
                    layer_id,
                    state: captured_state(doc, layer, record),
                });
            }
            walk(doc, &layer.children, &path, record, out);
        }
    }
    let mut entries = Vec::new();
    walk(doc, &doc.layers, "", record, &mut entries);
    entries
}

/// Capture a comp from the current document. Pass `id = 0` to let the store
/// allocate one when the comp is pushed.
pub fn capture_layer_comp(
    doc: &mut Document,
    id: u32,
    name: impl Into<String>,
    comment: impl Into<String>,
    record: CompRecord,
) -> LayerComp {
    ensure_layer_ids(doc, doc.layer_comps.next_layer_id());
    let entries = capture_entries(doc, record);
    LayerComp::new(id, name, comment, record, entries)
}

/// Capture the current state of every layer across all three classes.
pub fn capture_last_document_state(doc: &mut Document) -> LastDocumentState {
    ensure_layer_ids(doc, doc.layer_comps.next_layer_id());
    LastDocumentState {
        entries: capture_entries(doc, CompRecord::all()),
    }
}

/// The identities that a comp references but the document no longer has.
pub fn layer_comp_missing_ids(doc: &Document, comp: &LayerComp) -> BTreeSet<u32> {
    let present = layer_ids(doc);
    comp.entries
        .iter()
        .filter(|entry| !present.contains(&entry.layer_id))
        .map(|entry| entry.layer_id)
        .collect()
}

/// Whether the comp should show the caution affordance.
pub fn layer_comp_warning(doc: &Document, comp: &LayerComp) -> bool {
    layer_comp_missing_ids(doc, comp)
        .iter()
        .any(|id| !comp.warning_cleared.contains(id))
}

fn translate_node(doc: &mut Document, path: &str, dx: i32, dy: i32) -> bool {
    let Some(layer) = resolve_path(doc, path) else {
        return false;
    };
    let locked = layer_move_locked(layer) || layer.background;
    let is_group = layer.is_group;
    let child_count = layer.children.len();
    let is_shape = crate::is_shape_layer(layer);
    let is_adjustment = layer.adjustment.is_some() && !is_shape;
    if locked {
        return false;
    }
    if is_group {
        let children: Vec<String> = (0..child_count)
            .map(|index| format!("{path}/{index}"))
            .collect();
        let mut moved = false;
        for child in children {
            moved |= translate_node(doc, &child, dx, dy);
        }
        if let Some(group) = resolve_path_mut(doc, path) {
            crate::document_ops::crop::offset_layer(group, dx, dy);
            moved = true;
        }
        return moved;
    }
    if is_adjustment {
        if let Some(adjustment) = resolve_path_mut(doc, path) {
            crate::document_ops::crop::offset_layer(adjustment, dx, dy);
        }
        return true;
    }
    crate::translate_layer_path(doc, path, dx, dy)
}

fn ancestor_moved(path: &str, moved: &HashSet<String>) -> bool {
    let mut current = path;
    while let Some((parent, _)) = current.rsplit_once('/') {
        if moved.contains(parent) {
            return true;
        }
        current = parent;
    }
    false
}

fn apply_entries(doc: &mut Document, record: CompRecord, entries: &[CompEntry]) -> ApplyOutcome {
    let paths = layer_paths_by_id(doc);
    let mut outcome = ApplyOutcome::default();
    // Groups before descendants so ancestor suppression sees the moved path.
    let mut ordered: Vec<&CompEntry> = entries.iter().collect();
    ordered.sort_by_key(|entry| {
        paths
            .get(&entry.layer_id)
            .map_or(0, |path| path.matches('/').count())
    });
    let mut moved: HashSet<String> = HashSet::new();

    for entry in ordered {
        let Some(path) = paths.get(&entry.layer_id) else {
            outcome.missing += 1;
            continue;
        };
        let state = &entry.state;
        let mut layer_changed = false;
        if let Some(layer) = resolve_path_mut(doc, path) {
            if record.visibility {
                if let Some(visible) = state.visible {
                    if layer.visible != visible {
                        layer.visible = visible;
                        layer_changed = true;
                    }
                }
            }
            if record.appearance {
                if let Some(blend) = state.blend {
                    let (mode, raw) = blend.layer_parts();
                    if layer.blend != mode || layer.blend_key != raw {
                        layer.blend = mode;
                        layer.blend_key = raw;
                        layer_changed = true;
                    }
                }
                if let Some(style_visible) = state.style_visible {
                    let current = layer_style_value(layer, "fx.visible").map(|value| value != 0.0);
                    if current != Some(style_visible)
                        && set_layer_style_value(
                            layer,
                            "fx.visible",
                            f64::from(u8::from(style_visible)),
                        )
                    {
                        layer_changed = true;
                    }
                }
            }
        }
        if layer_changed {
            outcome.changed += 1;
        }

        if record.position && !ancestor_moved(path, &moved) {
            if let (Some(captured), Some(current)) = (
                state.position,
                resolve_path(doc, path).map(|layer| anchor_for(doc, layer)),
            ) {
                let dx = (captured.x - current.x).round() as i32;
                let dy = (captured.y - current.y).round() as i32;
                if (dx != 0 || dy != 0) && translate_node(doc, path, dx, dy) {
                    if let Some(layer) = resolve_path_mut(doc, path) {
                        if layer.extra_block(b"fxrp").is_some() {
                            set_layer_anchor(layer, captured);
                        }
                    }
                    outcome.changed += 1;
                    moved.insert(path.to_string());
                }
            }
        }
    }
    outcome
}

/// Whether applying `comp` would change any recorded field.
pub fn layer_comp_changes(doc: &Document, comp: &LayerComp) -> bool {
    let paths = layer_paths_by_id(doc);
    for entry in &comp.entries {
        let Some(path) = paths.get(&entry.layer_id) else {
            continue;
        };
        let Some(layer) = resolve_path(doc, path) else {
            continue;
        };
        let state = &entry.state;
        if comp.record.visibility
            && state
                .visible
                .is_some_and(|visible| layer.visible != visible)
        {
            return true;
        }
        if comp.record.appearance {
            if let Some(blend) = state.blend {
                let (mode, raw) = blend.layer_parts();
                if layer.blend != mode || layer.blend_key != raw {
                    return true;
                }
            }
            if let Some(style) = state.style_visible {
                let current = layer_style_value(layer, "fx.visible").map(|value| value != 0.0);
                if current != Some(style) {
                    return true;
                }
            }
        }
        if comp.record.position {
            if let Some(captured) = state.position {
                let current = anchor_for(doc, layer);
                if (captured.x - current.x).round() != 0.0
                    || (captured.y - current.y).round() != 0.0
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Apply a comp's recorded classes. Missing layers are skipped and counted.
pub fn apply_layer_comp(doc: &mut Document, comp: &LayerComp) -> ApplyOutcome {
    apply_entries(doc, comp.record, &comp.entries)
}

/// Restore a captured Last Document State.
pub fn restore_last_document_state(doc: &mut Document, state: &LastDocumentState) -> ApplyOutcome {
    apply_entries(doc, CompRecord::all(), &state.entries)
}

/// Re-capture a comp and clear its warning.
pub fn update_layer_comp(doc: &mut Document, comp: &mut LayerComp) {
    comp.entries = capture_entries(doc, comp.record);
    comp.warning_cleared.clear();
    doc.layer_comps.mark_dirty();
}

/// Record the currently missing ids as dismissed for a comp.
pub fn clear_layer_comp_warning(doc: &mut Document, comp_id: u32) -> bool {
    let missing = doc
        .layer_comps
        .comp(comp_id)
        .map(|comp| layer_comp_missing_ids(doc, comp))
        .unwrap_or_default();
    doc.layer_comps.clear_comp_warning(comp_id, &missing)
}

/// Dismiss every comp's currently missing ids.
pub fn clear_all_layer_comp_warnings(doc: &mut Document) -> bool {
    let present = layer_ids(doc);
    let mut changed = false;
    let ids: Vec<u32> = doc.layer_comps.comps().iter().map(|comp| comp.id).collect();
    for id in ids {
        let missing = doc
            .layer_comps
            .comp(id)
            .map(|comp| {
                comp.entries
                    .iter()
                    .filter(|entry| !present.contains(&entry.layer_id))
                    .map(|entry| entry.layer_id)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        changed |= doc.layer_comps.clear_comp_warning(id, &missing);
    }
    changed
}

/// Prune dismissed ids that resolve again, so a later loss warns again.
pub fn prune_layer_comp_warnings(doc: &mut Document) -> bool {
    let present = layer_ids(doc);
    doc.layer_comps.prune_resolved_warnings(&present)
}

/// The captured anchor of a layer, or its rect fallback.
pub fn layer_comp_anchor(layer: &Layer) -> PositionAnchor {
    layer_anchor(layer)
}
