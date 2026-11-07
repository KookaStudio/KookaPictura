use pictura_core::{
    memory, BitDepth, Channel, ColorMode, Document, Layer, LayerMask, LayerStack, PsdRect, Samples,
    SourceChannels,
};
use pictura_render::*;

fn rect(x: i32, y: i32, w: i32, h: i32) -> PsdRect {
    PsdRect {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    }
}

fn fixture() -> Document {
    let mut doc = Document::new(12, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![Layer {
        name: "Small".into(),
        rect: rect(2, 1, 3, 2),
        channels: [0, 1, 2, -1]
            .into_iter()
            .map(|id| Channel {
                id,
                data: vec![if id == -1 { 255 } else { 120 }; 6].into(),
            })
            .collect(),
        ..Default::default()
    }]);
    doc
}

fn imported(default: u8) -> Document {
    let mut doc = fixture();
    doc.layers[0].mask = Some(LayerMask {
        rect: rect(2, 1, 3, 2),
        default_color: default,
        flags: MASK_FLAG_UNLINKED,
        data: Some(vec![0, 1, 64, 128, 254, 255].into()),
        ..Default::default()
    });
    doc
}

fn sample(doc: &Document, x: i32, y: i32) -> u8 {
    let m = doc.layers[0].mask.as_ref().unwrap();
    let Some(data) = &m.data else { return 255 };
    if x < m.rect.left || x >= m.rect.right || y < m.rect.top || y >= m.rect.bottom {
        return m.default_color;
    }
    data.sample((y - m.rect.top) as usize * m.rect.width() as usize + (x - m.rect.left) as usize)
        .unwrap()
}

fn write_pixel(doc: &mut Document, x: i32, y: i32, value: u8) {
    let mut view = mask_document_coverage(doc, "0").expect("canvas editing view");
    assert_eq!(
        (view.document.width, view.document.height),
        (doc.width, doc.height)
    );
    view.document.layers[0].channels[0].data[y as usize * doc.width as usize + x as usize] = value;
    assert!(write_mask_back(
        doc,
        "0",
        &view.document,
        Some(rect(x, y, 1, 1))
    ));
}

#[test]
fn mask_creation_covers_canvas() {
    for kind in [LayerMaskKind::RevealAll, LayerMaskKind::HideAll] {
        let mut doc = fixture();
        let before = composite_rgba(&doc);
        assert!(add_layer_mask(&mut doc, "0", kind, None));
        let m = doc.layers[0].mask.as_ref().unwrap();
        assert_eq!(m.rect, rect(0, 0, 12, 8));
        assert_eq!(m.default_color, 0);
        for y in 0..8 {
            for x in 0..12 {
                let white =
                    kind == LayerMaskKind::RevealAll && (2..5).contains(&x) && (1..3).contains(&y);
                assert_eq!(sample(&doc, x, y), if white { 255 } else { 0 });
            }
        }
        if kind == LayerMaskKind::RevealAll {
            assert_eq!(composite_rgba(&doc), before);
        } else {
            assert!(composite_rgba(&doc).data.iter().all(|&v| v == 0));
        }
    }
}

#[test]
fn mask_creation_preserves_empty_and_group_defaults() {
    for group in [false, true] {
        for kind in [LayerMaskKind::RevealAll, LayerMaskKind::FromTransparency] {
            let mut doc = fixture();
            doc.layers[0].is_group = group;
            doc.layers[0].rect = rect(0, 0, 0, 0);
            doc.layers[0].channels.clear();
            assert!(add_layer_mask(&mut doc, "0", kind, None));
            assert_eq!(doc.layers[0].mask.as_ref().unwrap().rect, rect(0, 0, 12, 8));
            assert_eq!(sample(&doc, 11, 7), 255);
            write_pixel(&mut doc, 11, 7, 64);
            assert_eq!(sample(&doc, 11, 7), 64);
        }
    }
}

