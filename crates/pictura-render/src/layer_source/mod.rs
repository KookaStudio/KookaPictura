//! Content-keyed cache for per-layer render sources and effect passes.
//!
//! One process-wide map from a content key to an `Arc<OnceLock<Artifact>>`.
//! A lookup holds the map lock only long enough to clone the cell; the build
//! runs outside it, so a recursive source (an embedded document composited by
//! the renderer) cannot deadlock the cache. There is no thread-local front
//! cache: lookups happen once per layer per composite, not once per pixel, and
//! a front cache would defeat the byte budget.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use pictura_core::{Document, Layer, PixelBuffer, PsdRect, SmartObject};

use crate::layer_effects::{EffectPass, LayerEffects};

pub(crate) mod key;
#[cfg(test)]
mod tests;

/// The default resident-byte budget.
const DEFAULT_BUDGET: usize = 256 * 1024 * 1024;
/// Hard cap on map entries, so zero-byte misses cannot grow it forever.
const MAX_ENTRIES: usize = 4096;
static BUDGET: AtomicUsize = AtomicUsize::new(DEFAULT_BUDGET);

fn budget() -> usize {
    BUDGET.load(Ordering::Relaxed)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Embedded,
    Filtered,
    Text,
    Effects,
    Passes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    kind: Kind,
    hash: u64,
}

impl Key {
    fn new(kind: Kind, hash: u64) -> Self {
        Self { kind, hash }
    }
}

pub(crate) enum Artifact {
    Missing,
    Buffer(Arc<PixelBuffer>),
    Effects(Arc<LayerEffects>),
    Passes(Arc<LayerPasses>),
}

impl Artifact {
    fn size(&self) -> usize {
        match self {
            Self::Missing => 0,
            Self::Buffer(buffer) => buffer.data.len(),
            Self::Effects(_) => std::mem::size_of::<LayerEffects>(),
            Self::Passes(passes) => passes.size(),
        }
    }
}

impl Clone for Artifact {
    fn clone(&self) -> Self {
        match self {
            Self::Missing => Self::Missing,
            Self::Buffer(buffer) => Self::Buffer(Arc::clone(buffer)),
            Self::Effects(effects) => Self::Effects(Arc::clone(effects)),
            Self::Passes(passes) => Self::Passes(Arc::clone(passes)),
        }
    }
}

/// The pass groups a layer's effects build, in draw order.
#[derive(Default)]
pub(crate) struct LayerPasses {
    pub(crate) below: Vec<EffectPass>,
    pub(crate) above: Vec<EffectPass>,
}

impl LayerPasses {
    fn size(&self) -> usize {
        self.below
            .iter()
            .chain(&self.above)
            .map(EffectPass::size)
            .sum()
    }
}

/// A cached buffer and the document-space rectangle it covers.
pub(crate) struct CachedBuffer {
    pub(crate) rect: PsdRect,
    pub(crate) buffer: Arc<PixelBuffer>,
}

struct Entry {
    key: Key,
    cell: Arc<OnceLock<Artifact>>,
    bytes: usize,
}

struct Cache {
    entries: Vec<Entry>,
    bytes: usize,
}

impl Cache {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            bytes: 0,
        }
    }

    fn touch(&mut self, i: usize) -> Arc<OnceLock<Artifact>> {
        let entry = self.entries.remove(i);
        let cell = Arc::clone(&entry.cell);
        self.entries.push(entry);
        cell
    }

    fn note_size(&mut self, key: Key, size: usize) {
        let Some(i) = self.entries.iter().position(|entry| entry.key == key) else {
            return;
        };
        self.bytes = self.bytes.saturating_sub(self.entries[i].bytes);
        self.entries[i].bytes = 0;
        if size > budget() {
            self.entries.remove(i);
            crate::metrics::set_source_bytes(self.bytes);
            return;
        }
        self.entries[i].bytes = size;
        self.bytes += size;
        while (self.bytes > budget() || self.entries.len() > MAX_ENTRIES)
            && !self.entries.is_empty()
        {
            let entry = self.entries.remove(0);
            self.bytes = self.bytes.saturating_sub(entry.bytes);
        }
        crate::metrics::set_source_bytes(self.bytes);
    }
}

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Cache::new()))
}

