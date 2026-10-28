//! Session-scoped cache for encoded PSD layer-channel streams and the composite
//! image-data block.
//!
//! A key is the source plane's content identity ([`pictura_core::Plane::ident`])
//! plus every input that changes the encoded bytes. A reused stream is
//! byte-identical to a fresh encode. The cache is pruned to the keys the latest
//! successful encode used, so it tracks the live document rather than its edit
//! history. `ponytail: session-pruned cache, no byte budget; add an LRU budget
//! if profiling shows memory pressure.`

use pictura_core::Document;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Identity and encoding parameters for one cacheable layer channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ChannelKey {
    pub(crate) ident: u64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) depth: u16,
    pub(crate) compression: u16,
    pub(crate) psb: bool,
    /// `0` for a color channel, `1` for a layer mask.
    pub(crate) role: u8,
}

/// Identity and encoding parameters for a whole composite image-data block.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CompositeKey {
    pub(crate) ident: u64,
    pub(crate) extra_idents: Vec<u64>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) channels: u16,
    pub(crate) mode_code: u16,
    pub(crate) depth: u16,
    pub(crate) compression: u16,
    pub(crate) psb: bool,
}

/// Counters for profiling. `layers_encoded` counts layers with at least one
/// cache-missing channel in the latest encode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EncodeStats {
    pub hits: u64,
    pub misses: u64,
    pub layers_encoded: u64,
    pub bytes: u64,
    pub entries: u64,
}

#[derive(Default)]
struct Inner {
    channels: HashMap<ChannelKey, Arc<[u8]>>,
    composite: HashMap<CompositeKey, Arc<[u8]>>,
}

/// One document session's encoded-channel cache.
pub struct EncodeCache {
    inner: Mutex<Inner>,
    hits: AtomicU64,
    misses: AtomicU64,
    layers_encoded: AtomicU64,
}

impl Default for EncodeCache {
    fn default() -> Self {
        Self::new()
    }
}

