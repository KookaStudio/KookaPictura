use super::kernel::Interpolation;
use super::paths::resolve_path;
use super::transform::{
    transform_layer, transform_layer_quad, transform_layer_quad_with, transform_layer_with,
    LayerTransform,
};
use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerMask, LockFlags, PsdRect, RawChannel,
    Samples, SmartObject, SmartObjectKind, SourceChannels, SourcePlanes,
};

fn rect(w: i32, h: i32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h,
        right: w,
    }
}

fn plane(w: u32, h: u32, seed: u8) -> Vec<u8> {
    (0..(w * h) as usize)
        .map(|i| (i as u8).wrapping_mul(7).wrapping_add(seed))
        .collect()
}

fn channel(id: i16, data: Vec<u8>) -> Channel {
    Channel {
        id,
        data: data.into(),
    }
}

fn pixel_layer(w: u32, h: u32) -> Layer {
    let n = (w * h) as usize;
    Layer {
        name: "L".into(),
        rect: rect(w as i32, h as i32),
        channels: vec![
            channel(0, plane(w, h, 3)),
            channel(1, plane(w, h, 11)),
            channel(2, plane(w, h, 29)),
            channel(-1, vec![255; n]),
        ],
        ..Default::default()
    }
}

fn doc_with(layer: Layer) -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![layer];
    doc
}

fn identity() -> LayerTransform {
    LayerTransform {
        scale_x: 1.0,
        scale_y: 1.0,
        angle_radians: 0.0,
        dx: 0.0,
        dy: 0.0,
    }
}

fn transform(sx: f64, sy: f64, angle_radians: f64, dx: f64, dy: f64) -> LayerTransform {
    LayerTransform {
        scale_x: sx,
        scale_y: sy,
        angle_radians,
        dx,
        dy,
    }
}

#[test]
fn identity_is_bit_identical() {
    let mut doc = doc_with(pixel_layer(4, 4));
    let before = doc.clone();
    assert!(transform_layer(&mut doc, "0", identity()));
    assert_eq!(doc, before);
}

#[test]
fn scale_two_doubles_rect_and_planes() {
    let mut doc = doc_with(pixel_layer(4, 2));
    assert!(transform_layer(
        &mut doc,
        "0",
        transform(2.0, 2.0, 0.0, 0.0, 0.0)
    ));
    let layer = resolve_path(&doc, "0").unwrap();
    assert_eq!((layer.rect.width(), layer.rect.height()), (8, 4));
    for ch in &layer.channels {
        assert_eq!(ch.data.len(), 32, "channel {} plane", ch.id);
    }
}

#[test]
fn high_depth_transform_move_keeps_and_scale_resamples_the_native_store() {
    let mut layer = pixel_layer(4, 4);
    layer.raw_channels = vec![RawChannel {
        id: 3,
        data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    }];
    layer.source_channels = Some(SourceChannels {
        depth: BitDepth::Sixteen,
        rect: rect(4, 4),
        planes: vec![(3, Samples::U16(vec![0; 16]))],
    });
    let mut doc = doc_with(layer);
    doc.source_depth = Some(BitDepth::Sixteen);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Sixteen,
        width: 4,
        height: 4,
        samples: Samples::U16(vec![0; 3 * 4 * 4]),
    });

    // A pure integer translation keeps the channel and re-anchors the store.
    assert!(transform_layer(
        &mut doc,
        "0",
        transform(1.0, 1.0, 0.0, 2.0, 1.0)
    ));
    let moved = resolve_path(&doc, "0").unwrap();
    assert_eq!(
        moved.raw_channels.len(),
        1,
        "a pure move keeps the unmodeled channel"
    );
    assert_eq!(moved.source_channels.as_ref().unwrap().rect, moved.rect);
    assert_eq!(
        moved.rect,
        PsdRect {
            top: 1,
            left: 2,
            bottom: 5,
            right: 6
        }
    );
    assert!(
        pictura_codec::write_psd(&doc).is_ok(),
        "a moved layer saves"
    );

    // A scale cannot resample a raw on-disk stream, so it drops that; the native
    // store is resampled to the new bounds.
    assert!(transform_layer(
        &mut doc,
        "0",
        transform(2.0, 2.0, 0.0, 0.0, 0.0)
    ));
    let scaled = resolve_path(&doc, "0").unwrap();
    assert!(
        scaled.raw_channels.is_empty(),
        "a scale drops the unmodeled raw stream"
    );
    let store = scaled
        .source_channels
        .as_ref()
        .expect("a scale keeps a resampled native store");
    assert_eq!(store.rect, scaled.rect);
    assert_eq!(
        store
            .planes
            .iter()
            .find(|(id, _)| *id == 3)
            .unwrap()
            .1
            .len(),
        8 * 8
    );
    let bytes = pictura_codec::write_psd(&doc).expect("a scaled high-depth layer saves");
    assert_eq!(u16::from_be_bytes(bytes[22..24].try_into().unwrap()), 16);
}

