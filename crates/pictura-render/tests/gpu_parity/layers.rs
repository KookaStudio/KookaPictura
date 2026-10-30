//! Group, mask, fill, and adjustment-layer parity tests.

use pictura_adjust::Adjustment;
use pictura_core::{
    AdjustmentData, BitDepth, BlendMode, Channel, ColorMode, Document, Layer, LayerMask,
    LayerStack, PsdRect, SmartObject, SmartObjectKind, TextStyle, TypeTool, VectorFillRule,
    VectorMask, VectorSubpath,
};
use pictura_render::{
    composite_active, composite_gpu, composite_gpu_or_cpu, composite_region_active, composite_rgba,
    decode_layer_fill, encode_invert, encode_pattern_fill, metrics, set_layer_style_value, Backend,
    GpuError,
};

use crate::common::{
    adjustment_layer, base_layer, check_scene_parity, group, layer, require_gpu, SIZE,
};

/// The data path's mask plane and packed-group source are exercised here; the
/// blend/adjustment tests above never bind either.
#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn groups_and_masks_match_cpu() {
    require_gpu();
    let mut max_delta = 0i32;

    // Masked, reduced-opacity pixel layer over an opaque backdrop.
    let mut masked = layer("masked", BlendMode::Multiply, |x, y| {
        (x as u8 * 16 + 8, y as u8 * 16 + 8, 255 - x as u8 * 16, 200)
    });
    masked.opacity = 180;
    masked.mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some((0..SIZE * SIZE).map(|i| (i * 37 % 256) as u8).collect()),
        ..Default::default()
    });
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), masked]);
    check_scene_parity(&doc, &mut max_delta);

    // Isolated group (packed inner canvas source) over the backdrop.
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        base_layer(),
        group(
            "iso",
            BlendMode::Multiply,
            200,
            vec![layer("child", BlendMode::Normal, |x, y| {
                (255 - x as u8 * 20, 40 + y as u8 * 20, x as u8 * 32, 255)
            })],
        ),
    ]);
    check_scene_parity(&doc, &mut max_delta);

    // Adjustment layer gated by a mask.
    let mut adj = adjustment_layer("invert", encode_invert());
    adj.mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
        default_color: 255,
        disabled: false,
        flags: 0,
        data: Some(
            (0..SIZE * SIZE)
                .map(|i| (255 - i * 11 % 256) as u8)
                .collect(),
        ),
        ..Default::default()
    });
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), adj]);
    check_scene_parity(&doc, &mut max_delta);

    // Pixel layer clipped by a document-relative vector mask (per-pixel plane).
    let mut shaped = layer("shaped", BlendMode::Normal, |x, y| {
        (x as u8 * 20, y as u8 * 20, 128, 255)
    });
    let edge = 4 * 256;
    shaped.vector_mask = Some(VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: vec![[0, 0], [edge, 0], [edge, edge], [0, edge]],
        }],
        invert: false,
        disabled: false,
        ..Default::default()
    });
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), shaped]);
    check_scene_parity(&doc, &mut max_delta);

    eprintln!("group/mask parity: max delta {max_delta} LSB");
}

/// A reduced-fill layer must match the CPU oracle within ±1 LSB; over an opaque
/// backdrop the fill factor scales colour only, so the output alpha stays 255.
#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn fill_scene_matches_cpu() {
    require_gpu();
    let mut top = layer("fill", BlendMode::Multiply, |x, y| {
        (x as u8 * 16 + 8, y as u8 * 16 + 8, 255 - x as u8 * 16, 200)
    });
    top.opacity = 180;
    top.fill = 96;
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), top]);

    let mut max_delta = 0i32;
    check_scene_parity(&doc, &mut max_delta);

    let cpu = composite_rgba(&doc);
    let plane = cpu.pixel_count();
    for i in 0..plane {
        assert_eq!(
            cpu.data[3 * plane + i],
            255,
            "fill must not change the alpha over an opaque backdrop at {i}"
        );
    }
    eprintln!("fill parity: max delta {max_delta} LSB, alpha unchanged");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn adjustment_layers_match_cpu() {
    require_gpu();
    use pictura_render::{
        encode_brightness_contrast, encode_hue_saturation, encode_invert, encode_posterize,
        encode_threshold,
    };
    let cases: &[(&str, AdjustmentData)] = &[
        ("invert", encode_invert()),
        ("posterize", encode_posterize(4)),
        ("threshold", encode_threshold(128)),
        ("brightness_contrast", encode_brightness_contrast(20, 0)),
        ("hue_saturation", encode_hue_saturation(30, 0, 0)),
    ];
    let mut checked = 0;
    let mut max_delta = 0i32;
    for (name, data) in cases {
        let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![
            base_layer(),
            adjustment_layer(name, data.clone()),
        ]);
        let cpu = composite_rgba(&doc);
        let gpu = match composite_gpu(&doc) {
            Ok(buf) => buf,
            Err(e) => panic!("{name}: {e}"),
        };
        assert_eq!((gpu.width, gpu.height, gpu.channels), (SIZE, SIZE, 4));
        let mut case_delta = 0i32;
        for (i, (&a, &b)) in cpu.data.iter().zip(&gpu.data).enumerate() {
            let delta = (a as i32 - b as i32).abs();
            case_delta = case_delta.max(delta);
            max_delta = max_delta.max(delta);
            assert!(delta <= 1, "{name} byte {i}: cpu {a} vs gpu {b}");
        }
        eprintln!("adjustment {name}: max delta {case_delta} LSB");
        checked += 1;
    }
    eprintln!("adjustment parity: {checked} kinds ok, max delta {max_delta} LSB");
}

