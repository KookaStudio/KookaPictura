//! Content-keyed cache for per-layer render sources and effect passes.
//!
//! One process-wide map from a content key to an `Arc<Slot>`. A lookup holds
//! the map lock only long enough to clone the slot, and the build runs outside
//! every lock, so a recursive source (an embedded document composited by the
//! renderer) cannot deadlock the cache. A rayon worker never waits on another
//! caller's in-flight build (see [`Slot::resolve`]). There is no thread-local
//! front cache: lookups happen once per layer per composite, not once per
//! pixel, and a front cache would defeat the byte budget.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

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
    Composited,
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
    /// A content identity for the buffer, so the GPU resident store can key an
    /// upload without plane stamps.
    pub(crate) content: u64,
    /// Whether a whole-document composite may retain the upload. A pattern
    /// fill reads the document pattern library, which the key does not cover.
    pub(crate) retain: bool,
}

enum State {
    Empty,
    /// Claimed by the named thread's build.
    Building(std::thread::ThreadId),
    Ready(Artifact),
}

/// One key's artifact, built at most once by callers outside the worker pool.
struct Slot {
    state: Mutex<State>,
    ready: Condvar,
}

/// Resets an abandoned claim, so a build that panics lets a waiter retry.
struct Claim<'a>(&'a Slot);

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        let mut state = self.0.lock();
        if matches!(*state, State::Building(_)) {
            *state = State::Empty;
        }
        self.0.ready.notify_all();
    }
}

