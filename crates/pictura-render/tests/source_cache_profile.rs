//! Ignored profile for the layer-source cache. Run with:
//! `cargo test -p pictura-render --test source_cache_profile -- --ignored --nocapture`

use std::time::Instant;

use pictura_core::{
    BitDepth, BlendMode, Channel, CharacterAttrs, ColorMode, Document, Layer, LayerStack, PsdRect,
    SmartObject, SmartObjectKind, TextStyle, TypeTool,
};

fn source_doc() -> Document {
    let mut d = Document::new(200, 200, ColorMode::Rgb, BitDepth::Eight);
    let n = 200 * 200;
    d.layers = LayerStack::from_layers(vec![Layer {
        name: "source".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 200,
            right: 200,
        },
        channels: vec![
            Channel {
                id: 0,
                data: vec![180u8; n].into(),
            },
            Channel {
                id: 1,
                data: vec![40u8; n].into(),
            },
            Channel {
                id: 2,
                data: vec![10u8; n].into(),
            },
            Channel {
                id: -1,
                data: vec![255u8; n].into(),
            },
        ],
        ..Default::default()
    }]);
    d.composite = pictura_render::composite_rgba(&d);
    d
}

fn smart_source(rect: PsdRect) -> Layer {
    let payload = pictura_codec::write_psd(&source_doc()).expect("source writes");
    let mut so = {
        let mut so = SmartObject::default();
        so.kind = SmartObjectKind::Embedded;
        so
    };
    so.set_payload(payload);
    Layer {
        name: "smart".into(),
        rect,
        smart_object: Some(so),
        ..Default::default()
    }
}

fn smart_filtered_layer() -> Layer {
    let mut layer = smart_source(PsdRect {
        top: 0,
        left: 0,
        bottom: 2000,
        right: 2000,
    });
    let slots = [
        1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 25.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0, 100.0,
    ];
    pictura_codec::attach_lens_correction_filter(&mut layer, &slots).expect("attaches");
    layer
}

fn text_layer() -> Layer {
    let style = TextStyle {
        font: Some("Arial".into()),
        character: CharacterAttrs {
            size: 48.0,
            fill_color: [0.9, 0.1, 0.1, 1.0],
            ..CharacterAttrs::default()
        },
        ..TextStyle::default()
    };
    let mut layer = Layer {
        name: "text".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 200,
            right: 400,
        },
        type_tool: Some(TypeTool {
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text: "Cache".into(),
            bounds: [0, 0, 400, 200],
            text_desc: Vec::new(),
            warp_desc: Vec::new(),
            fonts: vec!["Arial".into()],
            style: Some(style),
            vertical: false,
            box_bounds: None,
            path: None,
        }),
        ..Default::default()
    };
    pictura_render::set_layer_style_value(&mut layer, "dropShadow.on", 1.0);
    layer.blend = BlendMode::Normal;
    layer
}

fn profile_doc() -> Document {
    let mut d = Document::new(4000, 4000, ColorMode::Rgb, BitDepth::Eight);
    d.layers = LayerStack::from_layers(vec![smart_filtered_layer(), text_layer()]);
    d
}

#[test]
#[ignore = "profile: cargo test -p pictura-render --test source_cache_profile -- --ignored --nocapture"]
fn warm_region_is_ten_times_faster() {
    let d = profile_doc();
    const N: usize = 5;
    const REGION: (u32, u32, u32, u32) = (100, 100, 512, 120);

    let cold = {
        let start = Instant::now();
        for _ in 0..N {
            pictura_render::purge_source_cache();
            let _ =
                pictura_render::composite_rgba_region(&d, REGION.0, REGION.1, REGION.2, REGION.3);
        }
        start.elapsed()
    };
    // Warm the cache, then time repeated warm composites.
    let _ = pictura_render::composite_rgba_region(&d, REGION.0, REGION.1, REGION.2, REGION.3);
    let warm = {
        let start = Instant::now();
        for _ in 0..N {
            let _ =
                pictura_render::composite_rgba_region(&d, REGION.0, REGION.1, REGION.2, REGION.3);
        }
        start.elapsed()
    };
    let region_counters = pictura_render::metrics::counters();

    pictura_render::purge_source_cache();
    let full_cold = {
        let start = Instant::now();
        let _ = pictura_render::composite_rgba(&d);
        start.elapsed()
    };
    let full_warm = {
        let start = Instant::now();
        let _ = pictura_render::composite_rgba(&d);
        start.elapsed()
    };
    let full_counters = pictura_render::metrics::counters();

    assert!(
        warm * 10 <= cold,
        "cold {cold:?} warm {warm:?}: a warm 512x120 region must be 10x faster"
    );
    assert!(
        full_counters.source_hits > region_counters.source_hits,
        "a repeated full composite hits the cache"
    );
    eprintln!(
        "region {N}x: cold {cold:?} ({:.2} ms/op), warm {warm:?} ({:.2} ms/op), ratio {:.1}x",
        cold.as_secs_f64() * 1000.0 / N as f64,
        warm.as_secs_f64() * 1000.0 / N as f64,
        cold.as_secs_f64() / warm.as_secs_f64()
    );
    eprintln!(
        "full 4000x4000: cold {full_cold:?}, warm {full_warm:?}, ratio {:.1}x",
        full_cold.as_secs_f64() / full_warm.as_secs_f64()
    );
    eprintln!(
        "counters after region: hits={} misses={} bytes={} decodes={}",
        region_counters.source_hits,
        region_counters.source_misses,
        region_counters.source_bytes,
        region_counters.effect_decodes
    );
    eprintln!(
        "counters after full:   hits={} misses={} bytes={} decodes={}",
        full_counters.source_hits,
        full_counters.source_misses,
        full_counters.source_bytes,
        full_counters.effect_decodes
    );
}