#[test]
fn mask_creation_preserves_alpha_coordinates() {
    for with_alpha in [false, true] {
        let mut doc = fixture();
        if with_alpha {
            doc.layers[0].channels[3].data = vec![0, 1, 64, 128, 254, 255].into();
        } else {
            doc.layers[0].channels.retain(|c| c.id != -1);
        }
        assert!(add_layer_mask(
            &mut doc,
            "0",
            LayerMaskKind::FromTransparency,
            None
        ));
        assert_eq!(sample(&doc, 0, 0), 0);
        for (i, expected) in [0, 1, 64, 128, 254, 255].into_iter().enumerate() {
            assert_eq!(
                sample(&doc, 2 + (i % 3) as i32, 1 + (i / 3) as i32),
                if with_alpha { expected } else { 255 }
            );
        }
    }
}

#[test]
fn mask_creation_preserves_selection_coordinates() {
    let selection = LayerMask {
        rect: rect(5, 3, 2, 1),
        default_color: 64,
        data: Some(vec![128, 255].into()),
        ..Default::default()
    };
    for kind in [LayerMaskKind::RevealSelection, LayerMaskKind::HideSelection] {
        let mut doc = fixture();
        assert!(add_layer_mask(&mut doc, "0", kind, Some(&selection)));
        let invert = |v| {
            if kind == LayerMaskKind::HideSelection {
                255 - v
            } else {
                v
            }
        };
        assert_eq!(sample(&doc, 0, 0), invert(64));
        assert_eq!(sample(&doc, 5, 3), invert(128));
        assert_eq!(sample(&doc, 6, 3), invert(255));
        assert_eq!(sample(&doc, -1, -1), 0);
    }
}

#[test]
fn mask_creation_retains_offcanvas_samples() {
    for kind in [LayerMaskKind::RevealAll, LayerMaskKind::FromTransparency] {
        let mut doc = fixture();
        doc.layers[0].rect = rect(-2, -1, 3, 2);
        assert!(add_layer_mask(&mut doc, "0", kind, None));
        assert_eq!(sample(&doc, -2, -1), 255);
        assert_eq!(sample(&doc, 11, 7), 0);
        assert!(doc.layers[0].mask.as_ref().unwrap().rect.left <= -2);
    }
    let mut doc = fixture();
    let sel = LayerMask {
        rect: rect(13, 9, 1, 1),
        data: Some(vec![64].into()),
        ..Default::default()
    };
    assert!(add_layer_mask(
        &mut doc,
        "0",
        LayerMaskKind::RevealSelection,
        Some(&sel)
    ));
    assert_eq!(sample(&doc, 13, 9), 64);
}

#[test]
fn mask_creation_refusals_are_atomic() {
    let mut doc = imported(128);
    let original = doc.clone();
    assert!(!add_layer_mask(
        &mut doc,
        "0",
        LayerMaskKind::RevealAll,
        None
    ));
    assert!(!add_layer_mask(
        &mut doc,
        "missing",
        LayerMaskKind::HideAll,
        None
    ));
    assert_eq!(doc, original);
    doc.layers[0].mask = None;
    doc.layers[0].background = true;
    let original = doc.clone();
    assert!(!add_layer_mask_converting_background(
        &mut doc,
        "0",
        LayerMaskKind::RevealSelection,
        None
    ));
    assert_eq!(doc, original);
}

#[test]
fn canvas_mask_adapter_preserves_document_coordinates() {
    for group in [false, true] {
        let mut doc = imported(128);
        doc.layers[0].is_group = group;
        let pixels = doc.layers[0].channels.clone();
        write_pixel(&mut doc, 10, 6, 7);
        assert_eq!(sample(&doc, 10, 6), 7);
        assert_eq!(sample(&doc, 0, 0), 128);
        assert_eq!(sample(&doc, 3, 1), 1);
        assert_eq!(doc.layers[0].channels, pixels);
    }
    let mut doc = fixture();
    doc.layers[0].mask = Some(LayerMask {
        default_color: 128,
        ..Default::default()
    });
    let original = doc.clone();
    let view = mask_document_coverage(&doc, "0").unwrap();
    assert!(view.document.layers[0].channels[0]
        .data
        .iter()
        .all(|&v| v == 255));
    assert_eq!(doc, original);
    write_pixel(&mut doc, 11, 7, 64);
    assert_eq!(sample(&doc, 0, 0), 255);
}