impl Slot {
    fn new() -> Self {
        Self {
            state: Mutex::new(State::Empty),
            ready: Condvar::new(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The published artifact, building it on a miss.
    ///
    /// A caller outside the pool waits for an in-flight build instead of
    /// duplicating it. A rayon worker never waits: builds run rayon work, so a
    /// worker waiting inside a join can run a queued job that looks up the key
    /// it is itself building, or one whose build waits on the half of its
    /// join that this worker holds (#1044). Such a worker builds its own copy;
    /// the first artifact published is the one every caller returns. No thread
    /// waits on a build it owns either: a crafted payload whose digest
    /// collides with an enclosing one re-enters its own key.
    fn resolve(&self, build: impl FnOnce() -> Artifact) -> Artifact {
        let mut state = self.lock();
        if let State::Ready(artifact) = &*state {
            crate::metrics::count_source_hit();
            return artifact.clone();
        }
        crate::metrics::count_source_miss();
        let me = std::thread::current().id();
        if rayon::current_thread_index().is_none() {
            while matches!(*state, State::Building(owner) if owner != me) {
                state = self.ready.wait(state).unwrap_or_else(|e| e.into_inner());
            }
            if let State::Ready(artifact) = &*state {
                return artifact.clone();
            }
        }
        let claim = matches!(*state, State::Empty).then(|| {
            *state = State::Building(me);
            Claim(self)
        });
        drop(state);
        let built = build();
        let mut state = self.lock();
        let artifact = match &*state {
            State::Ready(first) => first.clone(),
            State::Empty | State::Building(_) => {
                *state = State::Ready(built.clone());
                built
            }
        };
        drop(state);
        drop(claim);
        artifact
    }
}

struct Entry {
    key: Key,
    cell: Arc<Slot>,
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

    fn touch(&mut self, i: usize) -> Arc<Slot> {
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

/// Test-only purge lock: a cache test can hold this so another test's
/// `purge_source_cache` cannot clear the entry it is asserting on. Lookups are
/// not gated, so rayon workers never block on it.
#[cfg(test)]
static PURGE_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
thread_local! {
    static HOLDING_PURGE_LOCK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) struct CacheTestGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl Drop for CacheTestGuard {
    fn drop(&mut self) {
        HOLDING_PURGE_LOCK.with(|h| h.set(false));
    }
}

/// Hold the purge lock for the rest of the test's cache-sensitive section.
#[cfg(test)]
pub(crate) fn cache_test_guard() -> CacheTestGuard {
    HOLDING_PURGE_LOCK.with(|h| h.set(true));
    CacheTestGuard {
        _guard: PURGE_LOCK.lock().unwrap_or_else(|e| e.into_inner()),
    }
}

fn lookup(key: Key, build: impl FnOnce() -> Artifact) -> Artifact {
    let cell = {
        let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
        let found = cache.entries.iter().position(|entry| entry.key == key);
        match found {
            Some(i) => cache.touch(i),
            None => {
                let cell = Arc::new(Slot::new());
                cache.entries.push(Entry {
                    key,
                    cell: Arc::clone(&cell),
                    bytes: 0,
                });
                cell
            }
        }
    };
    let artifact = cell.resolve(build);
    let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
    cache.note_size(key, artifact.size());
    artifact
}

/// Drop every cached artifact. The next request rebuilds it.
pub fn purge_source_cache() {
    #[cfg(test)]
    let _guard = if HOLDING_PURGE_LOCK.with(|h| h.get()) {
        None
    } else {
        Some(PURGE_LOCK.lock().unwrap_or_else(|e| e.into_inner()))
    };
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

/// The deepest embedded object that still decodes. A payload comes from the
/// opened file, so nesting is attacker-controlled; each level renders on the
/// stack of the level that encloses it.
pub(crate) const MAX_EMBED_NESTING: u8 = 8;

/// The decoded embedded source of `so` at its own resolution, or `None` when
/// it is not an embedded object, has an empty payload, does not decode, or is
/// nested deeper than [`MAX_EMBED_NESTING`].
pub(crate) fn embedded_source(so: &SmartObject) -> Option<Arc<PixelBuffer>> {
    if so.kind != pictura_core::SmartObjectKind::Embedded || so.nesting >= MAX_EMBED_NESTING {
        return None;
    }
    so.payload().filter(|p| !p.is_empty())?;
    cached_buffer(Kind::Embedded, key::embedded_key(so), || {
        decode_embedded(so)
    })
}

fn decode_embedded(so: &SmartObject) -> Option<PixelBuffer> {
    let mut embedded = pictura_codec::read_psd(so.payload()?).ok()?;
    // The depth travels in the data, not the call stack: a nested composite
    // may run on another pool thread.
    let mut stacks = vec![&mut embedded.layers];
    while let Some(stack) = stacks.pop() {
        for layer in stack.iter_mut() {
            if let Some(inner) = layer.smart_object.as_mut() {
                inner.nesting = so.nesting + 1;
            }
            stacks.push(&mut layer.children);
        }
    }
    let comp = &embedded.composite;
    let usable = comp.width > 0
        && comp.height > 0
        && comp.channels >= 1
        && comp.data.len() == comp.width as usize * comp.height as usize * comp.channels as usize;
    if embedded.merged_composite_present && usable {
        // ponytail: depth-1 Bitmap extras stay packed, not 8-bit alpha.
        let merged = embedded.source_mode != Some(pictura_core::ColorMode::Bitmap);
        Some(if merged {
            with_merged_alpha(comp, &embedded.channels)
        } else {
            comp.clone()
        })
    } else {
        Some(crate::composite_rgba(&embedded))
    }
}

/// The stored merged composite with its first extra merged plane appended as
/// alpha. `read_psd` splits the merged image data into color planes and
/// document-level extra planes, so a flattened transparent source carries its
/// transparency in `channels`, not in `composite`.
///
/// ponytail: the first extra merged plane is treated as transparency, matching
/// `materialize_merged_composite`. A saved-selection or spot plane is
/// indistinguishable here; upgrade by reading alpha-channel metadata or a
/// background-layer discriminator if a real file shows it.
fn with_merged_alpha(comp: &PixelBuffer, channels: &[pictura_core::Channel]) -> PixelBuffer {
    let plane = comp.width as usize * comp.height as usize;
    if plane == 0 || !matches!(comp.channels, 1 | 3) {
        return comp.clone();
    }
    let Some(alpha) = channels.first().filter(|c| c.data.len() >= plane) else {
        return comp.clone();
    };
    let colors = comp.channels as usize;
    let mut out = PixelBuffer::new(comp.width, comp.height, comp.channels + 1);
    out.data[..colors * plane].copy_from_slice(&comp.data[..colors * plane]);
    out.data[colors * plane..].copy_from_slice(&alpha.data[..plane]);
    out
}

/// The smart-filter chain output for `layer` at `layer.rect ∩ document`, or
/// `None` when the embedded source cannot render.
pub(crate) fn filtered_source(layer: &Layer, doc: &Document) -> Option<CachedBuffer> {
    #[cfg(test)]
    crate::composite_plan::counters::note_source_lookup(layer);
    let so = layer.smart_object.as_ref()?;
    let rect = layer.rect.clip_to(doc.width, doc.height)?;
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
    Some(CachedBuffer {
        rect,
        buffer,
        content: hash,
        retain: true,
    })
}

/// Which cached raster a GPU route needs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RasterRoute {
    Type,
    SmartSource,
    SmartFiltered,
}

/// The exact straight-alpha RGBA the CPU blends for a proxy-less type layer, a
/// channel-less smart-object source, or a smart-filtered source (including the
/// smart-filter mask mix), cached by content.
pub(crate) fn composited_source(
    layer: &Layer,
    doc: &Document,
    route: RasterRoute,
) -> Option<CachedBuffer> {
    match route {
        RasterRoute::Type => {
            let type_tool = layer.type_tool.as_ref()?;
            let buffer = text_raster(layer)?;
            // ponytail: the CPU text renderer is box-sized, so it builds the
            // full layer raster; the GPU upload clips it to the document.
            Some(CachedBuffer {
                rect: layer.rect,
                buffer,
                content: key::text_key(type_tool, layer.rect),
                retain: true,
            })
        }
        RasterRoute::SmartSource => {
            let so = layer.smart_object.as_ref()?;
            let rect = layer.rect.clip_to(doc.width, doc.height)?;
            let mut hasher = DefaultHasher::new();
            key::embedded_key(so).hash(&mut hasher);
            rect_key(layer.rect).hash(&mut hasher);
            rect_key(rect).hash(&mut hasher);
            doc.width.hash(&mut hasher);
            doc.height.hash(&mut hasher);
            let hash = hasher.finish();
            let buffer = cached_buffer(Kind::Composited, hash, || {
                crate::smart_source::render_smart_source(so, layer.rect, rect)
            })?;
            Some(CachedBuffer {
                rect,
                buffer,
                content: hash,
                retain: true,
            })
        }
        RasterRoute::SmartFiltered => {
            let so = layer.smart_object.as_ref()?;
            let filtered = filtered_source(layer, doc)?;
            let has_mask = so
                .filter_mask
                .as_ref()
                .is_some_and(|_| so.filter_mask_enabled);
            if !has_mask {
                return Some(filtered);
            }
            let mut hasher = DefaultHasher::new();
            filtered.content.hash(&mut hasher);
            if let Some(mask) = so.filter_mask.as_ref() {
                rect_key(mask.rect).hash(&mut hasher);
                mask.data.ident().hash(&mut hasher);
                mask.data.len().hash(&mut hasher);
            }
            so.filter_mask_enabled.hash(&mut hasher);
            so.filter_mask_extend_with_white.hash(&mut hasher);
            let hash = hasher.finish();
            let buffer = cached_buffer(Kind::Composited, hash, || {
                let base = crate::smart_source::render_smart_source(so, layer.rect, filtered.rect)?;
                let full = &filtered.buffer;
                let fw = full.width as usize;
                let plane = fw * full.height as usize;
                let mut out = PixelBuffer::new(full.width, full.height, full.channels);
                out.data.copy_from_slice(&full.data);
                for y in filtered.rect.top..filtered.rect.bottom {
                    for x in filtered.rect.left..filtered.rect.right {
                        let i = (y - filtered.rect.top) as usize * fw
                            + (x - filtered.rect.left) as usize;
                        let m = crate::smart_filter::mask_value(so, x, y);
                        if m < 1.0 {
                            for k in 0..4 {
                                let b = f32::from(base.data[k * plane + i]);
                                let f = f32::from(full.data[k * plane + i]);
                                out.data[k * plane + i] =
                                    (b + (f - b) * m).round().clamp(0.0, 255.0) as u8;
                            }
                        }
                    }
                }
                Some(out)
            })?;
            Some(CachedBuffer {
                rect: filtered.rect,
                buffer,
                content: hash,
                retain: true,
            })
        }
    }
}

/// The type-layer raster at `layer.rect`, or `None` when nothing paints.
pub(crate) fn text_raster(layer: &Layer) -> Option<Arc<PixelBuffer>> {
    let type_tool = layer.type_tool.as_ref()?;
    #[cfg(test)]
    crate::composite_plan::counters::note_source_lookup(layer);
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

/// The content key of `layer`'s whole-layer effect passes.
fn pass_key_hash(layer: &Layer, doc: &Document) -> u64 {
    let mut hasher = DefaultHasher::new();
    key::content_key(layer, doc).hash(&mut hasher);
    key::effect_digest(layer).hash(&mut hasher);
    hasher.finish()
}

/// The built pass groups for `layer`. Pattern-driven passes are rebuilt on
/// every call instead of cached.
pub(crate) fn effect_passes(
    layer: &Layer,
    doc: &Document,
    effects: &LayerEffects,
    region: Option<PsdRect>,
) -> Arc<LayerPasses> {
    let destructive = crate::layer_effects::is_destructive_adjustment(layer);
    effect_passes_with(layer, doc, effects, region, destructive, None)
}

/// The ready whole-layer pass for `key`, without building, waiting, or
/// inserting. A miss is an absent, empty, building, or non-pass slot.
fn ready_pass(key: Key) -> Option<Arc<LayerPasses>> {
    let cell = {
        let cache = cache().lock().unwrap_or_else(|e| e.into_inner());
        cache
            .entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| Arc::clone(&entry.cell))
    };
    let Some(cell) = cell else {
        crate::metrics::count_source_miss();
        return None;
    };
    let ready = {
        let state = cell.lock();
        match &*state {
            State::Ready(Artifact::Passes(passes)) => Some(Arc::clone(passes)),
            State::Empty | State::Building(_) | State::Ready(_) => None,
        }
    };
    match ready {
        Some(passes) => {
            crate::metrics::count_source_hit();
            let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
            if let Some(i) = cache.entries.iter().position(|entry| entry.key == key) {
                cache.touch(i);
            }
            Some(passes)
        }
        None => {
            crate::metrics::count_source_miss();
            None
        }
    }
}

/// [`effect_passes`] when the caller already knows whether the layer is a
/// destructive adjustment, so it does not decode the adjustment again.
pub(crate) fn effect_passes_with(
    layer: &Layer,
    doc: &Document,
    effects: &LayerEffects,
    region: Option<PsdRect>,
    destructive: bool,
    fill: Option<&pictura_adjust::Adjustment>,
) -> Arc<LayerPasses> {
    if crate::layer_effects::no_passes(layer, effects, destructive) {
        return Arc::new(LayerPasses::default());
    }
    #[cfg(test)]
    use crate::tests::effect_preparation::probe::{observe, Event};
    #[cfg(test)]
    observe(doc, Some(layer), Event::Lookup(region));
    let key = Key::new(Kind::Passes, pass_key_hash(layer, doc));
    let build = || {
        #[cfg(test)]
        observe(doc, Some(layer), Event::BuildStart(region));
        let passes = Arc::new(crate::layer_effects::build_layer_passes(
            layer, doc, effects, region, fill,
        ));
        #[cfg(test)]
        observe(doc, Some(layer), Event::BuildEnd);
        Artifact::Passes(passes)
    };
    // Pattern-driven passes read the document pattern library, which the key
    // does not cover, so they are rebuilt rather than probed or retained.
    if crate::layer_effects::reads_pattern(layer, effects) {
        let passes = match build() {
            Artifact::Passes(passes) => passes,
            Artifact::Missing | Artifact::Buffer(_) | Artifact::Effects(_) => {
                Arc::new(LayerPasses::default())
            }
        };
        #[cfg(test)]
        observe(
            doc,
            Some(layer),
            Event::Ready(Arc::as_ptr(&passes) as usize),
        );
        return passes;
    }
    // A short region replays the retained whole-layer pass when one is ready.
    // On a miss it builds transient passes: retaining a region pass would let
    // a stroke's ever-changing rectangles evict other layers' sources, and
    // waiting for a whole-layer build would make a small region pay for it.
    let passes = if region.is_some() {
        match ready_pass(key) {
            Some(passes) => passes,
            None => match build() {
                Artifact::Passes(passes) => passes,
                Artifact::Missing | Artifact::Buffer(_) | Artifact::Effects(_) => {
                    Arc::new(LayerPasses::default())
                }
            },
        }
    } else {
        match lookup(key, build) {
            Artifact::Passes(passes) => passes,
            Artifact::Missing | Artifact::Buffer(_) | Artifact::Effects(_) => {
                Arc::new(LayerPasses::default())
            }
        }
    };
    #[cfg(test)]
    observe(
        doc,
        Some(layer),
        Event::Ready(Arc::as_ptr(&passes) as usize),
    );
    passes
}

#[cfg(test)]
pub(crate) struct TestBudget(usize);

#[cfg(test)]
impl TestBudget {
    pub(crate) fn new(bytes: usize) -> Self {
        Self(BUDGET.swap(bytes, Ordering::Relaxed))
    }
}

#[cfg(test)]
impl Drop for TestBudget {
    fn drop(&mut self) {
        BUDGET.store(self.0, Ordering::Relaxed);
        purge_source_cache();
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

/// Pass-cache entries for one layer's content key, so a test that asserts a
/// single pass is not racing every other test that builds a styled layer.
#[cfg(test)]
fn pass_builds() -> &'static Mutex<std::collections::HashMap<u64, usize>> {
    static BUILDS: OnceLock<Mutex<std::collections::HashMap<u64, usize>>> = OnceLock::new();
    BUILDS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// Test-only: count one built pass group, cached or transient, by content key.
#[cfg(test)]
pub(crate) fn note_pass_build(layer: &Layer, doc: &Document) {
    let mut builds = pass_builds().lock().unwrap_or_else(|e| e.into_inner());
    *builds.entry(pass_key_hash(layer, doc)).or_insert(0) += 1;
}

/// How many times `layer`'s whole-layer pass was actually built, so a test
/// that asserts one build is not racing every other cache test's purge.
#[cfg(test)]
pub(crate) fn pass_build_count(layer: &Layer, doc: &Document) -> usize {
    pass_builds()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&pass_key_hash(layer, doc))
        .copied()
        .unwrap_or(0)
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
