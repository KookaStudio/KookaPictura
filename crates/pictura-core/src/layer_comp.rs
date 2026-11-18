//! Layer comps: document-level saved layer states.
//!
//! A comp records only the classes it was created with (visibility, position,
//! appearance). Entries are keyed by the layer's stable `lyid` identity, which
//! is preserved in `Layer::extra_blocks`; `pictura-core` reads the block but
//! never parses PSD image resources.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::{BlendMode, Document, Layer, LayerStack, Plane};

/// The additional-layer-info key carrying a layer's stable identity.
pub const LYID: [u8; 4] = *b"lyid";

/// A stable per-layer identity (PSD `lyid`).
pub type LayerId = u32;

/// A comp's identity.
pub type CompId = u32;

/// Which classes a comp records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompRecord {
    pub visibility: bool,
    pub position: bool,
    pub appearance: bool,
}

impl CompRecord {
    pub fn all() -> Self {
        Self {
            visibility: true,
            position: true,
            appearance: true,
        }
    }

    /// The `capturedInfo` bit field (1 visibility, 2 position, 4 appearance).
    pub fn bits(self) -> u32 {
        (self.visibility as u32) | ((self.position as u32) << 1) | ((self.appearance as u32) << 2)
    }

    pub fn from_bits(bits: u32) -> Self {
        Self {
            visibility: bits & 1 != 0,
            position: bits & 2 != 0,
            appearance: bits & 4 != 0,
        }
    }
}

/// A layer position anchor. It is the layer's `fxrp` value when the layer has
/// one, otherwise a project convention derived from the layer rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionAnchor {
    pub x: f64,
    pub y: f64,
}

/// A blend mode as captured: a known mode, or an unrecognized raw PSD key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendKey {
    Mode(BlendMode),
    Raw([u8; 4]),
}

impl BlendKey {
    pub fn from_layer(layer: &Layer) -> Self {
        match layer.blend_key {
            Some(key) => BlendKey::Raw(key),
            None => BlendKey::Mode(layer.blend),
        }
    }

    /// The layer fields to write: the mode and the optional raw key.
    pub fn layer_parts(self) -> (BlendMode, Option<[u8; 4]>) {
        match self {
            BlendKey::Mode(mode) => (mode, None),
            BlendKey::Raw(key) => (BlendMode::Normal, Some(key)),
        }
    }
}

/// One layer's captured state. A field is present only when its class was
/// recorded.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CapturedState {
    pub visible: Option<bool>,
    pub position: Option<PositionAnchor>,
    pub blend: Option<BlendKey>,
    pub style_visible: Option<bool>,
}

impl CapturedState {
    pub fn is_empty(&self) -> bool {
        self.visible.is_none()
            && self.position.is_none()
            && self.blend.is_none()
            && self.style_visible.is_none()
    }
}

/// One captured layer.
#[derive(Debug, Clone, PartialEq)]
pub struct CompEntry {
    pub layer_id: LayerId,
    pub state: CapturedState,
}

/// A saved layer comp.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerComp {
    pub id: CompId,
    pub name: String,
    pub comment: String,
    pub record: CompRecord,
    pub entries: Vec<CompEntry>,
    /// Missing layer ids the user has already dismissed.
    pub warning_cleared: BTreeSet<LayerId>,
}

impl LayerComp {
    pub fn new(
        id: CompId,
        name: impl Into<String>,
        comment: impl Into<String>,
        record: CompRecord,
        entries: Vec<CompEntry>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            comment: comment.into(),
            record,
            entries,
            warning_cleared: BTreeSet::new(),
        }
    }

    pub fn entry(&self, layer_id: LayerId) -> Option<&CapturedState> {
        self.entries
            .iter()
            .find(|entry| entry.layer_id == layer_id)
            .map(|entry| &entry.state)
    }

    pub fn entry_mut(&mut self, layer_id: LayerId) -> Option<&mut CapturedState> {
        self.entries
            .iter_mut()
            .find(|entry| entry.layer_id == layer_id)
            .map(|entry| &mut entry.state)
    }

    pub fn set_entry(&mut self, layer_id: LayerId, state: CapturedState) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.layer_id == layer_id) {
            existing.state = state;
        } else {
            self.entries.push(CompEntry { layer_id, state });
        }
    }

    /// Whether any captured id fails to resolve in `doc`.
    pub fn is_missing(&self, doc: &Document) -> bool {
        let present = layer_ids(doc);
        self.entries
            .iter()
            .any(|entry| !present.contains(&entry.layer_id))
    }
}

/// The state captured for Last Document State.
#[derive(Debug, Clone, PartialEq)]
pub struct LastDocumentState {
    pub entries: Vec<CompEntry>,
}

