//! Process-wide compositing counters for the opt-in metrics report
//! (`PICTURA_METRICS`).
//!
//! The counters always count: each costs one relaxed atomic operation per
//! composite, canvas, or upload, never per pixel, and counting unconditionally
//! keeps the tests that read them independent of the environment. Only the
//! GPU device timestamps are gated on [`enabled`], because requesting them
//! changes how the device is created.

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::OnceLock;

use crate::GpuError;

/// Whether `PICTURA_METRICS` was a non-zero integer at first use.
pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        std::env::var("PICTURA_METRICS")
            .ok()
            .and_then(|v| v.trim().parse::<i64>().ok())
            .is_some_and(|v| v != 0)
    })
}

/// The decline reasons, in the order [`Counters::declines`] reports them.
pub const DECLINE_REASONS: [&str; 11] = [
    "unavailable",
    "too_large",
    "readback",
    "unsupported_mode",
    "unsupported_adjustment",
    "unsupported_smart_object",
    "type_source",
    "unsupported_layer_effect",
    "unsupported_advanced_blending",
    "unsupported_depth",
    "unsupported_smart_filter",
];

fn reason_index(err: &GpuError) -> usize {
    match err {
        GpuError::Unavailable => 0,
        GpuError::TooLarge => 1,
        GpuError::Readback => 2,
        GpuError::UnsupportedMode(_) => 3,
        GpuError::UnsupportedAdjustment => 4,
        GpuError::UnsupportedSmartObject => 5,
        GpuError::UnsupportedText => 6,
        GpuError::UnsupportedLayerEffect => 7,
        GpuError::UnsupportedAdvancedBlending => 8,
        GpuError::UnsupportedDepth => 9,
        GpuError::UnsupportedSmartFilter => 10,
    }
}

static DECLINES: [AtomicU64; DECLINE_REASONS.len()] = [const { AtomicU64::new(0) }; 11];
static LAYER_DECLINES: [AtomicU64; DECLINE_REASONS.len()] = [const { AtomicU64::new(0) }; 11];
static RESIDENT_HITS: AtomicU64 = AtomicU64::new(0);
static RESIDENT_MISSES: AtomicU64 = AtomicU64::new(0);
static RESIDENT_ENTRIES: AtomicU64 = AtomicU64::new(0);
static RESIDENT_BYTES: AtomicU64 = AtomicU64::new(0);
static SOURCE_HITS: AtomicU64 = AtomicU64::new(0);
static SOURCE_MISSES: AtomicU64 = AtomicU64::new(0);
static SOURCE_BYTES: AtomicU64 = AtomicU64::new(0);
static EFFECT_DECODES: AtomicU64 = AtomicU64::new(0);
static LARGEST_CANVAS: AtomicU64 = AtomicU64::new(0);
/// Device time of the last GPU composite in nanoseconds; `u64::MAX` is "none".
static DEVICE_NS: AtomicU64 = AtomicU64::new(u64::MAX);

/// Count one GPU decline under its reason.
pub(crate) fn count_decline(err: &GpuError) {
    DECLINES[reason_index(err)].fetch_add(1, Relaxed);
}

/// Count one declining layer the plan walk sees. A corpus run ranks reasons
/// without stopping at the first declining layer.
pub(crate) fn count_layer_decline(err: &GpuError) {
    LAYER_DECLINES[reason_index(err)].fetch_add(1, Relaxed);
}

pub(crate) fn count_resident(hit: bool) {
    if hit {
        RESIDENT_HITS.fetch_add(1, Relaxed);
    } else {
        RESIDENT_MISSES.fetch_add(1, Relaxed);
    }
}

/// Publish the resident store's post-eviction size.
pub(crate) fn set_resident(entries: usize, bytes: u64) {
    RESIDENT_ENTRIES.store(entries as u64, Relaxed);
    RESIDENT_BYTES.store(bytes, Relaxed);
}

pub(crate) fn count_source_hit() {
    SOURCE_HITS.fetch_add(1, Relaxed);
}

pub(crate) fn count_source_miss() {
    SOURCE_MISSES.fetch_add(1, Relaxed);
}

pub(crate) fn set_source_bytes(bytes: usize) {
    SOURCE_BYTES.store(bytes as u64, Relaxed);
}

pub(crate) fn count_effect_decode() {
    EFFECT_DECODES.fetch_add(1, Relaxed);
}

/// Note a composite canvas of `bytes`; the largest since the last
/// [`take_largest_canvas`] is kept.
pub(crate) fn note_canvas(bytes: usize) {
    LARGEST_CANVAS.fetch_max(bytes as u64, Relaxed);
}

/// The largest composite canvas noted since the previous call, then reset.
pub fn take_largest_canvas() -> u64 {
    LARGEST_CANVAS.swap(0, Relaxed)
}

/// Publish the device-side time of the GPU composite that just finished.
pub(crate) fn publish_device_ms(ms: f64) {
    DEVICE_NS.store((ms * 1.0e6).max(0.0) as u64, Relaxed);
}

/// The device time published since the previous call, then cleared. `None`
/// when no GPU composite reported one.
pub fn take_device_ms() -> Option<f64> {
    match DEVICE_NS.swap(u64::MAX, Relaxed) {
        u64::MAX => None,
        ns => Some(ns as f64 / 1.0e6),
    }
}