/// A 16-bit plane whose 8-bit narrowing is `plane(4, 4, seed)`, but whose low
/// byte is non-zero so it is deliberately not the `v * 257` widening.
fn native_color_store(seed: u8) -> Samples {
    Samples::U16(
        plane(4, 4, seed)
            .into_iter()
            .map(|v| (v as u16 * 257).saturating_add(7))
            .collect(),
    )
}

#[test]
fn pure_move_reemits_native_samples_on_save() {
    let mut layer = pixel_layer(4, 4);
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        layer.rect,
        vec![
            (0, native_color_store(3)),
            (1, native_color_store(11)),
            (2, native_color_store(29)),
            (-1, Samples::U16(vec![65535; 16])),
        ],
    ));
    let mut doc = doc_with(layer);
    doc.source_depth = Some(BitDepth::Sixteen);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Sixteen,
        width: 4,
        height: 4,
        samples: Samples::U16(vec![0; 3 * 16]),
    });

    assert!(transform_layer(
        &mut doc,
        "0",
        transform(1.0, 1.0, 0.0, 2.0, 1.0)
    ));
    let moved = resolve_path(&doc, "0").unwrap();
    assert_eq!(moved.source_channels.as_ref().unwrap().rect, moved.rect);

    let bytes = pictura_codec::write_psd(&doc).expect("a moved high-depth layer saves");
    let back = pictura_codec::read_psd(&bytes).expect("re-read");
    let store = back.layers[0]
        .source_channels
        .as_ref()
        .expect("the native store survived");
    let (_, samples) = store.planes.iter().find(|(id, _)| *id == 0).unwrap();
    let Samples::U16(v) = samples else {
        panic!("expected u16 samples")
    };
    assert!(
        v.iter().any(|s| (*s >> 8) != (*s & 0xff)),
        "the save re-emits native samples, not the 8-bit widen"
    );
}

#[test]
fn quarter_turn_matches_rotate90_cw() {
    let mut doc = doc_with(pixel_layer(4, 4));
    let src = doc.layers[0].channels.clone();
    assert!(transform_layer(
        &mut doc,
        "0",
        transform(1.0, 1.0, std::f64::consts::FRAC_PI_2, 0.0, 0.0)
    ));
    let after = resolve_path(&doc, "0").unwrap();
    assert_eq!(after.rect, rect(4, 4));
    for (c, ch) in after.channels.iter().enumerate() {
        let mut buf = pictura_core::PixelBuffer::new(4, 4, 1);
        buf.data.copy_from_slice(&src[c].data);
        let expected = pictura_ops::rotate90_cw(&buf);
        for (i, (got, want)) in ch.data.iter().zip(expected.data.iter()).enumerate() {
            assert!(
                (*got as i32 - *want as i32).abs() <= 1,
                "channel {} byte {i}: {got} vs {want}",
                ch.id
            );
        }
    }
}