impl LastDocumentState {
    pub fn entry(&self, layer_id: LayerId) -> Option<&CapturedState> {
        self.entries
            .iter()
            .find(|entry| entry.layer_id == layer_id)
            .map(|entry| &entry.state)
    }
}

/// The document-level comp store.
///
/// The comp list and Last Document State share their contents across clones
/// until one side edits them. `parsed`, `dirty`, `base_dirty`, and `ids_dirty`
/// are persistence bookkeeping and are deliberately excluded from equality.
#[derive(Debug, Clone, Default)]
pub struct LayerCompStore {
    comps: Arc<Vec<LayerComp>>,
    last_state: Option<Arc<LastDocumentState>>,
    next_comp_id: CompId,
    next_layer_id: LayerId,
    pub parsed: bool,
    pub dirty: bool,
    pub base_dirty: bool,
    pub ids_dirty: bool,
}

impl PartialEq for LayerCompStore {
    fn eq(&self, other: &Self) -> bool {
        self.comps == other.comps && self.last_state == other.last_state
    }
}

impl LayerCompStore {
    pub fn comps(&self) -> &[LayerComp] {
        &self.comps
    }

    pub fn comps_mut(&mut self) -> &mut Vec<LayerComp> {
        Arc::make_mut(&mut self.comps)
    }

    pub fn len(&self) -> usize {
        self.comps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.comps.is_empty()
    }

    pub fn comp(&self, id: CompId) -> Option<&LayerComp> {
        self.comps.iter().find(|comp| comp.id == id)
    }

    pub fn comp_mut(&mut self, id: CompId) -> Option<&mut LayerComp> {
        Arc::make_mut(&mut self.comps)
            .iter_mut()
            .find(|comp| comp.id == id)
    }

    pub fn alloc_comp_id(&mut self) -> CompId {
        let highest = self.comps.iter().map(|comp| comp.id).max().unwrap_or(0);
        let id = self.next_comp_id.max(highest.saturating_add(1)).max(1);
        self.next_comp_id = id.saturating_add(1);
        id
    }

    pub fn push_comp(&mut self, mut comp: LayerComp) -> CompId {
        if comp.id == 0 {
            comp.id = self.alloc_comp_id();
        } else if comp.id >= self.next_comp_id {
            self.next_comp_id = comp.id.saturating_add(1);
        }
        let id = comp.id;
        self.comps_mut().push(comp);
        self.dirty = true;
        self.parsed = true;
        id
    }

    pub fn remove_comp(&mut self, id: CompId) -> bool {
        let comps = self.comps_mut();
        let Some(index) = comps.iter().position(|comp| comp.id == id) else {
            return false;
        };
        comps.remove(index);
        self.dirty = true;
        true
    }

    pub fn rename_comp(&mut self, id: CompId, name: impl Into<String>) -> bool {
        let Some(comp) = self.comp_mut(id) else {
            return false;
        };
        comp.name = name.into();
        self.dirty = true;
        true
    }

    pub fn set_comment(&mut self, id: CompId, comment: impl Into<String>) -> bool {
        let Some(comp) = self.comp_mut(id) else {
            return false;
        };
        comp.comment = comment.into();
        self.dirty = true;
        true
    }

    pub fn set_record(&mut self, id: CompId, record: CompRecord) -> bool {
        let Some(comp) = self.comp_mut(id) else {
            return false;
        };
        comp.record = record;
        self.dirty = true;
        true
    }

    pub fn duplicate_comp(&mut self, id: CompId) -> Option<CompId> {
        let index = self.comps.iter().position(|comp| comp.id == id)?;
        let mut copy = self.comps[index].clone();
        copy.id = self.alloc_comp_id();
        copy.name = format!("{} copy", copy.name);
        copy.warning_cleared.clear();
        let copy_id = copy.id;
        self.comps_mut().insert(index + 1, copy);
        self.dirty = true;
        self.parsed = true;
        Some(copy_id)
    }

    /// Record the currently missing ids as dismissed for `id`.
    pub fn clear_comp_warning(&mut self, id: CompId, missing: &BTreeSet<LayerId>) -> bool {
        if missing.is_empty() {
            return false;
        }
        let Some(comp) = self.comp_mut(id) else {
            return false;
        };
        comp.warning_cleared.extend(missing.iter().copied());
        self.dirty = true;
        true
    }

    /// Prune dismissed ids that resolve again, so a later loss warns again.
    pub fn prune_resolved_warnings(&mut self, present: &BTreeSet<LayerId>) -> bool {
        let mut changed = false;
        for comp in self.comps_mut() {
            let before = comp.warning_cleared.len();
            comp.warning_cleared.retain(|id| !present.contains(id));
            changed |= comp.warning_cleared.len() != before;
        }
        if changed {
            self.dirty = true;
        }
        changed
    }

    pub fn last_state(&self) -> Option<&LastDocumentState> {
        self.last_state.as_deref()
    }