#[test]
fn canvas_mask_preserves_import_defaults() {
    for default in [128, 255] {
        let mut doc = imported(default);
        doc.layers[0].mask.as_mut().unwrap().set_feather(1.0);
        doc.layers[0].mask.as_mut().unwrap().density = 192;
        let original = doc.clone();
        let rendered = composite_rgba(&doc);
        write_pixel(&mut doc, 11, 7, 7);
        for y in 0..8 {
            for x in 0..12 {
                if (x, y) != (11, 7) {
                    assert_eq!(sample(&doc, x, y), sample(&original, x, y));
                }
            }
        }
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().density, 192);
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().feather(), 1.0);
        assert_eq!(composite_rgba(&doc), rendered);
    }
}

#[test]
fn canvas_mask_data_less_import_preserves_visible_coverage() {
    for default in [0, 128] {
        let mut doc = imported(default);
        doc.layers[0].mask.as_mut().unwrap().data = None;
        let original = doc.clone();
        let rendered = composite_rgba(&doc);
        let view = mask_document(&doc, "0").unwrap();
        assert!(view.document.layers[0].channels[0]
            .data
            .iter()
            .all(|&v| v == 255));
        assert_eq!(doc, original);
        write_pixel(&mut doc, 11, 7, 64);
        assert_eq!(sample(&doc, 0, 0), 255);
        assert_eq!(sample(&doc, 11, 7), 64);
        assert_eq!(composite_rgba(&doc), rendered);
    }
}

#[test]
fn canvas_mask_moves_keep_black_exterior() {
    let mut doc = imported(255);
    let pixels = doc.layers[0].channels.clone();
    assert!(translate_layer_mask(&mut doc, "0", 3, 2));
    assert_eq!(sample(&doc, 0, 0), 0);
    assert_eq!(sample(&doc, 5, 3), 0);
    assert_eq!(sample(&doc, 6, 3), 1);
    assert_eq!(doc.layers[0].channels, pixels);
}

#[test]
fn canvas_mask_moves_restore_every_edge() {
    for (dx, dy) in [
        (-1, 0),
        (1, 0),
        (0, -1),
        (0, 1),
        (-30, 0),
        (30, 0),
        (0, -30),
        (0, 30),
        (i32::MIN + 1, 0),
        (0, i32::MIN + 1),
        (-30, -30),
        (30, 30),
        (-30, 30),
        (30, -30),
    ] {
        let mut doc = imported(0);
        let original = doc.clone();
        assert!(translate_layer_mask(&mut doc, "0", dx, dy));
        for y in 1..3 {
            for x in 2..5 {
                assert_eq!(sample(&doc, x + dx, y + dy), sample(&original, x, y));
            }
        }
        if dx == i32::MIN + 1 || dy == i32::MIN + 1 {
            assert!(composite_rgba(&doc).data.iter().all(|&value| value == 0));
        }
        assert!(translate_layer_mask(&mut doc, "0", -dx, -dy));
        for y in 0..8 {
            for x in 0..12 {
                assert_eq!(sample(&doc, x, y), sample(&original, x, y));
            }
        }
    }
}

#[test]
fn canvas_mask_edit_preserves_offcanvas_samples() {
    let mut doc = imported(0);
    assert!(translate_layer_mask(&mut doc, "0", -30, -30));
    for (x, y, v) in [(0, 0, 255), (11, 7, 128)] {
        write_pixel(&mut doc, x, y, v);
    }
    assert!(invert_layer_mask(&mut doc, "0"));
    assert!(translate_layer_mask(&mut doc, "0", 30, 30));
    assert_eq!(
        (sample(&doc, 3, 1), sample(&doc, 4, 1), sample(&doc, 4, 2)),
        (1, 64, 255)
    );
}

fn native_sample(doc: &Document, x: i32, y: i32) -> u32 {
    let layer = &doc.layers[0];
    let m = layer.mask.as_ref().unwrap();
    let i = (y - m.rect.top) as usize * m.rect.width() as usize + (x - m.rect.left) as usize;
    let samples = &layer
        .source_channels
        .as_ref()
        .unwrap()
        .planes
        .iter()
        .find(|(id, _)| *id == -2)
        .unwrap()
        .1;
    match samples {
        Samples::U16(p) => u32::from(p.sample(i).unwrap()),
        Samples::F32(p) => p.sample(i).unwrap().to_bits(),
        Samples::U8(_) => panic!("native samples"),
    }
}