#[test]
fn forty_five_degrees_uses_the_computed_box_with_zeroed_corners() {
    let mut doc = doc_with(pixel_layer(4, 4));
    assert!(transform_layer(
        &mut doc,
        "0",
        transform(1.0, 1.0, std::f64::consts::FRAC_PI_4, 0.0, 0.0)
    ));
    let layer = resolve_path(&doc, "0").unwrap();
    assert_eq!(
        layer.rect,
        PsdRect {
            top: -1,
            left: -1,
            bottom: 5,
            right: 5
        }
    );
    let w = layer.rect.width() as usize;
    for ch in &layer.channels {
        assert_eq!(ch.data.len(), 36);
        assert_eq!(ch.data[0], 0, "top-left outside channel {}", ch.id);
        assert_eq!(ch.data[w - 1], 0, "top-right outside channel {}", ch.id);
        assert_eq!(ch.data[35], 0, "bottom-right outside channel {}", ch.id);
        assert!(ch.data[(2 * w) + 2] > 0, "centre inside channel {}", ch.id);
    }
}

#[test]
fn translation_moves_the_rect_and_copies_source() {
    let mut doc = doc_with(pixel_layer(4, 4));
    let src = doc.layers[0].channels.clone();
    assert!(transform_layer(
        &mut doc,
        "0",
        transform(1.0, 1.0, 0.0, -2.0, 0.0)
    ));
    let layer = resolve_path(&doc, "0").unwrap();
    assert_eq!(
        layer.rect,
        PsdRect {
            top: 0,
            left: -2,
            bottom: 4,
            right: 2
        }
    );
    for (c, ch) in layer.channels.iter().enumerate() {
        assert_eq!(ch.data, src[c].data, "channel {} content", ch.id);
    }
}

#[test]
fn mask_follows_the_layer() {
    let mut layer = pixel_layer(4, 4);
    layer.mask = Some(LayerMask {
        rect: rect(4, 4),
        data: Some(vec![128; 16].into()),
        ..Default::default()
    });
    let mut doc = doc_with(layer);
    assert!(transform_layer(
        &mut doc,
        "0",
        transform(2.0, 2.0, 0.0, 0.0, 0.0)
    ));
    let layer = resolve_path(&doc, "0").unwrap();
    let mask = layer.mask.as_ref().unwrap();
    assert_eq!((mask.rect.width(), mask.rect.height()), (8, 8));
    assert_eq!(mask.data.as_ref().unwrap().len(), 64);
}

#[test]
fn mask_without_data_still_follows_the_layer() {
    let mut layer = pixel_layer(4, 4);
    layer.mask = Some(LayerMask {
        rect: rect(4, 4),
        data: None,
        ..Default::default()
    });
    let mut doc = doc_with(layer);
    assert!(transform_layer(
        &mut doc,
        "0",
        transform(2.0, 2.0, 0.0, 0.0, 0.0)
    ));
    let layer = resolve_path(&doc, "0").unwrap();
    let mask = layer.mask.as_ref().unwrap();
    assert_eq!((mask.rect.width(), mask.rect.height()), (8, 8));
    assert!(mask.data.is_none());
}

#[test]
fn refusals_leave_the_document_bit_identical() {
    let cases: Vec<(&str, Document, LayerTransform)> = vec![
        (
            "group",
            doc_with(Layer {
                is_group: true,
                ..Default::default()
            }),
            identity(),
        ),
        (
            "adjustment",
            doc_with(Layer {
                adjustment: Some(pictura_core::AdjustmentData {
                    key: *b"nvrt",
                    data: Vec::new(),
                }),
                ..Default::default()
            }),
            identity(),
        ),
        (
            "background",
            doc_with(Layer {
                background: true,
                ..pixel_layer(2, 2)
            }),
            identity(),
        ),
        (
            "position-locked",
            doc_with(Layer {
                lock: pictura_core::LockFlags::default().with(LockFlags::POSITION, true),
                ..pixel_layer(2, 2)
            }),
            identity(),
        ),
        (
            "zero-area",
            doc_with(Layer {
                rect: PsdRect {
                    top: 0,
                    left: 0,
                    bottom: 0,
                    right: 0,
                },
                ..pixel_layer(2, 2)
            }),
            identity(),
        ),
        (
            "zero-scale",
            doc_with(pixel_layer(2, 2)),
            transform(0.0, 1.0, 0.0, 0.0, 0.0),
        ),
    ];
    for (name, mut doc, t) in cases {
        let before = doc.clone();
        assert!(!transform_layer(&mut doc, "0", t), "{name} must refuse");
        assert_eq!(doc, before, "{name} must not mutate");
    }
    let mut doc = doc_with(pixel_layer(2, 2));
    let before = doc.clone();
    assert!(!transform_layer(&mut doc, "missing", identity()));
    assert_eq!(doc, before);
}