    pub fn set_last_state(&mut self, state: LastDocumentState) {
        self.last_state = Some(Arc::new(state));
    }

    pub fn clear_last_state(&mut self) {
        self.last_state = None;
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_base_dirty(&mut self) {
        self.base_dirty = true;
    }

    pub fn mark_ids_dirty(&mut self) {
        self.ids_dirty = true;
    }

    pub fn mark_parsed(&mut self) {
        self.parsed = true;
    }

    pub fn next_layer_id(&self) -> LayerId {
        self.next_layer_id
    }

    pub fn set_next_layer_id(&mut self, next: LayerId) {
        self.next_layer_id = next;
    }
}

/// Read `layer`'s stable identity from its `lyid` block.
pub fn layer_id(layer: &Layer) -> Option<LayerId> {
    let block = layer.extra_block(&LYID)?;
    let bytes = block.data.as_slice().get(..4)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Write `layer`'s stable identity, replacing an existing `lyid` block in place.
pub fn set_layer_id(layer: &mut Layer, id: LayerId) {
    let data: Plane<u8> = id.to_be_bytes().to_vec().into();
    layer.set_metadata_block(LYID, Some(data));
}

/// Remove `layer`'s stable identity.
pub fn clear_layer_id(layer: &mut Layer) {
    layer.set_metadata_block(LYID, None);
}

/// The layer's position anchor: its `fxrp` block when present, otherwise the
/// layer rect's top-left as a project convention.
pub fn layer_anchor(layer: &Layer) -> PositionAnchor {
    if let Some(block) = layer.extra_block(b"fxrp") {
        let bytes = block.data.as_slice();
        if let (Some(x), Some(y)) = (bytes.get(..8), bytes.get(8..16)) {
            return PositionAnchor {
                x: f64::from_be_bytes(x.try_into().expect("eight bytes")),
                y: f64::from_be_bytes(y.try_into().expect("eight bytes")),
            };
        }
    }
    PositionAnchor {
        x: f64::from(layer.rect.left),
        y: f64::from(layer.rect.top),
    }
}

/// Write the layer's position anchor into its `fxrp` block.
pub fn set_layer_anchor(layer: &mut Layer, anchor: PositionAnchor) {
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&anchor.x.to_be_bytes());
    data.extend_from_slice(&anchor.y.to_be_bytes());
    let data: Plane<u8> = data.into();
    layer.set_metadata_block(*b"fxrp", Some(data));
}

/// Shift the layer's position anchor by `(dx, dy)` when it has an `fxrp`
/// block. A layer without one falls back to its rect, which the caller moves.
pub fn bump_layer_anchor(layer: &mut Layer, dx: i32, dy: i32) {
    if layer.extra_block(b"fxrp").is_none() {
        return;
    }
    let anchor = layer_anchor(layer);
    set_layer_anchor(
        layer,
        PositionAnchor {
            x: anchor.x + f64::from(dx),
            y: anchor.y + f64::from(dy),
        },
    );
}

/// Walk the tree and report every present layer id.
pub fn layer_ids(doc: &Document) -> BTreeSet<LayerId> {
    fn walk(layers: &LayerStack, ids: &mut BTreeSet<LayerId>) {
        for layer in layers.iter() {
            if let Some(id) = layer_id(layer) {
                ids.insert(id);
            }
            walk(layer.children(), ids);
        }
    }
    let mut ids = BTreeSet::new();
    walk(&doc.layers, &mut ids);
    ids
}

/// The map from a layer id to its `/`-separated path, in bottom-first order.
pub fn layer_paths_by_id(doc: &Document) -> std::collections::BTreeMap<LayerId, String> {
    fn walk(
        layers: &LayerStack,
        prefix: &str,
        out: &mut std::collections::BTreeMap<LayerId, String>,
    ) {
        for (index, layer) in layers.iter().enumerate() {
            let path = if prefix.is_empty() {
                index.to_string()
            } else {
                format!("{prefix}/{index}")
            };
            if let Some(id) = layer_id(layer) {
                out.entry(id).or_insert_with(|| path.clone());
            }
            walk(layer.children(), &path, out);
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(&doc.layers, "", &mut out);
    out
}

/// Ensure every layer has a unique identity.
///
/// The first occurrence keeps its id; a missing or duplicate id gets a fresh
/// value from `seed` (the codec's resource-1044 seed) and the document counter.
/// Returns true when anything changed. This does not create a history state.
pub fn ensure_layer_ids(doc: &mut Document, seed: LayerId) -> bool {
    let mut used = BTreeSet::new();
    let mut highest = seed.saturating_sub(1);
    fn scan(layers: &LayerStack, used: &mut BTreeSet<LayerId>, highest: &mut LayerId) {
        for layer in layers.iter() {
            if let Some(id) = layer_id(layer) {
                used.insert(id);
                *highest = (*highest).max(id);
            }
            scan(layer.children(), used, highest);
        }
    }
    scan(&doc.layers, &mut used, &mut highest);
    let mut next = seed
        .max(doc.layer_comps.next_layer_id())
        .max(highest.saturating_add(1))
        .max(1);

    fn assign(layers: &mut LayerStack, used: &mut BTreeSet<LayerId>, next: &mut LayerId) -> bool {
        let mut changed = false;
        for layer in layers.iter_mut() {
            let keep = layer_id(layer).is_some_and(|id| used.insert(id));
            if !keep {
                while used.contains(next) {
                    *next = next.saturating_add(1);
                }
                set_layer_id(layer, *next);
                used.insert(*next);
                *next = next.saturating_add(1);
                changed = true;
            }
            if let Some(children) = layer.children_mut() {
                changed |= assign(children, used, next);
            }
        }
        changed
    }

    // The first pass above records every existing id, so the assign pass must
    // start from a clean "seen" set that keeps the first occurrence.
    used.clear();
    let changed = assign(&mut doc.layers, &mut used, &mut next);
    doc.layer_comps.set_next_layer_id(next);
    if changed {
        doc.layer_comps.mark_ids_dirty();
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BitDepth, ColorMode};

    fn layer(name: &str, id: Option<LayerId>) -> Layer {
        let mut layer = crate::Layer::from_parts(
            crate::LayerParts {
                name: name.to_string(),
                rect: crate::PsdRect {
                    top: 0,
                    left: 0,
                    bottom: 4,
                    right: 4,
                },
                ..Default::default()
            },
            crate::LayerContent::Pixel,
        );
        if let Some(id) = id {
            set_layer_id(&mut layer, id);
        }
        layer
    }

    fn document(layers: Vec<Layer>) -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(layers);
        doc
    }

    fn comp(id: CompId, name: &str) -> LayerComp {
        LayerComp::new(
            id,
            name,
            "",
            CompRecord {
                visibility: true,
                ..Default::default()
            },
            Vec::new(),
        )
    }

    #[test]
    fn layer_comp_store_starts_empty() {
        let store = LayerCompStore::default();
        assert!(store.is_empty());
        assert!(store.last_state().is_none());
    }

    #[test]
    fn layer_comp_store_is_shared_by_document_clones() {
        let mut doc = document(vec![layer("one", Some(1))]);
        doc.layer_comps.push_comp(comp(1, "A"));
        let mut clone = doc.clone();
        clone.layer_comps.push_comp(comp(2, "B"));
        assert_eq!(doc.layer_comps.len(), 1);
        assert_eq!(clone.layer_comps.len(), 2);
    }

    #[test]
    fn layer_comp_store_equality_ignores_bookkeeping() {
        let mut a = LayerCompStore::default();
        a.push_comp(comp(1, "A"));
        let mut b = a.clone();
        b.parsed = true;
        b.dirty = true;
        b.base_dirty = true;
        b.ids_dirty = true;
        assert_eq!(a, b);
    }

    #[test]
    fn layer_comp_records_only_enabled_classes() {
        let record = CompRecord {
            visibility: true,
            ..Default::default()
        };
        assert_eq!(record.bits(), 1);
        assert_eq!(CompRecord::from_bits(7), CompRecord::all());
    }

    #[test]
    fn layer_id_helpers_read_and_write_lyid() {
        let mut layer = layer("one", None);
        assert_eq!(layer_id(&layer), None);
        set_layer_id(&mut layer, 42);
        assert_eq!(layer_id(&layer), Some(42));
        clear_layer_id(&mut layer);
        assert_eq!(layer_id(&layer), None);
    }

    #[test]
    fn layer_id_allocator_assigns_missing_ids() {
        let mut doc = document(vec![layer("one", Some(7)), layer("two", None)]);
        assert!(ensure_layer_ids(&mut doc, 7));
        assert_eq!(layer_id(&doc.layers[0]), Some(7));
        let assigned = layer_id(&doc.layers[1]).expect("assigned");
        assert_ne!(assigned, 7);
        assert!(doc.layer_comps.ids_dirty);
    }

    #[test]
    fn comp_ids_are_not_reused() {
        let mut store = LayerCompStore::default();
        let first = store.push_comp(comp(0, "A"));
        assert!(store.remove_comp(first));
        let second = store.push_comp(comp(0, "B"));
        assert_ne!(first, second);
    }

    #[test]
    fn document_clone_still_shares_the_comp_store() {
        let mut doc = document(vec![layer("one", Some(1))]);
        doc.layer_comps.push_comp(comp(1, "A"));
        let clone = doc.clone();
        assert_eq!(doc.layer_comps, clone.layer_comps);
    }
}