/// A snapshot of the counters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub resident_hits: u64,
    pub resident_misses: u64,
    pub resident_entries: u64,
    pub resident_bytes: u64,
    pub source_hits: u64,
    pub source_misses: u64,
    pub source_bytes: u64,
    pub effect_decodes: u64,
    /// One `(reason, count)` per [`DECLINE_REASONS`] entry, in that order.
    pub declines: Vec<(&'static str, u64)>,
    /// One `(reason, count)` per [`DECLINE_REASONS`] entry, counted per
    /// declining layer rather than per composite.
    pub layer_declines: Vec<(&'static str, u64)>,
}

pub fn counters() -> Counters {
    Counters {
        resident_hits: RESIDENT_HITS.load(Relaxed),
        resident_misses: RESIDENT_MISSES.load(Relaxed),
        resident_entries: RESIDENT_ENTRIES.load(Relaxed),
        resident_bytes: RESIDENT_BYTES.load(Relaxed),
        source_hits: SOURCE_HITS.load(Relaxed),
        source_misses: SOURCE_MISSES.load(Relaxed),
        source_bytes: SOURCE_BYTES.load(Relaxed),
        effect_decodes: EFFECT_DECODES.load(Relaxed),
        declines: DECLINE_REASONS
            .iter()
            .zip(&DECLINES)
            .map(|(reason, n)| (*reason, n.load(Relaxed)))
            .collect(),
        layer_declines: DECLINE_REASONS
            .iter()
            .zip(&LAYER_DECLINES)
            .map(|(reason, n)| (*reason, n.load(Relaxed)))
            .collect(),
    }
}

/// The current count for one decline reason.
pub fn decline_count(reason: &str) -> u64 {
    DECLINE_REASONS
        .iter()
        .position(|r| *r == reason)
        .map_or(0, |i| DECLINES[i].load(Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, BlendMode, Channel, ColorMode, Document, Layer, PsdRect};

    fn solid_doc(w: u32, h: u32, blend: BlendMode) -> Document {
        let n = (w * h) as usize;
        let layer = Layer {
            name: "solid".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h as i32,
                right: w as i32,
            },
            blend,
            opacity: 255,
            fill: 255,
            visible: true,
            channels: [0i16, 1, 2, -1]
                .map(|id| Channel {
                    id,
                    data: vec![200u8; n].into(),
                })
                .to_vec(),
            ..Default::default()
        };
        let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        doc
    }

    #[test]
    fn largest_canvas_covers_full_document() {
        let (w, h) = (37u32, 23u32);
        let _ = crate::composite_rgba(&solid_doc(w, h, BlendMode::Normal));
        // Other tests composite concurrently; the max is at least this canvas.
        assert!(take_largest_canvas() >= u64::from(w * h * 4));
    }

    #[test]
    fn dissolve_decline_is_counted() {
        let doc = solid_doc(4, 4, BlendMode::Dissolve);
        let before = decline_count("unsupported_mode");
        assert!(matches!(
            crate::composite_gpu(&doc),
            Err(GpuError::UnsupportedMode(BlendMode::Dissolve))
        ));
        assert!(decline_count("unsupported_mode") > before);
    }

    #[test]
    fn layer_decline_is_counted() {
        let doc = solid_doc(4, 4, BlendMode::Dissolve);
        let before = counters()
            .layer_declines
            .iter()
            .find(|(reason, _)| *reason == "unsupported_mode")
            .map_or(0, |(_, n)| *n);
        assert!(matches!(
            crate::composite_gpu(&doc),
            Err(GpuError::UnsupportedMode(BlendMode::Dissolve))
        ));
        let after = counters()
            .layer_declines
            .iter()
            .find(|(reason, _)| *reason == "unsupported_mode")
            .map_or(0, |(_, n)| *n);
        assert!(after > before);
    }

    #[test]
    fn type_source_decline_is_counted() {
        let layer = Layer {
            name: "text".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 2,
            },
            type_tool: Some(pictura_core::TypeTool {
                transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                text: "Hi".into(),
                bounds: [0, 0, 2, 2],
                text_desc: Vec::new().into(),
                warp_desc: Vec::new().into(),
                fonts: Vec::new(),
                style: None,
                vertical: false,
                box_bounds: None,
                path: None,
            }),
            ..Default::default()
        };
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        let before = decline_count("type_source");
        assert!(matches!(
            crate::composite_gpu(&doc),
            Err(GpuError::UnsupportedText)
        ));
        assert!(decline_count("type_source") > before);
        assert!(DECLINE_REASONS.contains(&"type_source"));
        assert!(!DECLINE_REASONS.contains(&"unsupported_text"));
    }

    #[test]
    fn device_time_is_taken_once() {
        publish_device_ms(1.5);
        let first = take_device_ms();
        assert!(first.is_some_and(|ms| (ms - 1.5).abs() < 1e-6));
        assert!(take_device_ms().is_none());
    }

    #[test]
    fn resident_gauges_are_visible() {
        set_resident(3, 1234);
        let counters = counters();
        assert_eq!(counters.resident_entries, 3);
        assert_eq!(counters.resident_bytes, 1234);
    }
}