impl EncodeCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            layers_encoded: AtomicU64::new(0),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn get_channel(&self, key: &ChannelKey) -> Option<Arc<[u8]>> {
        let inner = self.lock();
        match inner.channels.get(key) {
            Some(bytes) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(Arc::clone(bytes))
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    pub(crate) fn put_channel(&self, key: ChannelKey, bytes: Arc<[u8]>) {
        self.lock().channels.insert(key, bytes);
    }

    pub(crate) fn get_composite(&self, key: &CompositeKey) -> Option<Arc<[u8]>> {
        let inner = self.lock();
        match inner.composite.get(key) {
            Some(bytes) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(Arc::clone(bytes))
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    pub(crate) fn put_composite(&self, key: CompositeKey, bytes: Arc<[u8]>) {
        self.lock().composite.insert(key, bytes);
    }

    pub(crate) fn record_layers_encoded(&self, layers: u64) {
        self.layers_encoded.fetch_add(layers, Ordering::Relaxed);
    }

    /// Keep only the channel keys and composite block the successful encode used.
    pub(crate) fn prune(
        &self,
        used_channels: &[ChannelKey],
        used_composite: Option<&CompositeKey>,
    ) {
        let mut inner = self.lock();
        let used: HashSet<ChannelKey> = used_channels.iter().copied().collect();
        inner.channels.retain(|key, _| used.contains(key));
        match used_composite {
            Some(key) => inner.composite.retain(|stored, _| stored == key),
            None => inner.composite.clear(),
        }
    }

    pub fn stats(&self) -> EncodeStats {
        let inner = self.lock();
        let bytes = inner
            .channels
            .values()
            .map(|bytes| bytes.len() as u64)
            .sum::<u64>()
            + inner
                .composite
                .values()
                .map(|bytes| bytes.len() as u64)
                .sum::<u64>();
        EncodeStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            layers_encoded: self.layers_encoded.load(Ordering::Relaxed),
            bytes,
            entries: (inner.channels.len() + inner.composite.len()) as u64,
        }
    }
}

/// Assign stable content identities to every cacheable plane of the live
/// document. A clone copies the current ident value, so warm the source before
/// snapshotting it for a worker; otherwise the writer assigns fresh idents to
/// the clone and the next snapshot misses every channel.
pub fn warm_encode_idents(doc: &Document) {
    doc.composite.data.ident();
    for channel in &doc.channels {
        channel.data.ident();
    }
    fn walk(layers: &[pictura_core::Layer]) {
        for layer in layers {
            for channel in &layer.channels {
                channel.data.ident();
            }
            if let Some(data) = layer.mask.as_ref().and_then(|mask| mask.data.as_ref()) {
                data.ident();
            }
            walk(&layer.children);
        }
    }
    walk(&doc.layers);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode, Document, Layer, LayerStack};

    fn layer(name: &str, width: u32, height: u32, value: u8) -> Layer {
        let mut rgba = vec![value; width as usize * height as usize * 4];
        for alpha in rgba.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
        let doc = Document::from_rgba(name, width, height, &rgba);
        let mut layer = doc.layers.get(0).expect("from_rgba layer").clone();
        layer.name = name.to_string();
        for channel in &mut layer.channels {
            channel.data.as_mut_slice().fill(value);
        }
        layer
    }

    fn document(layers: usize) -> Document {
        let width = 24;
        let height = 18;
        let mut layers: Vec<Layer> = (0..layers)
            .map(|i| layer(&format!("Layer {i}"), width, height, (i * 7 + 3) as u8))
            .collect();
        layers.reverse();
        let rgba = vec![128u8; width as usize * height as usize * 4];
        let mut doc = Document::from_rgba("Layer 0", width, height, &rgba);
        doc.layers = LayerStack::from_layers(layers);
        doc
    }

    #[test]
    fn a_cache_hit_equals_a_fresh_encode() {
        let doc = document(4);
        let cache = EncodeCache::new();
        let first = crate::write_psd_cached(&doc, &cache).expect("cold encode");
        let warm = crate::write_psd_cached(&doc, &cache).expect("warm encode");
        let fresh = crate::write_psd(&doc).expect("uncached encode");
        assert_eq!(first, warm);
        assert_eq!(warm, fresh);
        assert!(cache.stats().hits > 0, "the warm encode hits");
    }

    #[test]
    fn an_edited_plane_is_re_encoded() {
        let mut doc = document(2);
        let cache = EncodeCache::new();
        let before = crate::write_psd_cached(&doc, &cache).expect("cold encode");
        let stats = cache.stats();
        doc.layers.get_mut(1).expect("second layer").channels[0]
            .data
            .as_mut_slice()
            .fill(41);
        let after = crate::write_psd_cached(&doc, &cache).expect("warm encode");
        assert_ne!(before, after);
        assert_eq!(cache.stats().layers_encoded - stats.layers_encoded, 1);
    }

    #[test]
    fn a_warmed_clone_hits_the_encode_cache() {
        let doc = document(4);
        warm_encode_idents(&doc);
        let cache = EncodeCache::new();
        let first_clone = doc.clone();
        let cold = crate::write_psd_cached(&first_clone, &cache).expect("cold clone encode");
        let before = cache.stats();
        let second_clone = doc.clone();
        let warm = crate::write_psd_cached(&second_clone, &cache).expect("warm clone encode");
        assert_eq!(cold, warm);
        assert_eq!(cache.stats().layers_encoded, before.layers_encoded);
        assert!(cache.stats().hits > before.hits, "the warmed clone hits");
    }

    #[test]
    fn parallel_encoding_matches_serial() {
        let doc = document(8);
        let parallel = crate::write_psd_cached(&doc, &EncodeCache::new()).expect("parallel encode");
        let serial = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("one-thread pool")
            .install(|| crate::write_psd_cached(&doc, &EncodeCache::new()))
            .expect("serial encode");
        assert_eq!(parallel, serial);
    }

    #[test]
    fn a_channel_error_fails_the_encode() {
        let row: Vec<u8> = (0..70_000).map(|i| (i % 251) as u8).collect();
        let error = crate::write::encode_scanlines(&[row.as_slice()], 70_000, 1, false);
        assert!(error.is_err(), "an oversized RLE row is rejected");
    }

    #[test]
    fn unused_entries_are_pruned() {
        let mut doc = document(4);
        let cache = EncodeCache::new();
        crate::write_psd_cached(&doc, &cache).expect("cold encode");
        let before = cache.stats().entries;
        let first = doc.layers.get(0).expect("layer").clone();
        doc.layers = LayerStack::from_layers(vec![first]);
        crate::write_psd_cached(&doc, &cache).expect("pruned encode");
        assert!(
            cache.stats().entries < before,
            "deleted layers leave the cache"
        );
    }

    #[test]
    fn only_the_edited_layer_is_encoded() {
        let mut doc = document(20);
        let cache = EncodeCache::new();
        crate::write_psd_cached(&doc, &cache).expect("cold encode");
        let before = cache.stats();
        doc.layers.get_mut(10).expect("layer").channels[1]
            .data
            .as_mut_slice()
            .fill(9);
        crate::write_psd_cached(&doc, &cache).expect("warm encode");
        assert_eq!(cache.stats().layers_encoded - before.layers_encoded, 1);
    }

    #[test]
    fn unchanged_cmyk_layer_reuses_retained_planes() {
        let raw = include_bytes!("../tests/fixtures/cmyk.psd");
        let doc = crate::read_psd(raw).expect("fixture reads");
        let layer = doc.layers.get(0).expect("fixture layer");
        let ids = [0i16, 1, 2].map(|id| {
            layer
                .channels
                .iter()
                .find(|channel| channel.id == id)
                .map(|channel| channel.data.ident())
                .unwrap_or(0)
        });
        assert!(ids.iter().all(|&id| id != 0), "working planes have idents");
        let mut layer = layer.clone();
        layer
            .source_channels
            .as_mut()
            .expect("source store")
            .working_idents = ids;
        let profile = crate::icc::cmyk_profile_from_resources(&doc);
        let retained: Vec<Vec<u8>> = (0..4)
            .map(|id| crate::color_mode::layer_retained_8(&layer, 8, id).expect("retained"))
            .collect();
        crate::write_cmyk::reset_forward_conversions();
        let planes = crate::write_cmyk::cmyk_layer_color_planes(&layer, 8, profile.as_ref());
        assert_eq!(planes, retained);
        assert_eq!(crate::write_cmyk::forward_conversions(), 0);
    }

    #[test]
    fn edited_cmyk_layer_is_converted() {
        let raw = include_bytes!("../tests/fixtures/cmyk.psd");
        let doc = crate::read_psd(raw).expect("fixture reads");
        let mut layer = doc.layers.get(0).expect("fixture layer").clone();
        let ids = [0i16, 1, 2].map(|id| {
            layer
                .channels
                .iter()
                .find(|channel| channel.id == id)
                .map(|channel| channel.data.ident())
                .unwrap_or(0)
        });
        layer
            .source_channels
            .as_mut()
            .expect("source store")
            .working_idents = ids;
        let retained: Vec<Vec<u8>> = (0..4)
            .map(|id| crate::color_mode::layer_retained_8(&layer, 8, id).expect("retained"))
            .collect();
        layer.channels[0].data.as_mut_slice().fill(7);
        let profile = crate::icc::cmyk_profile_from_resources(&doc);
        crate::write_cmyk::reset_forward_conversions();
        let planes = crate::write_cmyk::cmyk_layer_color_planes(&layer, 8, profile.as_ref());
        assert_ne!(planes, retained, "the edited layer is converted");
        assert_eq!(crate::write_cmyk::forward_conversions(), 0);
    }

    #[test]
    #[ignore = "24 MP profile; run manually with --ignored --nocapture"]
    fn profile_24mp_cold_and_warm_encode() {
        // ponytail: twenty 6000x4000 RGBA layers plus their converted copies
        // peak around 2 GB; keep the count down if this moves to CI.
        use std::time::Instant;
        let width = 6000;
        let height = 4000;
        let rgba = vec![128u8; width * height * 4];
        let mut doc = Document::new(width as u32, height as u32, ColorMode::Rgb, BitDepth::Eight);
        doc.composite = pictura_core::PixelBuffer {
            width: width as u32,
            height: height as u32,
            channels: 4,
            data: rgba.into(),
        };
        let base = layer("Layer 0", width as u32, height as u32, 30);
        let layers: Vec<Layer> = (0..20)
            .map(|i| {
                let mut layer = base.clone();
                layer.name = format!("Layer {i}");
                layer.channels[0].data.as_mut_slice().fill((i * 11) as u8);
                layer
            })
            .collect();
        doc.layers = LayerStack::from_layers(layers);
        let cache = EncodeCache::new();
        let cold = Instant::now();
        crate::write_psd_cached(&doc, &cache).expect("cold encode");
        let cold = cold.elapsed();
        let before = cache.stats().layers_encoded;
        doc.layers.get_mut(10).expect("layer").channels[0]
            .data
            .as_mut_slice()
            .fill(1);
        let warm = Instant::now();
        crate::write_psd_cached(&doc, &cache).expect("warm encode");
        let warm = warm.elapsed();
        println!(
            "profile_24mp_cold_and_warm_encode cold={cold:?} warm={warm:?} layers_encoded={}",
            cache.stats().layers_encoded - before
        );
        assert_eq!(cache.stats().layers_encoded - before, 1);
    }
}