#[test]
fn unsupported_adjustment_falls_back_or_errors() {
    // A key the renderer does not decode, and one that decodes but the GPU has
    // no kind for (Selective Color): both must error and fall back without
    // panicking. Levels is now accepted through the lookup-table route.
    let cases = [
        AdjustmentData {
            key: *b"clrL",
            data: vec![1, 2, 3].into(),
        },
        AdjustmentData {
            key: *b"selc",
            data: vec![1, 2, 3].into(),
        },
    ];
    for data in cases {
        let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![
            base_layer(),
            adjustment_layer("unsupported", data),
        ]);
        assert!(
            matches!(composite_gpu(&doc), Err(GpuError::UnsupportedAdjustment)),
            "expected UnsupportedAdjustment"
        );
        assert_eq!(
            composite_gpu_or_cpu(&doc).data,
            composite_rgba(&doc).data,
            "fallback must equal the CPU oracle"
        );
    }
}

// --- New routes ------------------------------------------------------------

fn alpha_ramp(x: u32, y: u32) -> u8 {
    let i = y * SIZE + x;
    (i * 255 / (SIZE * SIZE - 1)) as u8
}

fn type_style() -> TextStyle {
    TextStyle {
        font: Some("Arial".into()),
        character: pictura_core::CharacterAttrs {
            size: 4.0,
            fill_color: [1.0, 0.0, 0.0, 0.0],
            ..Default::default()
        },
        ..Default::default()
    }
}

fn type_tool() -> TypeTool {
    TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "Hi".into(),
        bounds: [0, 0, SIZE as i32, SIZE as i32],
        text_desc: Vec::new().into(),
        warp_desc: Vec::new().into(),
        fonts: Vec::new(),
        style: Some(type_style()),
        vertical: false,
        box_bounds: None,
        path: None,
    }
}

fn whole_rect() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: SIZE as i32,
        right: SIZE as i32,
    }
}

fn embedded_object() -> SmartObject {
    let mut source = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    source.layers = LayerStack::from_layers(vec![layer("source", BlendMode::Normal, |x, y| {
        (x as u8 * 20, y as u8 * 20, 80, 255)
    })]);
    source.composite = composite_rgba(&source);
    let payload = pictura_codec::write_psd(&source).expect("payload writes");
    let mut so = SmartObject::default();
    so.kind = SmartObjectKind::Embedded;
    so.set_payload(payload);
    so
}

fn lens_slots() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 25.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0, 100.0,
    ]
}

fn proxy_less_type() -> Layer {
    let mut text = layer("text", BlendMode::Normal, |x, y| {
        (0, 0, 0, alpha_ramp(x, y))
    });
    text.channels = Vec::new();
    text.type_tool = Some(type_tool());
    text
}

fn sub_layer(name: &str, rect: PsdRect, color: (u8, u8, u8, u8), blend: BlendMode) -> Layer {
    let n = (rect.width() * rect.height()) as usize;
    let (r, g, b, a) = color;
    Layer {
        name: name.into(),
        rect,
        blend,
        opacity: 255,
        fill: 255,
        visible: true,
        channels: [0i16, 1, 2, -1]
            .map(|id| Channel {
                id,
                data: vec![
                    match id {
                        0 => r,
                        1 => g,
                        2 => b,
                        _ => a,
                    };
                    n
                ]
                .into(),
            })
            .to_vec(),
        ..Default::default()
    }
}

