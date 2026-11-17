use crate::*;
use pictura_adjust::Adjustment;
use pictura_core::{
    assert_no_materialization, BitDepth, Channel, ColorMode, Coverage, Document, Layer, LayerMask,
    LayerStack, Plane, PsdRect,
};
use pictura_filters::Filter;

fn document(w: u32, h: u32) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    };
    let data: Vec<u8> = (0..w as usize * h as usize)
        .map(|i| (i % 197 + 20) as u8)
        .collect();
    doc.layers = LayerStack::from_layers(vec![pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect,
        opacity: 255,
        fill: 255,
        visible: true,
        channels: [0, 1, 2, -1]
            .into_iter()
            .map(|id| Channel {
                id,
                data: if id == -1 {
                    vec![255; data.len()].into()
                } else {
                    data.clone().into()
                },
            })
            .collect(),
        ..Default::default()
    })]);
    doc
}
fn coverage(w: u32, h: u32, value: u8) -> Coverage {
    Coverage::for_canvas(&Plane::filled(w as usize, h as usize, 1, value), w, h).unwrap()
}

#[test]
fn coverage_constraints_distinguish_absent_and_empty() {
    let doc = document(4, 4);
    let baseline = &doc.layers[0];
    let mut unrestricted = baseline.clone();
    apply_adjustment_region(&mut unrestricted, &Adjustment::Invert, None, baseline.rect).unwrap();
    for (value, expected) in [
        (0, baseline.channels()[0].data.sample(0).unwrap()),
        (255, unrestricted.channels()[0].data.sample(0).unwrap()),
        (128, 128),
    ] {
        let mut layer = baseline.clone();
        apply_adjustment_region(
            &mut layer,
            &Adjustment::Invert,
            Some(&coverage(4, 4, value)),
            baseline.rect,
        )
        .unwrap();
        assert_eq!(layer.channels()[0].data.sample(0).unwrap(), expected);
    }
}

#[test]
fn renderer_edits_share_coverage_input() {
    let mut doc = document(8, 8);
    doc.layers[0] = document(6, 6).layers[0].clone();
    doc.layers[0].rect = PsdRect {
        left: 1,
        top: 1,
        right: 7,
        bottom: 7,
    };
    let clip = Coverage::new(
        PsdRect {
            left: 2,
            top: 1,
            right: 6,
            bottom: 7,
        },
        Plane::filled(4, 6, 1, 128),
        0,
    )
    .unwrap();
    let mut layer = doc.layers[0].clone();
    let outside = layer.channels()[0].data.sample(0);
    let region = layer.rect;
    apply_filter(
        &mut layer,
        &Filter::GaussianBlur { radius: 2.0 },
        Some(&clip),
        false,
    )
    .unwrap();
    apply_adjustment_region(&mut layer, &Adjustment::Invert, Some(&clip), region).unwrap();
    remove_matte(&mut layer, MatteBackground::Black, Some(&clip)).unwrap();
    defringe(&mut layer, 1, Some(&clip)).unwrap();
    assert_eq!(layer.channels()[0].data.sample(0), outside);
    let params = ApplyImageParams {
        source: std::sync::Arc::new(
            ApplyImagePlanes::from_coverage(8, 8, Plane::filled(8, 8, 1, 255)).unwrap(),
        ),
        invert: false,
        mode: ApplyImageMode::Layer(pictura_core::BlendMode::Normal),
        opacity: 1.0,
        preserve_transparency: true,
        mask: None,
        mask_invert: false,
    };
    apply_image_region(
        &mut layer,
        &params,
        Some(&clip),
        region,
        ApplyImageTarget {
            origin: (0, 0),
            gray: false,
            native: false,
        },
    )
    .unwrap();
    assert_eq!(layer.channels()[0].data.sample(0), outside);
    assert!(FilterProxy::new(&doc, Some(&clip), region, 0.5)
        .unwrap()
        .render("0", &[], false)
        .is_ok());
    let mut copy = doc.clone();
    assert!(!layer_via_copy(&mut copy, "0", &clip).is_empty());
    assert!(!layer_via_cut(&mut copy, "0", &clip).is_empty());
    assert!(move_selection_content(&mut copy, "0", &clip, 1, 0, true));
    assert!(add_layer_mask(
        &mut copy,
        "0",
        LayerMaskKind::RevealSelection,
        Some(&clip)
    ));
}