fn lookup(key: Key, build: impl FnOnce() -> Artifact) -> Artifact {
    let cell = {
        let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
        let found = cache.entries.iter().position(|entry| entry.key == key);
        let cell = match found {
            Some(i) => cache.touch(i),
            None => {
                let cell = Arc::new(OnceLock::new());
                cache.entries.push(Entry {
                    key,
                    cell: Arc::clone(&cell),
                    bytes: 0,
                });
                cell
            }
        };
        match cell.get() {
            Some(_) => crate::metrics::count_source_hit(),
            None => crate::metrics::count_source_miss(),
        }
        cell
    };
    let artifact = cell.get_or_init(build).clone();
    let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
    cache.note_size(key, artifact.size());
    artifact
}

/// Drop every cached artifact. The next request rebuilds it.
pub fn purge_source_cache() {
    let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
    cache.entries.clear();
    cache.bytes = 0;
    crate::metrics::set_source_bytes(0);
}

fn cached_buffer(
    kind: Kind,
    hash: u64,
    build: impl FnOnce() -> Option<PixelBuffer>,
) -> Option<Arc<PixelBuffer>> {
    match lookup(Key::new(kind, hash), || match build() {
        Some(buffer) => Artifact::Buffer(Arc::new(buffer)),
        None => Artifact::Missing,
    }) {
        Artifact::Buffer(buffer) => Some(buffer),
        Artifact::Missing => None,
        Artifact::Effects(_) | Artifact::Passes(_) => {
            debug_assert!(false, "cache kind collision");
            None
        }
    }
}

/// The decoded embedded source of `so` at its own resolution, or `None` when
/// it is not an embedded object, has an empty payload, or does not decode.
pub(crate) fn embedded_source(so: &SmartObject) -> Option<Arc<PixelBuffer>> {
    if so.kind != pictura_core::SmartObjectKind::Embedded {
        return None;
    }
    so.payload().filter(|p| !p.is_empty())?;
    cached_buffer(Kind::Embedded, key::embedded_key(so), || {
        decode_embedded(so.payload()?)
    })
}

fn decode_embedded(payload: &[u8]) -> Option<PixelBuffer> {
    // ponytail: an embedded source that itself holds a no-proxy smart object
    // recurses without a depth guard; a crafted cyclic payload can re-enter
    // this OnceLock. Add a depth pass-through when untrusted embeddings appear.
    let embedded = pictura_codec::read_psd(payload).ok()?;
    let comp = &embedded.composite;
    let usable = comp.width > 0
        && comp.height > 0
        && comp.channels >= 1
        && comp.data.len() == comp.width as usize * comp.height as usize * comp.channels as usize;
    if embedded.merged_composite_present && usable {
        Some(comp.clone())
    } else {
        Some(crate::composite_rgba(&embedded))
    }
}