fn doc_with(layers: Vec<Layer>) -> Document {
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(layers);
    doc
}

fn fixture(bytes: &[u8]) -> Document {
    pictura_codec::read_psd(bytes).expect("fixture parses")
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn raster_sources_match_cpu() {
    require_gpu();
    let mut max_delta = 0i32;

    let doc = doc_with(vec![base_layer(), proxy_less_type()]);
    check_scene_parity(&doc, &mut max_delta);

    let mut smart = layer("smart", BlendMode::Normal, |_, _| (0, 0, 0, 0));
    smart.channels = Vec::new();
    smart.smart_object = Some(embedded_object());
    let doc = doc_with(vec![base_layer(), smart]);
    check_scene_parity(&doc, &mut max_delta);

    let mut filtered = layer("filtered", BlendMode::Normal, |_, _| (0, 0, 0, 0));
    filtered.channels = Vec::new();
    filtered.smart_object = Some(embedded_object());
    pictura_codec::attach_lens_correction_filter(&mut filtered, &lens_slots())
        .expect("lens filter");
    let so = filtered.smart_object.as_mut().expect("smart object");
    so.filter_mask_enabled = true;
    so.filter_mask_extend_with_white = true;
    so.filter_mask = Some(pictura_core::FilterMask {
        rect: whole_rect(),
        data: (0..(SIZE * SIZE))
            .map(|i| (i * 17) as u8)
            .collect::<Vec<u8>>()
            .into(),
    });
    let doc = doc_with(vec![base_layer(), filtered]);
    check_scene_parity(&doc, &mut max_delta);

    eprintln!("raster parity: max delta {max_delta} LSB");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn fill_layers_match_cpu() {
    require_gpu();
    let mut max_delta = 0i32;
    for bytes in [
        include_bytes!("../../../pictura-codec/tests/fixtures/solid_fill.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/gradient_fill.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/pattern_fill.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/vector_fill.psd").as_slice(),
    ] {
        let doc = fixture(bytes);
        check_scene_parity(&doc, &mut max_delta);
    }
    eprintln!("fill parity: max delta {max_delta} LSB");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn adjustment_luts_match_cpu() {
    require_gpu();
    let mut max_delta = 0i32;

    let mut levels = vec![0, 2];
    for v in [5u16, 250, 10, 240, 120] {
        levels.extend_from_slice(&v.to_be_bytes());
    }
    let levels = adjustment_layer(
        "levels",
        AdjustmentData {
            key: *b"levl",
            data: levels.into(),
        },
    );
    let doc = doc_with(vec![base_layer(), levels]);
    check_scene_parity(&doc, &mut max_delta);

    let mut exposure = vec![0, 1];
    exposure.extend_from_slice(&0.5f32.to_be_bytes());
    exposure.extend_from_slice(&0.02f32.to_be_bytes());
    exposure.extend_from_slice(&1.1f32.to_be_bytes());
    let exposure = adjustment_layer(
        "exposure",
        AdjustmentData {
            key: *b"expA",
            data: exposure.into(),
        },
    );
    let doc = doc_with(vec![base_layer(), exposure]);
    check_scene_parity(&doc, &mut max_delta);

    for bytes in [
        include_bytes!("../../../pictura-codec/tests/fixtures/curves.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/gradient_map.psd").as_slice(),
    ] {
        let doc = fixture(bytes);
        check_scene_parity(&doc, &mut max_delta);
    }

    let mut masked = adjustment_layer(
        "masked levels",
        AdjustmentData {
            key: *b"levl",
            data: levels_bytes().into(),
        },
    );
    masked.opacity = 180;
    masked.mask = Some(LayerMask {
        rect: whole_rect(),
        default_color: 255,
        data: Some(
            (0..(SIZE * SIZE))
                .map(|i| (i * 11 % 256) as u8)
                .collect::<Vec<u8>>()
                .into(),
        ),
        ..Default::default()
    });
    let doc = doc_with(vec![base_layer(), masked]);
    check_scene_parity(&doc, &mut max_delta);

    eprintln!("adjustment LUT parity: max delta {max_delta} LSB");
}

fn levels_bytes() -> Vec<u8> {
    let mut levels = vec![0, 2];
    for v in [5u16, 250, 10, 240, 120] {
        levels.extend_from_slice(&v.to_be_bytes());
    }
    levels
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn effects_match_cpu() {
    require_gpu();
    let mut max_delta = 0i32;
    for bytes in [
        include_bytes!("../../../pictura-codec/tests/fixtures/drop_shadow.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/outer_glow.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/inner_shadow.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/inner_glow.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/bevel.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/satin.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/stroke.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/color_overlay.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/gradient_overlay.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/pattern_overlay.psd").as_slice(),
        include_bytes!("../../../pictura-codec/tests/fixtures/legacy_effects.psd").as_slice(),
    ] {
        let doc = fixture(bytes);
        check_scene_parity(&doc, &mut max_delta);
    }
    eprintln!("effect parity: max delta {max_delta} LSB");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn effect_draw_order_matches_cpu() {
    require_gpu();
    let mut styled = layer("styled", BlendMode::Normal, |x, y| {
        (x as u8 * 20, y as u8 * 20, 128, 255)
    });
    set_layer_style_value(&mut styled, "dropShadow.on", 1.0);
    set_layer_style_value(&mut styled, "stroke.on", 1.0);
    set_layer_style_value(&mut styled, "innerShadow.on", 1.0);
    let doc = doc_with(vec![base_layer(), styled]);
    let mut max_delta = 0i32;
    check_scene_parity(&doc, &mut max_delta);
    eprintln!("effect order parity: max delta {max_delta} LSB");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn disabled_effects_stay_eligible() {
    require_gpu();
    let mut styled = layer("styled", BlendMode::Normal, |x, y| {
        (x as u8 * 20, y as u8 * 20, 128, 255)
    });
    set_layer_style_value(&mut styled, "dropShadow.on", 0.0);
    let doc = doc_with(vec![base_layer(), styled]);
    let mut max_delta = 0i32;
    check_scene_parity(&doc, &mut max_delta);
    assert!(composite_gpu(&doc).is_ok());
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn clipping_matches_cpu() {
    require_gpu();
    let base = sub_layer(
        "base",
        PsdRect {
            top: 2,
            left: 2,
            bottom: 6,
            right: 6,
        },
        (255, 0, 0, 255),
        BlendMode::Normal,
    );
    let mut clipped = sub_layer("clipped", whole_rect(), (0, 255, 0, 255), BlendMode::Normal);
    clipped.clipping = true;
    let doc = doc_with(vec![base, clipped]);
    let mut max_delta = 0i32;
    check_scene_parity(&doc, &mut max_delta);
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn clipping_hidden_base_matches_cpu() {
    require_gpu();
    let mut base = sub_layer(
        "base",
        PsdRect {
            top: 2,
            left: 2,
            bottom: 6,
            right: 6,
        },
        (255, 0, 0, 255),
        BlendMode::Normal,
    );
    base.visible = false;
    let mut clipped = sub_layer("clipped", whole_rect(), (0, 255, 0, 255), BlendMode::Normal);
    clipped.clipping = true;
    let doc = doc_with(vec![base, clipped]);
    let mut max_delta = 0i32;
    check_scene_parity(&doc, &mut max_delta);
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn clipping_no_base_matches_cpu() {
    require_gpu();
    let mut clipped = sub_layer("clipped", whole_rect(), (0, 255, 0, 255), BlendMode::Normal);
    clipped.clipping = true;
    let doc = doc_with(vec![clipped]);
    let mut max_delta = 0i32;
    check_scene_parity(&doc, &mut max_delta);
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn clipping_multiple_layers_match_cpu() {
    require_gpu();
    let base = sub_layer(
        "base",
        PsdRect {
            top: 2,
            left: 2,
            bottom: 6,
            right: 6,
        },
        (255, 0, 0, 255),
        BlendMode::Normal,
    );
    let mut first = sub_layer("first", whole_rect(), (0, 255, 0, 200), BlendMode::Normal);
    first.clipping = true;
    let mut second = sub_layer(
        "second",
        whole_rect(),
        (0, 0, 255, 128),
        BlendMode::Multiply,
    );
    second.clipping = true;
    let doc = doc_with(vec![base, first, second]);
    let mut max_delta = 0i32;
    check_scene_parity(&doc, &mut max_delta);
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn clipping_base_fill_and_effects_match_cpu() {
    require_gpu();
    let mut base = sub_layer(
        "base",
        PsdRect {
            top: 2,
            left: 2,
            bottom: 6,
            right: 6,
        },
        (255, 0, 0, 255),
        BlendMode::Normal,
    );
    base.fill = 0;
    set_layer_style_value(&mut base, "dropShadow.on", 1.0);
    let mut clipped = sub_layer("clipped", whole_rect(), (0, 255, 0, 255), BlendMode::Normal);
    clipped.clipping = true;
    let doc = doc_with(vec![base, clipped]);
    let mut max_delta = 0i32;
    check_scene_parity(&doc, &mut max_delta);
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn cached_raster_source_is_reused() {
    require_gpu();
    let doc = doc_with(vec![proxy_less_type()]);
    let _ = composite_gpu(&doc).expect("first composite");
    let before = metrics::counters().resident_hits;
    let _ = composite_gpu(&doc).expect("second composite");
    assert!(
        metrics::counters().resident_hits > before,
        "the cached type raster is re-uploaded"
    );
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn cached_raster_source_invalidates() {
    require_gpu();
    let mut doc = doc_with(vec![base_layer(), proxy_less_type()]);
    let first = composite_gpu(&doc).expect("first composite");
    doc.layers[1].type_tool.as_mut().expect("type").text = "Changed".into();
    let before = metrics::counters().resident_misses;
    let second = composite_gpu(&doc).expect("second composite");
    assert_ne!(first.data.as_ref(), second.data.as_ref());
    assert!(
        metrics::counters().resident_misses > before,
        "the changed raster is re-uploaded"
    );
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn pattern_fill_is_not_served_stale() {
    require_gpu();
    let mut doc = fixture(include_bytes!(
        "../../../pictura-codec/tests/fixtures/pattern_fill.psd"
    ));
    let first = composite_gpu(&doc).expect("first composite");
    let layer = doc
        .layers
        .iter_mut()
        .find(|layer| matches!(decode_layer_fill(layer), Some(Adjustment::PatternFill(_))))
        .expect("a pattern fill layer");
    let Some(Adjustment::PatternFill(mut params)) = decode_layer_fill(layer) else {
        unreachable!()
    };
    params.pattern_id = "pictura-changed".into();
    layer.adjustment = Some(encode_pattern_fill(&params));
    let rgba = [255u8; 4 * 4 * 4];
    let record = pictura_codec::encode_rgb_pattern("changed", "pictura-changed", 4, 4, &rgba)
        .expect("pattern encodes");
    assert!(
        pictura_codec::add_document_pattern(&mut doc, "pictura-changed", &record),
        "the changed pattern is added"
    );
    let second = composite_gpu(&doc).expect("second composite");
    assert_ne!(
        first.data.as_ref(),
        second.data.as_ref(),
        "a changed pattern must not be served from the old upload"
    );
}

fn assert_region_matches_full(doc: &Document, label: &str) {
    let (full, backend) = composite_active(doc, true);
    assert_eq!(backend, Backend::Gpu, "{label}: full backend");
    let w = doc.width;
    let h = doc.height;
    let (left, top, right, bottom): (i32, i32, i32, i32) = if w > 2 && h > 2 {
        (1, 1, (w - 1) as i32, (h - 1) as i32)
    } else {
        (0, 0, w as i32, h as i32)
    };
    let rect = PsdRect {
        left,
        top,
        right,
        bottom,
    };
    let (region, backend) = composite_region_active(doc, rect, true);
    assert_eq!(backend, Backend::Gpu, "{label}: region backend");
    let rw = rect.width() as usize;
    let rh = rect.height() as usize;
    let plane = rw * rh;
    let full_plane = w as usize * h as usize;
    for y in 0..rh {
        for x in 0..rw {
            let ri = y * rw + x;
            let fi = (rect.top as usize + y) * w as usize + rect.left as usize + x;
            for c in 0..4 {
                assert_eq!(
                    region.data[c * plane + ri],
                    full.data[c * full_plane + fi],
                    "{label}: channel {c} at {x},{y}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn new_routes_region_matches_full_slice() {
    require_gpu();
    assert_region_matches_full(
        &doc_with(vec![base_layer(), proxy_less_type()]),
        "cached raster",
    );
    assert_region_matches_full(
        &fixture(include_bytes!(
            "../../../pictura-codec/tests/fixtures/solid_fill.psd"
        )),
        "fill",
    );
    assert_region_matches_full(
        &fixture(include_bytes!(
            "../../../pictura-codec/tests/fixtures/drop_shadow.psd"
        )),
        "effect",
    );
    let base = sub_layer(
        "base",
        PsdRect {
            top: 2,
            left: 2,
            bottom: 6,
            right: 6,
        },
        (255, 0, 0, 255),
        BlendMode::Normal,
    );
    let mut clipped = sub_layer("clipped", whole_rect(), (0, 255, 0, 255), BlendMode::Normal);
    clipped.clipping = true;
    assert_region_matches_full(&doc_with(vec![base, clipped]), "clipping");
}