#[test]
fn canvas_mask_native_moves_preserve_precision() {
    for samples in [
        Samples::U16(vec![1, 257, 16384, 32768, 65534, 65535].into()),
        Samples::F32(vec![f32::from_bits(1), 0.0001, 0.25, 0.5, 0.9999, 1.0].into()),
    ] {
        let mut doc = imported(0);
        let depth = if matches!(&samples, Samples::U16(_)) {
            BitDepth::Sixteen
        } else {
            BitDepth::ThirtyTwo
        };
        doc.layers[0].mask.as_mut().unwrap().data = Some(samples.narrow_to_u8().into());
        doc.layers[0].source_channels = Some(SourceChannels::new(
            depth,
            doc.layers[0].rect,
            vec![(-2, samples)],
        ));
        let original = (0..6)
            .map(|i| native_sample(&doc, 2 + i % 3, 1 + i / 3))
            .collect::<Vec<_>>();
        assert!(translate_layer_mask(&mut doc, "0", -30, 30));
        write_pixel(&mut doc, 11, 7, 128);
        assert!(translate_layer_mask(&mut doc, "0", 30, -30));
        assert_eq!(
            (0..6)
                .map(|i| native_sample(&doc, 2 + i % 3, 1 + i / 3))
                .collect::<Vec<_>>(),
            original
        );
        let owner = doc.layers[0].rect;
        let native = doc.layers[0].source_channels.as_ref().unwrap().planes[0]
            .1
            .to_bytes();
        assert!(pictura_core::layer_bounds::grow_layer_to(
            &mut doc.layers[0],
            rect(0, 0, 15, 10),
            "mask native regression"
        )
        .unwrap());
        assert_eq!(
            doc.layers[0].source_channels.as_ref().unwrap().planes[0]
                .1
                .to_bytes(),
            native
        );
        pictura_core::layer_bounds::shrink_layer_to(&mut doc.layers[0], owner);
        assert_eq!(
            doc.layers[0].source_channels.as_ref().unwrap().planes[0]
                .1
                .to_bytes(),
            native
        );
        assert!(set_layer_mask_linked(&mut doc, "0", true));
        assert!(translate_layer_path(&mut doc, "0", 3, 2));
        assert_eq!(
            (0..6)
                .map(|i| native_sample(&doc, 5 + i % 3, 3 + i / 3))
                .collect::<Vec<_>>(),
            original
        );
    }
}

#[test]
fn selected_mask_move_preserves_offcanvas_samples() {
    for duplicate in [false, true] {
        for (dx, dy) in [(-20, 0), (20, 0), (0, -20), (0, 20)] {
            let mut doc = imported(0);
            let mut coverage = vec![0; 96];
            coverage[15] = 255;
            coverage[16] = 255;
            assert!(move_mask_selection_content(
                &mut doc, "0", &coverage, dx, dy, duplicate, 255
            ));
            assert_eq!(sample(&doc, 3 + dx, 1 + dy), 1);
            assert_eq!(sample(&doc, 4 + dx, 1 + dy), 64);
            assert_eq!(sample(&doc, 4, 1), if duplicate { 64 } else { 0 });
        }
    }
    let mut doc = imported(0);
    let mut coverage = vec![0; 96];
    coverage[15] = 255;
    coverage[16] = 255;
    assert!(move_mask_selection_content(
        &mut doc, "0", &coverage, 1, 0, false, 255
    ));
    assert_eq!(
        (sample(&doc, 3, 1), sample(&doc, 4, 1), sample(&doc, 5, 1)),
        (0, 1, 64)
    );
}

#[test]
fn selected_mask_move_blends_toward_black() {
    let mut doc = imported(0);
    doc.layers[0].mask.as_mut().unwrap().data.as_mut().unwrap()[0] = 200;
    let mut coverage = vec![0; 96];
    coverage[14] = 128;
    assert!(move_mask_selection_content(
        &mut doc, "0", &coverage, 5, 0, false, 255
    ));
    assert_eq!((sample(&doc, 2, 1), sample(&doc, 7, 1)), (100, 100));
}