#[test]
fn clipboard_reads_shared_coverage() {
    let doc = document(8, 8);
    let clip_rect = PsdRect {
        left: 2,
        top: 3,
        right: 4,
        bottom: 5,
    };
    let flat = Coverage::new(clip_rect, vec![255, 128, 64, 0].into(), 0).unwrap();
    let mut plane = flat.plane_within(clip_rect).unwrap();
    assert!(plane.tile(2, 2));
    let tiled = Coverage::new(clip_rect, plane, 0).unwrap();
    let a = copy_layer(&doc, "0", Some(&flat)).unwrap();
    let b = assert_no_materialization(|| copy_layer(&doc, "0", Some(&tiled))).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.rect, clip_rect);
    assert_eq!(a.rgba.len(), 16);
    assert_eq!(
        copy_merged(&doc, &composite_rgba(&doc), Some(&tiled)).unwrap(),
        a
    );
    let mut locked = doc.clone();
    locked.layers[0].lock =
        pictura_core::LockFlags::default().with(pictura_core::LockFlags::PIXELS, true);
    let before = locked.clone();
    assert!(!clear_layer(&mut locked, "0", Some(&tiled)));
    assert_eq!(locked, before);
    let mut cleared = doc.clone();
    assert!(clear_layer(&mut cleared, "0", Some(&tiled)));
    assert_eq!(cleared.layers[0].channels()[3].data.sample(0), Some(255));
    assert!(copy_layer(&doc, "0", Some(&Coverage::constant(0))).is_none());
    for mode in [PasteMode::Into, PasteMode::Outside] {
        let mut pasted = doc.clone();
        assert!(!paste_clip(&mut pasted, "0", &a, (2, 3), mode, Some(&tiled)).is_empty());
    }
}

#[test]
fn coverage_preserves_real_mask_semantics() {
    let mut doc = document(8, 8);
    let clip = coverage(8, 8, 128);
    assert!(add_layer_mask(
        &mut doc,
        "0",
        LayerMaskKind::RevealSelection,
        Some(&clip)
    ));
    let mask = doc.layers[0].mask.as_mut().unwrap();
    assert_eq!(mask.density, 255);
    mask.density = 128;
    mask.set_feather(2.0);
    mask.flags = 9;
    let original = mask.clone();
    let rendered = composite_rgba(&doc);
    assert_eq!(doc.layers[0].mask.as_ref().unwrap(), &original);
    doc.layers[0].mask.as_mut().unwrap().disabled = true;
    let disabled = composite_rgba(&doc);
    assert_ne!(rendered, disabled);
    doc.layers[0].mask.as_mut().unwrap().data = None;
    doc.layers[0].mask.as_mut().unwrap().disabled = false;
    assert_eq!(composite_rgba(&doc), disabled);
    let raw = original.raw_coverage().unwrap();
    assert_eq!(raw.sample(0, 0), 128);
    assert_eq!(original.flags, 9);
    let mut retained = document(8, 8);
    let samples =
        pictura_core::Samples::U16(vec![1, 257, 16384, 32768, 65534, 65535, 17, 33].into());
    retained.layers[0].mask = Some(LayerMask {
        rect: PsdRect {
            left: -2,
            top: 1,
            right: 2,
            bottom: 3,
        },
        default_color: 128,
        data: Some(samples.narrow_to_u8().into()),
        density: 192,
        feather_bits: 1.0_f64.to_bits(),
        flags: 9,
        ..Default::default()
    });
    retained.layers[0].source_channels = Some(pictura_core::SourceChannels::new(
        BitDepth::Sixteen,
        retained.layers[0].rect,
        vec![(-2, samples.clone())],
    ));
    let before = retained.clone();
    let _ = composite_rgba(&retained);
    assert_eq!(retained, before);
    let mut edit = mask_document(&retained, "0").unwrap();
    edit.document.layers[0].channels_mut()[0]
        .data
        .write_rect_from(7..8, 7..8, 8, &[64]);
    assert!(write_mask_back(
        &mut retained,
        "0",
        &edit.document,
        Some(PsdRect {
            left: 7,
            top: 7,
            right: 8,
            bottom: 8
        })
    ));
    let mask = retained.layers[0].mask.as_ref().unwrap();
    assert_eq!(mask.flags, 9);
    assert_eq!(mask.density, 192);
    assert_eq!(mask.feather(), 1.0);
    let retained_index = ((1 - mask.rect.top) * mask.rect.width() + (-2 - mask.rect.left)) as usize;
    assert_eq!(mask.data.as_ref().unwrap().sample(retained_index), Some(0));
    assert_eq!(mask.raw_coverage().unwrap().sample(7, 7), 64);
    let native = &retained.layers[0]
        .source_channels
        .as_ref()
        .unwrap()
        .planes
        .iter()
        .find(|(id, _)| *id == -2)
        .unwrap()
        .1;
    match native {
        pictura_core::Samples::U16(values) => assert_eq!(values.sample(retained_index), Some(1)),
        _ => panic!("native mask precision was discarded"),
    }
}