#[test]
fn one_by_one_does_not_panic() {
    let mut doc = doc_with(pixel_layer(1, 1));
    for angle in [
        0.0,
        std::f64::consts::FRAC_PI_4,
        std::f64::consts::FRAC_PI_2,
    ] {
        let _ = transform_layer(&mut doc, "0", transform(1.0, 1.0, angle, 0.0, 0.0));
    }
}

#[test]
fn absurd_scale_is_refused_without_panic() {
    let mut doc = doc_with(pixel_layer(4, 4));
    let before = doc.clone();
    assert!(!transform_layer(
        &mut doc,
        "0",
        transform(1e12, 1e12, 0.0, 0.0, 0.0)
    ));
    assert_eq!(doc, before);
}

fn embedded_payload(value: u8) -> Vec<u8> {
    let mut embedded = doc_with(pixel_layer(2, 2));
    for ch in &mut embedded.layers[0].channels {
        ch.data.fill(value);
    }
    pictura_codec::write_psd(&embedded).expect("write embedded source")
}

fn channel_less_layer(payload: Vec<u8>) -> Layer {
    Layer {
        name: "SO".into(),
        rect: rect(2, 2),
        channels: Vec::new(),
        smart_object: Some(SmartObject {
            kind: SmartObjectKind::Embedded,
            payload: Some(payload),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn channel_less_embedded_object_materializes_and_consumes() {
    let mut doc = doc_with(channel_less_layer(embedded_payload(200)));
    assert!(transform_layer(
        &mut doc,
        "0",
        transform(2.0, 2.0, 0.0, 0.0, 0.0)
    ));
    let layer = resolve_path(&doc, "0").unwrap();
    assert!(layer.channels.iter().any(|c| c.id == 0));
    assert_eq!((layer.rect.width(), layer.rect.height()), (4, 4));
    assert!(layer.smart_object.is_none());
}

#[test]
fn undecodable_channel_less_target_is_refused_untouched() {
    let mut doc = doc_with(channel_less_layer(b"not a psd".to_vec()));
    let before = doc.clone();
    assert!(!transform_layer(
        &mut doc,
        "0",
        transform(2.0, 2.0, 0.0, 0.0, 0.0)
    ));
    assert_eq!(doc, before);
}

fn src_quad(w: i32, h: i32) -> [(f64, f64); 4] {
    [
        (0.0, 0.0),
        (w as f64, 0.0),
        (w as f64, h as f64),
        (0.0, h as f64),
    ]
}

#[test]
fn quad_identity_is_bit_identical() {
    let mut doc = doc_with(pixel_layer(4, 4));
    let before = doc.clone();
    assert!(transform_layer_quad(&mut doc, "0", src_quad(4, 4)));
    assert_eq!(doc, before);
}

#[test]
fn quad_integer_translation_matches_similarity() {
    let mut quad_doc = doc_with(pixel_layer(4, 4));
    let mut sim_doc = doc_with(pixel_layer(4, 4));
    let pulled = [(2.0, 1.0), (6.0, 1.0), (6.0, 5.0), (2.0, 5.0)];
    assert!(transform_layer_quad(&mut quad_doc, "0", pulled));
    assert!(transform_layer(
        &mut sim_doc,
        "0",
        transform(1.0, 1.0, 0.0, 2.0, 1.0)
    ));
    assert_eq!(quad_doc, sim_doc);
}

#[test]
fn projective_quad_maps_source_corners_onto_targets() {
    let quad = [(0.5, 0.0), (4.0, 1.5), (3.0, 4.5), (0.0, 3.0)];
    let got = super::transform::quad_corner_targets(rect(4, 4), quad).unwrap();
    for (want, g) in quad.iter().zip(got.iter()) {
        assert!(
            (g.0 - want.0).abs() < 1e-6 && (g.1 - want.1).abs() < 1e-6,
            "{g:?} vs {want:?}"
        );
    }
}

#[test]
fn quad_degenerate_inputs_are_refused_bit_identically() {
    let cases: Vec<[(f64, f64); 4]> = vec![
        // Three collinear targets: the homography is singular.
        [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)],
        // A non-finite corner.
        [(0.0, 0.0), (f64::NAN, 0.0), (4.0, 4.0), (0.0, 4.0)],
        // Zero-area: every corner collapses to one point.
        [(1.0, 1.0), (1.0, 1.0), (1.0, 1.0), (1.0, 1.0)],
    ];
    for q in cases {
        let mut doc = doc_with(pixel_layer(4, 4));
        let before = doc.clone();
        assert!(!transform_layer_quad(&mut doc, "0", q), "must refuse {q:?}");
        assert_eq!(doc, before, "must not mutate {q:?}");
    }
}

#[test]
fn quad_past_horizon_draws_only_the_front_side() {
    let mut doc = doc_with(pixel_layer(100, 100));
    let quad = [(0.0, 0.0), (300.0, 0.0), (60.0, 60.0), (0.0, 300.0)];
    let source = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    let map = super::transform::solve_homography(source, quad).unwrap();
    assert!(transform_layer_quad(&mut doc, "0", quad));
    let layer = resolve_path(&doc, "0").unwrap();
    let alpha = layer.channels.iter().find(|c| c.id == -1).unwrap();
    let (w, h) = (layer.rect.width() as usize, layer.rect.height() as usize);
    let (mut drawn, mut behind) = (0, 0);
    for y in 0..h {
        for x in 0..w {
            if alpha.data[y * w + x] == 0 {
                continue;
            }
            drawn += 1;
            let dx = f64::from(layer.rect.left) + x as f64 + 0.5;
            let dy = f64::from(layer.rect.top) + y as f64 + 0.5;
            if map.front() * map.inverse_w(dx, dy) <= 0.0 {
                behind += 1;
            }
        }
    }
    assert!(drawn > 1000, "the front side is drawn ({drawn} px)");
    assert_eq!(behind, 0, "{behind} pixels came from behind the horizon");
}

#[test]
fn quad_out_of_source_pixels_are_zero() {
    let mut doc = doc_with(pixel_layer(4, 4));
    // A trapezoid inside the source bbox: bbox pixels outside it inverse-map
    // outside the source square.
    let quad = [(0.0, 0.0), (4.0, 0.0), (3.0, 4.0), (1.0, 4.0)];
    assert!(transform_layer_quad(&mut doc, "0", quad));
    let layer = resolve_path(&doc, "0").unwrap();
    let alpha = layer.channels.iter().find(|c| c.id == -1).unwrap();
    assert!(alpha.data.contains(&0), "an outside pixel is transparent");
    assert!(alpha.data.contains(&255), "an inside pixel is opaque");
}

#[test]
fn quad_refusals_leave_the_document_bit_identical() {
    let cases: Vec<(&str, Document)> = vec![
        (
            "group",
            doc_with(Layer {
                is_group: true,
                ..Default::default()
            }),
        ),
        (
            "adjustment",
            doc_with(Layer {
                adjustment: Some(pictura_core::AdjustmentData {
                    key: *b"nvrt",
                    data: Vec::new(),
                }),
                ..Default::default()
            }),
        ),
        (
            "background",
            doc_with(Layer {
                background: true,
                ..pixel_layer(2, 2)
            }),
        ),
        (
            "position-locked",
            doc_with(Layer {
                lock: pictura_core::LockFlags::default().with(LockFlags::POSITION, true),
                ..pixel_layer(2, 2)
            }),
        ),
    ];
    for (name, mut doc) in cases {
        let before = doc.clone();
        assert!(
            !transform_layer_quad(&mut doc, "0", src_quad(2, 2)),
            "{name} must refuse"
        );
        assert_eq!(doc, before, "{name} must not mutate");
    }
    let mut doc = doc_with(pixel_layer(2, 2));
    let before = doc.clone();
    assert!(!transform_layer_quad(&mut doc, "missing", src_quad(2, 2)));
    assert_eq!(doc, before);
}

#[test]
fn quad_channel_less_embedded_object_materializes_and_consumes() {
    let mut doc = doc_with(channel_less_layer(embedded_payload(200)));
    let quad = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
    assert!(transform_layer_quad(&mut doc, "0", quad));
    let layer = resolve_path(&doc, "0").unwrap();
    assert!(layer.channels.iter().any(|c| c.id == 0));
    assert!(layer.smart_object.is_none());
}

#[test]
fn quad_one_by_one_does_not_panic() {
    let mut doc = doc_with(pixel_layer(1, 1));
    let _ = transform_layer_quad(&mut doc, "0", src_quad(1, 1));
    let _ = transform_layer_quad(
        &mut doc,
        "0",
        [(0.0, 0.0), (2.0, 1.0), (1.0, 2.0), (0.0, 1.0)],
    );
}

fn checker_layer() -> Layer {
    let mut layer = pixel_layer(4, 4);
    let ch = layer.channels.iter_mut().find(|c| c.id == 0).unwrap();
    for y in 0..4 {
        for x in 0..4 {
            ch.data[y * 4 + x] = if (x + y) % 2 == 0 { 0 } else { 255 };
        }
    }
    layer
}

#[test]
fn legacy_op_equals_the_bilinear_variant() {
    let transform = transform(2.0, 2.0, 0.3, 1.0, -2.0);
    let mut legacy = doc_with(pixel_layer(4, 4));
    let mut explicit = doc_with(pixel_layer(4, 4));
    assert!(transform_layer(&mut legacy, "0", transform));
    assert!(transform_layer_with(
        &mut explicit,
        "0",
        transform,
        Interpolation::Bilinear
    ));
    assert_eq!(legacy, explicit);
}

#[test]
fn nearest_scale_replicates_a_checkerboard() {
    let mut doc = doc_with(checker_layer());
    assert!(transform_layer_with(
        &mut doc,
        "0",
        transform(4.0, 4.0, 0.0, 0.0, 0.0),
        Interpolation::Nearest
    ));
    let layer = resolve_path(&doc, "0").unwrap();
    let ch = layer.channels.iter().find(|c| c.id == 0).unwrap();
    assert_eq!(ch.data.len(), 16 * 16);
    assert!(
        ch.data.iter().all(|&v| v == 0 || v == 255),
        "nearest must not invent intermediate values"
    );
}

#[test]
fn bicubic_differs_from_bilinear_at_scale_two() {
    let mut bilinear = doc_with(pixel_layer(4, 4));
    let mut bicubic = doc_with(pixel_layer(4, 4));
    let t = transform(2.0, 2.0, 0.0, 0.5, 0.5);
    assert!(transform_layer_with(
        &mut bilinear,
        "0",
        t,
        Interpolation::Bilinear
    ));
    assert!(transform_layer_with(
        &mut bicubic,
        "0",
        t,
        Interpolation::Bicubic
    ));
    assert_ne!(bilinear, bicubic);
}

#[test]
fn quad_interpolation_reaches_the_planes() {
    let mut nearest = doc_with(checker_layer());
    let mut bilinear = doc_with(checker_layer());
    let quad = [(0.0, 0.0), (8.0, 0.0), (8.0, 8.0), (0.0, 8.0)];
    assert!(transform_layer_quad_with(
        &mut nearest,
        "0",
        quad,
        Interpolation::Nearest
    ));
    assert!(transform_layer_quad(&mut bilinear, "0", quad));
    let nearest_layer = resolve_path(&nearest, "0").unwrap();
    let ch = nearest_layer.channels.iter().find(|c| c.id == 0).unwrap();
    assert!(ch.data.iter().all(|&v| v == 0 || v == 255));
    assert_ne!(nearest, bilinear);
}

#[test]
fn interpolated_refusal_is_bit_identical() {
    let mut doc = doc_with(Layer {
        background: true,
        ..pixel_layer(2, 2)
    });
    let before = doc.clone();
    assert!(!transform_layer_with(
        &mut doc,
        "0",
        transform(2.0, 2.0, 0.0, 0.0, 0.0),
        Interpolation::Nearest
    ));
    assert_eq!(doc, before);
}