#[test]
fn canvas_mask_properties_invert_preserves_retained_samples() {
    for samples in [
        Samples::U16(vec![1, 257, 16384, 32768, 65534, 65535].into()),
        Samples::F32(vec![f32::from_bits(1), 0.0001, 0.25, 0.5, 0.9999, 1.0].into()),
    ] {
        let mut doc = imported(0);
        let depth = if matches!(&samples, Samples::U16(_)) {
            BitDepth::Sixteen
        } else {
            BitDepth::ThirtyTwo
        };
        doc.layers[0].mask.as_mut().unwrap().data = Some(samples.narrow_to_u8().into());
        doc.layers[0].source_channels = Some(SourceChannels::new(
            depth,
            doc.layers[0].rect,
            vec![(-2, samples)],
        ));
        assert!(translate_layer_mask(&mut doc, "0", -5, 0));
        let sentinel = native_sample(&doc, -2, 1);
        let pixels = doc.layers[0].channels.clone();
        assert!(invert_layer_mask(&mut doc, "0"));
        assert_eq!(native_sample(&doc, -2, 1), sentinel);
        assert_eq!(sample(&doc, 11, 7), 255);
        assert_eq!(
            native_sample(&doc, 11, 7),
            if depth == BitDepth::Sixteen {
                65535
            } else {
                1.0f32.to_bits()
            }
        );
        assert_eq!(doc.layers[0].mask.as_ref().unwrap().default_color, 0);
        assert_eq!(doc.layers[0].channels, pixels);
    }
}

struct HostReset;
impl Drop for HostReset {
    fn drop(&mut self) {
        memory::set_host_for_test(None);
        memory::clear_refusal();
    }
}

#[test]
fn canvas_mask_refusals_are_atomic() {
    let _reset = HostReset;
    let mut doc = imported(128);
    let before = doc.clone();
    assert!(!translate_layer_mask(&mut doc, "0", i32::MAX, 0));
    assert_eq!(doc, before);
    for rect in [
        rect(2, 1, 3, 2),
        PsdRect {
            left: i32::MIN,
            top: 0,
            right: 0,
            bottom: 1,
        },
    ] {
        let mut malformed = imported(0);
        malformed.layers[0].mask.as_mut().unwrap().rect = rect;
        malformed.layers[0].mask.as_mut().unwrap().data = Some(vec![1].into());
        let original = malformed.clone();
        assert!(!translate_layer_mask(&mut malformed, "0", 1, 0));
        assert_eq!(malformed, original);
    }
    let mut edited = mask_document_coverage(&doc, "0").unwrap().document;
    edited.layers[0].channels[0].data[81] = 255;
    memory::set_host_for_test(Some(memory::HostReading {
        available: Some(0),
        ..Default::default()
    }));
    assert!(!translate_layer_mask(&mut doc, "0", 1, 1));
    assert_eq!(doc, before);
    assert!(mask_document_coverage(&doc, "0").is_none());
    assert_eq!(doc, before);
    assert!(!invert_layer_mask(&mut doc, "0"));
    assert_eq!(doc, before);
    for missing in [false, true] {
        let mut retained = before.clone();
        if missing {
            retained.layers[0].mask.as_mut().unwrap().data = None;
        } else {
            retained.layers[0].source_channels = Some(SourceChannels::new(
                BitDepth::Sixteen,
                retained.layers[0].rect,
                vec![(
                    -2,
                    Samples::U16(vec![1, 257, 16384, 32768, 65534, 65535].into()),
                )],
            ));
        }
        let original = retained.clone();
        assert!(!write_mask_back(
            &mut retained,
            "0",
            &edited,
            Some(rect(9, 6, 1, 1))
        ));
        assert_eq!(retained, original);
        let mut selected = vec![0; 96];
        selected[15] = 128;
        assert!(!move_mask_selection_content(
            &mut retained,
            "0",
            &selected,
            30,
            0,
            false,
            255
        ));
        assert_eq!(retained, original);
        assert!(!invert_layer_mask(&mut retained, "0"));
        assert_eq!(retained, original);
    }
    doc.layers[0].mask = None;
    let before = doc.clone();
    assert!(!add_layer_mask(
        &mut doc,
        "0",
        LayerMaskKind::RevealAll,
        None
    ));
    assert_eq!(doc, before);
}