#[test]
fn feathered_sparse_coverage_matches_flat() {
    let w = 1100;
    let h = 900;
    let selection = pictura_select::Selection::rect(w, h, 260, 260, 570, 570).feather(4.0);
    let bytes = selection.coverage_bytes();
    assert!(bytes.iter().any(|v| *v > 0 && *v < 255));
    let flat = Coverage::for_canvas(&bytes.into(), w, h).unwrap();
    let tiled = selection.coverage();
    assert!(selection.data.is_tiled());
    for class in 0..3 {
        assert!(
            (0..selection.data.tile_count()).any(|i| match selection.data.tile_state(i) {
                Some(pictura_core::TileState::Empty) => class == 0,
                Some(pictura_core::TileState::Uniform(255)) => class == 1,
                Some(pictura_core::TileState::Dense(_)) => class == 2,
                _ => false,
            })
        );
    }
    let doc = document(w, h);
    let (mut a, mut b) = (doc.layers[0].clone(), doc.layers[0].clone());
    let filter = Filter::GaussianBlur { radius: 1.0 };
    apply_filter(&mut a, &filter, Some(&flat), false).unwrap();
    assert_no_materialization(|| apply_filter(&mut b, &filter, Some(&tiled), false)).unwrap();
    assert_eq!(a, b);
}

#[test]
fn feathered_selection_also_clips_paint_without_materialization() {
    let (w, h) = (1100, 900);
    let selection = pictura_select::Selection::rect(w, h, 260, 260, 570, 570).feather(4.0);
    let flat = Coverage::for_canvas(&selection.coverage_bytes().into(), w, h).unwrap();
    let tiled = selection.coverage();
    let (mut expected, mut actual) = (document(w, h), document(w, h));
    let bounds = expected.layers[0].rect;
    let mask = Plane::filled(w as usize, h as usize, 1, 255);
    let paint = pictura_paint::bucket::BucketPaint::Foreground(pictura_paint::Rgba {
        r: 250,
        g: 20,
        b: 4,
        a: 255,
    });
    let run = |doc: &mut Document, clip: &Coverage| {
        pictura_paint::bucket::fill_tiled(
            doc,
            "0",
            bounds,
            &mask,
            paint,
            pictura_paint::PaintMode::Normal,
            1.0,
            Some(clip),
            pictura_paint::Alpha::Blend,
        )
    };
    assert!(run(&mut expected, &flat).is_some());
    assert!(assert_no_materialization(|| run(&mut actual, &tiled)).is_some());
    assert_eq!(expected, actual);
}
