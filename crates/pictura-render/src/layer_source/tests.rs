use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use pictura_core::{
    BitDepth, ColorMode, Document, Layer, LayerMask, PixelBuffer, SmartObject, SmartObjectKind,
};

use super::*;
use crate::tests::composite::{type_layer, type_style};
use crate::tests::{doc, rect, solid};

static LOCK: Mutex<()> = Mutex::new(());

fn locked() -> std::sync::MutexGuard<'static, ()> {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    purge_source_cache();
    guard
}

struct BudgetGuard;
impl BudgetGuard {
    fn set(bytes: usize) -> Self {
        set_budget_for_test(bytes);
        Self
    }
}
impl Drop for BudgetGuard {
    fn drop(&mut self) {
        restore_budget_for_test();
        purge_source_cache();
    }
}

fn lens_slots() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 25.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0, 100.0,
    ]
}

fn source_doc() -> Document {
    let mut d = doc(
        4,
        4,
        vec![solid(
            "source",
            rect(0, 0, 4, 4),
            (200, 50, 0),
            255,
            pictura_core::BlendMode::Normal,
            255,
        )],
    );
    d.composite = crate::composite_rgba(&d);
    d
}

fn embedded() -> SmartObject {
    let payload = pictura_codec::write_psd(&source_doc()).expect("source writes");
    let mut so = SmartObject::default();
    so.kind = SmartObjectKind::Embedded;
    so.set_payload(payload);
    so
}

fn smart_layer(rect: pictura_core::PsdRect) -> Layer {
    Layer {
        name: "smart".into(),
        rect,
        smart_object: Some(embedded()),
        ..Default::default()
    }
}

fn filtered_layer(rect: pictura_core::PsdRect) -> Layer {
    let mut layer = smart_layer(rect);
    pictura_codec::attach_lens_correction_filter(&mut layer, &lens_slots()).expect("attaches");
    layer
}

fn styled_layer() -> Layer {
    let mut layer = solid(
        "styled",
        rect(10, 10, 30, 30),
        (255, 0, 0),
        255,
        pictura_core::BlendMode::Normal,
        255,
    );
    crate::set_layer_style_value(&mut layer, "dropShadow.on", 1.0);
    layer
}

#[test]
fn second_request_hits() {
    let _guard = locked();
    let key = Key::new(Kind::Text, 0xfeed);
    let _ = lookup(key, || {
        Artifact::Buffer(Arc::new(PixelBuffer::new(1, 1, 4)))
    });
    let hits = crate::metrics::counters().source_hits;
    let _ = lookup(key, || panic!("a warm key must not rebuild"));
    assert!(crate::metrics::counters().source_hits > hits);
}

#[test]
fn eviction_respects_the_byte_budget() {
    let _guard = locked();
    let _budget = BudgetGuard::set(16);
    let make = || PixelBuffer {
        width: 1,
        height: 3,
        channels: 4,
        data: vec![7u8; 12].into(),
    };
    let (first, second) = (Key::new(Kind::Text, 1), Key::new(Kind::Text, 2));
    let _ = lookup(first, || Artifact::Buffer(Arc::new(make())));
    let _ = lookup(second, || Artifact::Buffer(Arc::new(make())));
    assert!(cache_len() <= 1, "the budget evicts the first entry");
    assert!(!cache_has_key(first), "the oldest entry is evicted");
    assert!(cache_has_key(second), "the newest entry is kept");
    let rebuilt = lookup(first, || Artifact::Buffer(Arc::new(make())));
    assert!(
        matches!(rebuilt, Artifact::Buffer(_)),
        "the evicted key rebuilds"
    );
    assert!(
        crate::metrics::counters().source_bytes <= 16,
        "resident bytes stay under the budget"
    );
}

#[test]
fn purge_forces_a_rebuild() {
    let _guard = locked();
    let key = Key::new(Kind::Text, 3);
    let builds = Arc::new(AtomicUsize::new(0));
    let make = || PixelBuffer {
        width: 1,
        height: 3,
        channels: 4,
        data: vec![9u8; 12].into(),
    };
    let first = Arc::clone(&builds);
    let before = match lookup(key, || {
        first.fetch_add(1, Ordering::SeqCst);
        Artifact::Buffer(Arc::new(make()))
    }) {
        Artifact::Buffer(buffer) => buffer,
        _ => panic!("buffer expected"),
    };
    purge_source_cache();
    let second = Arc::clone(&builds);
    let after = match lookup(key, || {
        second.fetch_add(1, Ordering::SeqCst);
        Artifact::Buffer(Arc::new(make()))
    }) {
        Artifact::Buffer(buffer) => buffer,
        _ => panic!("buffer expected"),
    };
    assert_eq!(builds.load(Ordering::SeqCst), 2);
    assert_eq!(before.data.as_ref(), after.data.as_ref(), "pixels survive");
}