/// The smart-filter chain output for `layer` at `layer.rect ∩ document`, or
/// `None` when the embedded source cannot render.
pub(crate) fn filtered_source(layer: &Layer, doc: &Document) -> Option<CachedBuffer> {
    let so = layer.smart_object.as_ref()?;
    let rect = clip_to_document(layer.rect, doc);
    if rect.width() <= 0 || rect.height() <= 0 {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    so.payload_digest().hash(&mut hasher);
    rect_key(layer.rect).hash(&mut hasher);
    doc.width.hash(&mut hasher);
    doc.height.hash(&mut hasher);
    key::chain_hash(so).hash(&mut hasher);
    key::text_state_hash().hash(&mut hasher);
    let hash = hasher.finish();
    let buffer = cached_buffer(Kind::Filtered, hash, || {
        let base = crate::smart_source::render_smart_source(so, layer.rect, rect)?;
        Some(crate::smart_filter::apply_smart_filter_chain(
            &base,
            &so.smart_filters,
            so.smart_filters_enabled,
        ))
    })?;
    Some(CachedBuffer { rect, buffer })
}

/// The type-layer raster at `layer.rect`, or `None` when nothing paints.
pub(crate) fn text_raster(layer: &Layer) -> Option<Arc<PixelBuffer>> {
    let type_tool = layer.type_tool.as_ref()?;
    let width = layer.rect.width();
    let height = layer.rect.height();
    if width <= 0 || height <= 0 {
        return None;
    }
    let hash = key::text_key(type_tool, layer.rect);
    cached_buffer(Kind::Text, hash, || {
        crate::text_render::render_text_buffer(type_tool, width, height)
    })
}

/// The decoded effects descriptor for `layer`, eagerly caching a successful
/// decode by the authoritative block bytes.
pub(crate) fn decoded_effects(layer: &Layer) -> Arc<LayerEffects> {
    if layer.extra_block(b"lfx2").is_none() && layer.extra_block(b"lrFX").is_none() {
        return Arc::new(LayerEffects::default());
    }
    let hash = key::effect_digest(layer);
    match lookup(Key::new(Kind::Effects, hash), || {
        crate::metrics::count_effect_decode();
        Artifact::Effects(Arc::new(crate::layer_effects::decode_layer_effects(layer)))
    }) {
        Artifact::Effects(effects) => effects,
        Artifact::Missing | Artifact::Buffer(_) | Artifact::Passes(_) => {
            debug_assert!(false, "cache kind collision");
            Arc::new(LayerEffects::default())
        }
    }
}

/// The built pass groups for `layer`. Pattern-driven passes are rebuilt on
/// every call instead of cached.
pub(crate) fn effect_passes(
    layer: &Layer,
    doc: &Document,
    effects: &LayerEffects,
    region: Option<PsdRect>,
) -> Arc<LayerPasses> {
    if crate::layer_effects::no_passes(layer, effects) {
        return Arc::new(LayerPasses::default());
    }
    let build = || {
        Artifact::Passes(Arc::new(crate::layer_effects::build_layer_passes(
            layer, doc, effects, region,
        )))
    };
    // A sub-region pass is built for this composite only: retaining it would
    // let a stroke's ever-changing rectangles evict other layers' sources.
    // Pattern-driven passes are rebuilt for the same reason.
    if crate::layer_effects::reads_pattern(layer, effects) || region.is_some() {
        return match build() {
            Artifact::Passes(passes) => passes,
            Artifact::Missing | Artifact::Buffer(_) | Artifact::Effects(_) => {
                Arc::new(LayerPasses::default())
            }
        };
    }
    let mut hasher = DefaultHasher::new();
    key::content_key(layer, doc).hash(&mut hasher);
    key::effect_digest(layer).hash(&mut hasher);
    let hash = hasher.finish();
    match lookup(Key::new(Kind::Passes, hash), build) {
        Artifact::Passes(passes) => passes,
        Artifact::Missing | Artifact::Buffer(_) | Artifact::Effects(_) => {
            Arc::new(LayerPasses::default())
        }
    }
}

pub(crate) fn clip_to_document(rect: PsdRect, doc: &Document) -> PsdRect {
    PsdRect {
        top: rect.top.max(0),
        left: rect.left.max(0),
        bottom: rect.bottom.min(doc.height.min(i32::MAX as u32) as i32),
        right: rect.right.min(doc.width.min(i32::MAX as u32) as i32),
    }
}

fn rect_key(rect: PsdRect) -> [i32; 4] {
    [rect.left, rect.top, rect.right, rect.bottom]
}

#[cfg(test)]
fn cache_len() -> usize {
    cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entries
        .len()
}

#[cfg(test)]
fn cache_has_key(key: Key) -> bool {
    cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entries
        .iter()
        .any(|entry| entry.key == key)
}

#[cfg(test)]
pub(crate) fn pass_cache_len() -> usize {
    cache_kind_count(Kind::Passes)
}

#[cfg(test)]
fn cache_kind_count(kind: Kind) -> usize {
    cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entries
        .iter()
        .filter(|entry| entry.key.kind == kind)
        .count()
}

#[cfg(test)]
fn set_budget_for_test(bytes: usize) {
    BUDGET.store(bytes, Ordering::Relaxed);
}

#[cfg(test)]
fn restore_budget_for_test() {
    BUDGET.store(DEFAULT_BUDGET, Ordering::Relaxed);
}