#[test]
fn parallel_build_of_one_key_runs_once() {
    let _guard = locked();
    let key = Key::new(Kind::Text, 4);
    let builds = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                let _ = lookup(key, || {
                    builds.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    Artifact::Buffer(Arc::new(PixelBuffer::new(1, 1, 4)))
                });
            });
        }
    });
    assert_eq!(builds.load(Ordering::SeqCst), 1);
}

#[test]
fn counters_are_visible() {
    let _guard = locked();
    let _ = lookup(Key::new(Kind::Text, 5), || {
        Artifact::Buffer(Arc::new(PixelBuffer::new(1, 1, 4)))
    });
    let counters = crate::metrics::counters();
    assert!(counters.source_hits + counters.source_misses > 0);
    assert!(counters.source_bytes >= 4);
}

#[test]
fn embedded_source_reuses_a_hit() {
    let _guard = locked();
    let so = embedded();
    let first = embedded_source(&so).expect("source decodes");
    let second = embedded_source(&so).expect("source decodes");
    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn hidden_layer_is_not_decoded() {
    let _guard = locked();
    let mut smart = smart_layer(rect(0, 0, 4, 4));
    smart.visible = false;
    let mut text = type_layer(Some(type_style()));
    text.visible = false;
    let d = doc(20, 20, vec![smart, text]);
    let _ = crate::composite_rgba(&d);
    assert_eq!(cache_len(), 0, "a hidden object layer is not decoded");
}

#[test]
fn text_raster_hits_and_invalidates() {
    let _guard = locked();
    let mut layer = type_layer(Some(type_style()));
    let first = text_raster(&layer).expect("text rasterizes");
    let second = text_raster(&layer).expect("text rasterizes");
    assert!(Arc::ptr_eq(&first, &second), "an unchanged type layer hits");

    layer.type_tool.as_mut().unwrap().text = "Hello".into();
    let edited = text_raster(&layer).expect("text rasterizes");
    assert!(!Arc::ptr_eq(&first, &edited), "editing text invalidates");

    layer
        .type_tool
        .as_mut()
        .unwrap()
        .style
        .as_mut()
        .unwrap()
        .character
        .size = 64.0;
    let restyled = text_raster(&layer).expect("text rasterizes");
    assert!(
        !Arc::ptr_eq(&edited, &restyled),
        "editing style invalidates"
    );

    let plain_key = key::text_key(layer.type_tool.as_ref().unwrap(), layer.rect);
    layer.type_tool.as_mut().unwrap().vertical = true;
    assert_ne!(
        plain_key,
        key::text_key(layer.type_tool.as_ref().unwrap(), layer.rect),
        "vertical text invalidates"
    );
    layer.type_tool.as_mut().unwrap().vertical = false;
    layer.type_tool.as_mut().unwrap().box_bounds = Some([0.0, 0.0, 40.0, 120.0]);
    assert_ne!(
        plain_key,
        key::text_key(layer.type_tool.as_ref().unwrap(), layer.rect),
        "a paragraph box invalidates"
    );
    layer.type_tool.as_mut().unwrap().box_bounds = None;

    let before_font = text_raster(&layer).expect("text rasterizes");
    assert!(crate::fonts::register_font(
        "CacheTest",
        "Regular",
        include_bytes!("../../../../assets/fonts/NotoSans-Regular.ttf").to_vec()
    ));
    let after_font = text_raster(&layer).expect("text rasterizes");
    assert!(
        !Arc::ptr_eq(&before_font, &after_font),
        "registering a font invalidates"
    );

    crate::fonts::set_missing_glyph_protection(false);
    let flagged = text_raster(&layer).expect("text rasterizes");
    crate::fonts::set_missing_glyph_protection(true);
    assert!(
        !Arc::ptr_eq(&after_font, &flagged),
        "a global flag invalidates"
    );
}

#[test]
fn effects_decode_once() {
    let _guard = locked();
    let d = doc(40, 40, vec![styled_layer()]);
    let before = crate::metrics::counters().effect_decodes;
    let _ = crate::composite_rgba(&d);
    assert_eq!(crate::metrics::counters().effect_decodes, before + 1);
}

#[test]
fn effect_passes_invalidate_on_style_or_rect() {
    let _guard = locked();
    let mut layer = styled_layer();
    let d = doc(60, 60, vec![layer.clone()]);
    let first = crate::composite_rgba(&d).data;

    crate::set_layer_style_value(&mut layer, "dropShadow.size", 12.0);
    layer.rect = rect(8, 8, 32, 32);
    let d2 = doc(60, 60, vec![layer]);
    let warm = crate::composite_rgba(&d2).data;
    purge_source_cache();
    let cold = crate::composite_rgba(&d2).data;
    assert_eq!(warm.as_ref(), cold.as_ref());
    assert_ne!(
        first.as_ref(),
        warm.as_ref(),
        "the style change moves pixels"
    );
}

#[test]
fn cold_and_warm_composites_agree() {
    let _guard = locked();
    let d = doc(
        60,
        60,
        vec![
            styled_layer(),
            type_layer(Some(type_style())),
            filtered_layer(rect(0, 0, 20, 20)),
            smart_layer(rect(30, 30, 50, 50)),
        ],
    );
    let cold = crate::composite_rgba(&d);
    let warm = crate::composite_rgba(&d);
    assert_eq!(cold.data.as_ref(), warm.data.as_ref());
}

#[test]
fn every_source_kind_hits_on_the_second_composite() {
    let _guard = locked();
    let d = doc(
        60,
        60,
        vec![
            styled_layer(),
            type_layer(Some(type_style())),
            filtered_layer(rect(0, 0, 20, 20)),
            smart_layer(rect(30, 30, 50, 50)),
        ],
    );
    let _ = crate::composite_rgba(&d);
    let hits = crate::metrics::counters().source_hits;
    let _ = crate::composite_rgba(&d);
    assert!(
        crate::metrics::counters().source_hits >= hits + 4,
        "the smart object, smart filter, type raster, and effect passes all hit"
    );
    assert_eq!(cache_kind_count(Kind::Embedded), 1);
    assert_eq!(cache_kind_count(Kind::Filtered), 1);
    assert_eq!(cache_kind_count(Kind::Text), 1);
    assert_eq!(cache_kind_count(Kind::Effects), 1);
    assert_eq!(cache_kind_count(Kind::Passes), 1);
}

#[test]
fn cache_keeps_a_document_size_independent_key() {
    let _guard = locked();
    let mut d = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    d.layers = pictura_core::LayerStack::from_layers(vec![styled_layer()]);
    let before = key::content_key(&d.layers[0], &d);
    d.width = 16;
    let after = key::content_key(&d.layers[0], &d);
    assert_ne!(before, after, "the document size is part of the key");
}

#[test]
fn paint_bounds_bounds_a_styled_pixel_layer() {
    let _guard = locked();
    let mut layer = solid(
        "styled",
        rect(50, 50, 60, 60),
        (255, 0, 0),
        255,
        pictura_core::BlendMode::Normal,
        255,
    );
    crate::set_layer_style_value(&mut layer, "dropShadow.on", 1.0);
    crate::set_layer_style_value(&mut layer, "dropShadow.size", 6.0);
    let d = doc(120, 120, vec![layer]);
    let bounds = crate::layer_paint_bounds(&d, "0").expect("bounds");
    assert!(bounds.width() < 120, "{bounds:?}");
}

#[test]
fn missing_entries_are_capped() {
    let _guard = locked();
    for i in 0..(MAX_ENTRIES as u64 + 8) {
        let _ = lookup(Key::new(Kind::Text, 10_000 + i), || Artifact::Missing);
    }
    assert!(
        cache_len() <= MAX_ENTRIES,
        "zero-byte misses cannot grow the map forever"
    );
}

#[test]
fn filtered_source_invalidates_when_only_the_off_canvas_rect_changes() {
    let _guard = locked();
    let d = doc(8, 8, vec![]);
    let first_layer = filtered_layer(rect(-4, -4, 12, 12));
    let first = filtered_source(&first_layer, &d).expect("first source");
    let mut second_layer = first_layer.clone();
    second_layer.rect = rect(-5, -5, 13, 13);
    let second = filtered_source(&second_layer, &d).expect("second source");
    assert!(
        !Arc::ptr_eq(&first.buffer, &second.buffer),
        "an off-canvas rect change still invalidates the filtered source"
    );
}

#[test]
fn text_state_invalidates_sources_and_passes() {
    let _guard = locked();
    let _gamma = crate::test_support::gamma_guard();
    let old_gamma = crate::text_gamma();
    let so = embedded();
    let first = embedded_source(&so).expect("source decodes");
    let styled = styled_layer();
    let d = doc(60, 60, vec![styled.clone()]);
    let before = key::content_key(&styled, &d);
    crate::set_text_gamma(Some(1.8));
    let second = embedded_source(&so).expect("source decodes");
    let after = key::content_key(&styled, &d);
    crate::set_text_gamma(old_gamma);
    assert!(
        !Arc::ptr_eq(&first, &second),
        "a text-gamma change invalidates the embedded source"
    );
    assert_ne!(
        before, after,
        "a text-gamma change invalidates effect passes"
    );
}

#[test]
fn effect_passes_invalidate_on_master_switch_and_mask() {
    let _guard = locked();
    let mut layer = styled_layer();
    let mask = LayerMask {
        rect: rect(10, 10, 30, 30),
        default_color: 0,
        disabled: false,
        data: Some(vec![255u8; 400].into()),
        ..Default::default()
    };
    layer.mask = Some(mask);
    let d = doc(60, 60, vec![layer.clone()]);
    let warm = crate::composite_rgba(&d).data;
    purge_source_cache();
    let cold = crate::composite_rgba(&d).data;
    assert_eq!(warm.as_ref(), cold.as_ref());

    crate::set_layer_style_value(&mut layer, "fx.visible", 0.0);
    let d2 = doc(60, 60, vec![layer.clone()]);
    let switched = crate::composite_rgba(&d2).data;
    purge_source_cache();
    let switched_cold = crate::composite_rgba(&d2).data;
    assert_eq!(switched.as_ref(), switched_cold.as_ref());
    assert_ne!(
        warm.as_ref(),
        switched.as_ref(),
        "the master switch invalidates"
    );

    layer.mask.as_mut().expect("mask").data = Some(vec![0u8; 400].into());
    let d3 = doc(60, 60, vec![layer]);
    let masked = crate::composite_rgba(&d3).data;
    purge_source_cache();
    let masked_cold = crate::composite_rgba(&d3).data;
    assert_eq!(masked.as_ref(), masked_cold.as_ref());
    assert_ne!(switched.as_ref(), masked.as_ref(), "the mask invalidates");
}

#[test]
fn paint_bounds_unions_a_group_and_bounds_a_masked_adjustment() {
    let _guard = locked();
    let group = Layer {
        name: "group".into(),
        is_group: true,
        children: pictura_core::LayerStack::from_layers(vec![
            solid(
                "a",
                rect(10, 10, 20, 20),
                (255, 0, 0),
                255,
                pictura_core::BlendMode::Normal,
                255,
            ),
            solid(
                "b",
                rect(30, 30, 40, 40),
                (0, 255, 0),
                255,
                pictura_core::BlendMode::Normal,
                255,
            ),
        ]),
        ..Default::default()
    };
    let group_doc = doc(80, 80, vec![group]);
    let bounds = crate::layer_paint_bounds(&group_doc, "0").expect("group bounds");
    assert_eq!(
        (bounds.left, bounds.top, bounds.right, bounds.bottom),
        (10, 10, 40, 40)
    );

    let mask = LayerMask {
        rect: rect(12, 14, 30, 32),
        default_color: 0,
        disabled: false,
        data: Some(vec![255u8; 18 * 18].into()),
        ..Default::default()
    };
    let adjustment = Layer {
        name: "adjustment".into(),
        rect: rect(0, 0, 80, 80),
        adjustment: Some(crate::encode_brightness_contrast(0, 0)),
        mask: Some(mask.clone()),
        ..Default::default()
    };
    let bounded_doc = doc(80, 80, vec![adjustment.clone()]);
    assert_eq!(
        crate::layer_paint_bounds(&bounded_doc, "0"),
        Some(rect(12, 14, 30, 32))
    );

    let mut unbounded_mask = mask;
    unbounded_mask.default_color = 255;
    let unbounded = Layer {
        mask: Some(unbounded_mask),
        ..adjustment
    };
    let unbounded_doc = doc(80, 80, vec![unbounded]);
    assert_eq!(
        crate::layer_paint_bounds(&unbounded_doc, "0"),
        Some(rect(0, 0, 80, 80))
    );
    assert_eq!(crate::layer_paint_bounds(&bounded_doc, "missing"), None);
}
