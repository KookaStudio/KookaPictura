use super::*;

#[test]
fn all_27_blend_keys_round_trip() {
    assert_eq!(BlendMode::LAYER_MODES.len(), 27);
    let mut keys: Vec<[u8; 4]> = Vec::new();
    for mode in BlendMode::LAYER_MODES {
        let key = mode.to_psd_key();
        assert_eq!(BlendMode::from_psd_key(key), Some(mode), "{mode:?}");
        keys.push(key);
    }
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), 27, "blend keys must be unique");
    assert!(
        !BlendMode::LAYER_MODES.contains(&BlendMode::PassThrough),
        "Pass Through is group-only, not a 28th layer mode"
    );
}

#[test]
fn pass_through_maps_to_and_from_pass() {
    assert_eq!(BlendMode::PassThrough.to_psd_key(), *b"pass");
    assert_eq!(
        BlendMode::from_psd_key(*b"pass"),
        Some(BlendMode::PassThrough)
    );
}

#[test]
fn known_key_examples() {
    assert_eq!(BlendMode::Normal.to_psd_key(), *b"norm");
    assert_eq!(BlendMode::Multiply.to_psd_key(), *b"mul ");
    assert_eq!(BlendMode::Screen.to_psd_key(), *b"scrn");
    assert_eq!(BlendMode::ColorBurn.to_psd_key(), *b"idiv");
    assert_eq!(BlendMode::Luminosity.to_psd_key(), *b"lum ");
    assert_eq!(BlendMode::from_psd_key(*b"mul "), Some(BlendMode::Multiply));
}

#[test]
fn blend_names_are_distinct_and_non_empty() {
    assert_eq!(BlendMode::ColorBurn.name(), "Color Burn");
    assert_eq!(BlendMode::Luminosity.name(), "Luminosity");
    let mut names: Vec<&str> = BlendMode::LAYER_MODES.iter().map(BlendMode::name).collect();
    assert!(names.iter().all(|name| !name.is_empty()));
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 27, "blend names must be unique");
}

#[test]
fn unknown_blend_key_is_none() {
    assert_eq!(BlendMode::from_psd_key(*b"zzzz"), None);
    assert_eq!(BlendMode::from_psd_key(*b"nrml"), None);
    assert_eq!(BlendMode::from_psd_key(*b"pas "), None);
}

#[test]
fn extra_block_finds_present_key_and_none_for_absent() {
    let layer = Layer {
        extra_blocks: vec![LayerBlock {
            key: *b"lfx2",
            data: vec![1, 2, 3, 4],
        }],
        ..Default::default()
    };
    assert_eq!(
        layer.extra_block(b"lfx2").map(|b| b.data.as_slice()),
        Some(&[1, 2, 3, 4][..])
    );
    assert!(layer.extra_block(b"SoLd").is_none());
}

#[test]
fn group_vs_pixel_layer() {
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 4,
        right: 4,
    };
    let pixel = Layer {
        name: "Pixel".into(),
        rect,
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![Channel {
            id: 0,
            data: vec![0; 16].into(),
        }],
        children: crate::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    };
    assert!(!pixel.is_group());
    assert_eq!(pixel.channels.len(), 1);

    let group = Layer {
        name: "Group".into(),
        rect,
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: crate::LayerStack::from_layers(vec![pixel]),
        is_group: true,
        background: false,
        ..Default::default()
    };
    assert!(group.is_group());
    assert_eq!(group.children.len(), 1);
}

#[test]
fn mask_present_and_absent() {
    let rect = PsdRect {
        top: 1,
        left: 2,
        bottom: 3,
        right: 4,
    };
    let bare = Layer {
        name: "bare".into(),
        rect,
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: crate::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    };
    assert!(bare.mask.is_none());

    let masked = Layer {
        mask: Some(LayerMask {
            rect,
            default_color: 255,
            disabled: false,
            flags: 0,
            data: Some(vec![255; 4].into()),
            ..Default::default()
        }),
        ..bare
    };
    let mask = masked.mask.expect("mask present");
    assert_eq!(mask.default_color, 255);
    assert_eq!(mask.data.as_deref(), Some(&[255u8, 255, 255, 255][..]));
}

#[test]
fn rect_width_height_signed_and_offset() {
    let outside = PsdRect {
        top: -10,
        left: -20,
        bottom: 30,
        right: 40,
    };
    assert_eq!(outside.width(), 60);
    assert_eq!(outside.height(), 40);

    let offset = PsdRect {
        top: 100,
        left: 50,
        bottom: 150,
        right: 250,
    };
    assert_eq!(offset.width(), 200);
    assert_eq!(offset.height(), 50);
}

#[test]
fn document_new_has_no_layers_and_keeps_composite() {
    let doc = Document::new(3, 2, ColorMode::Rgb, BitDepth::Eight);
    assert!(doc.layers.is_empty());
    assert_eq!(doc.composite.channels, 3);
}

#[test]
fn field_and_method_is_group_agree() {
    let layer = Layer {
        name: "g".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: crate::LayerStack::default(),
        is_group: true,
        background: false,
        ..Default::default()
    };
    assert!(layer.is_group());
}

#[test]
fn default_layer_attribute_values() {
    let layer = Layer {
        name: "d".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: crate::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    };
    assert_eq!(layer.fill, 255);
    assert_eq!(layer.lock.bits(), 0);
    assert_eq!(layer.color, ColorLabel::None);
    assert!(!layer.background, "background defaults to false");

    let mut flagged = layer.clone();
    flagged.background = true;
    assert!(flagged.clone().background, "the flag is cloned");
}

#[test]
fn color_label_byte_round_trip_and_out_of_range() {
    for v in 0u8..=7 {
        assert_eq!(ColorLabel::from_byte(v).to_byte(), v);
    }
    for v in 8u8..=255 {
        assert_eq!(ColorLabel::from_byte(v), ColorLabel::None);
    }
    assert_eq!(ColorLabel::Red.to_byte(), 1);
    assert_eq!(ColorLabel::Gray.to_byte(), 7);
}

#[test]
fn lock_flags_bits_contains_with_and_all() {
    assert_eq!(LockFlags::all().bits(), 0x0F);
    assert!(LockFlags::all().is_all());
    assert!(!LockFlags::default().is_all());
    let t = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    assert!(t.contains(LockFlags::TRANSPARENCY));
    assert!(!t.contains(LockFlags::PIXELS));
    assert!(!t.is_all());
    assert_eq!(t.with(LockFlags::TRANSPARENCY, false).bits(), 0);
    assert!(LockFlags::all().contains(LockFlags::PIXELS));
    assert!(LockFlags::all().contains(LockFlags::POSITION));
    assert!(LockFlags::all().contains(LockFlags::NESTING));
    let n = LockFlags::default().with(LockFlags::NESTING, true);
    assert!(n.contains(LockFlags::NESTING));
    assert!(!n.is_all());
}

fn channel(layer: &Layer, id: i16) -> &[u8] {
    &layer
        .channels
        .iter()
        .find(|c| c.id == id)
        .expect("channel present")
        .data
}

#[test]
fn from_rgba_sets_size_mode_depth_and_one_layer() {
    let doc = Document::from_rgba("photo", 2, 3, &[0u8; 24]);
    assert_eq!((doc.width, doc.height), (2, 3));
    assert_eq!(doc.mode, ColorMode::Rgb);
    assert_eq!(doc.depth, BitDepth::Eight);
    assert_eq!(doc.composite.channels, 4);
    assert_eq!(doc.layers.len(), 1);
    let layer = &doc.layers[0];
    assert_eq!(layer.name, "photo");
    assert_eq!(
        (
            layer.rect.top,
            layer.rect.left,
            layer.rect.bottom,
            layer.rect.right
        ),
        (0, 0, 3, 2)
    );
    assert!(layer.smart_object.is_none());
    assert!(layer.adjustment.is_none());
    assert!(!layer.is_group);
    assert_eq!(
        layer.channels.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![0, 1, 2, -1]
    );
}

#[test]
fn from_rgba_preserves_rgba_including_alpha() {
    // 2x1: pixel 0 red opaque, pixel 1 green half-alpha.
    let rgba = [255, 0, 0, 255, 0, 255, 0, 128];
    let doc = Document::from_rgba("px", 2, 1, &rgba);
    let layer = &doc.layers[0];
    assert_eq!(channel(layer, 0), &[255, 0]);
    assert_eq!(channel(layer, 1), &[0, 255]);
    assert_eq!(channel(layer, 2), &[0, 0]);
    assert_eq!(channel(layer, -1), &[255, 128]);
    let plane = 2;
    assert_eq!(&doc.composite.data[0..2], &[255, 0]);
    assert_eq!(&doc.composite.data[plane..plane + 2], &[0, 255]);
    assert_eq!(&doc.composite.data[3 * plane..3 * plane + 2], &[255, 128]);
}

#[test]
fn from_rgba_short_buffer_does_not_panic() {
    let doc = Document::from_rgba("short", 2, 2, &[1, 2, 3, 4, 5]);
    assert_eq!(doc.layers.len(), 1);
    let layer = &doc.layers[0];
    assert_eq!(channel(layer, 0)[0], 1);
    assert_eq!(channel(layer, -1)[0], 4);
    assert_eq!(channel(layer, 0)[1], 0, "missing pixels stay transparent");
    assert_eq!(channel(layer, -1)[3], 0);
    assert_eq!(doc.composite.data.len(), 16);
}

#[test]
fn retains_source_depth_tracks_the_retained_store() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    assert!(!doc.retains_source_depth(), "8-bit retains nothing");
    // A converted mode records the depth for the notice but keeps no samples.
    doc.source_depth = Some(BitDepth::Sixteen);
    assert!(!doc.retains_source_depth());
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Sixteen,
        width: 1,
        height: 1,
        samples: Samples::U16(vec![0]),
    });
    assert!(doc.retains_source_depth());

    // A layer store alone also counts (a layered file with no composite).
    let mut layered = Document::new(1, 1, ColorMode::Rgb, BitDepth::Sixteen);
    layered.source_depth = Some(BitDepth::Sixteen);
    layered.layers = crate::LayerStack::from_layers(vec![Layer {
        source_channels: Some(SourceChannels {
            depth: BitDepth::Sixteen,
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 1,
            },
            planes: vec![(0, Samples::U16(vec![0]))],
        }),
        ..Default::default()
    }]);
    assert!(layered.retains_source_depth());
}

/// Copy-on-write: a cloned document shares every pixel plane, and a write
/// through the clone forks only the planes it touches, leaving the original
/// byte-identical. That is what lets `Document::clone` run per stroke and per
/// history state without copying the pixels.
#[test]
fn a_cloned_document_shares_its_planes_until_a_write_forks_them() {
    use crate::plane::shares;

    let rgba: Vec<u8> = (0..64).map(|i| (i * 7 % 251) as u8).collect();
    let mut doc = Document::from_rgba("cow", 4, 4, &rgba);
    let snapshot = doc.clone();

    assert!(shares(&doc.composite.data, &snapshot.composite.data));
    assert!(shares(
        &doc.layers[0].channels[0].data,
        &snapshot.layers[0].channels[0].data
    ));
    assert_eq!(doc.composite.data, snapshot.composite.data);

    let composite_before = doc.composite.data[0];
    let plane_before = doc.layers[0].channels[0].data[0];
    let untouched_before = doc.layers[0].channels[1].data[0];

    doc.composite.data[0] = !composite_before;
    doc.layers[0].channels[0].data[0] = !plane_before;

    assert!(!shares(&doc.composite.data, &snapshot.composite.data));
    assert!(!shares(
        &doc.layers[0].channels[0].data,
        &snapshot.layers[0].channels[0].data
    ));
    assert_eq!(
        snapshot.composite.data[0], composite_before,
        "old bytes survive"
    );
    assert_eq!(
        snapshot.layers[0].channels[0].data[0], plane_before,
        "old bytes survive"
    );
    assert_eq!(doc.composite.data[0], !composite_before, "the write landed");
    assert!(
        shares(
            &doc.layers[0].channels[1].data,
            &snapshot.layers[0].channels[1].data
        ),
        "a plane nobody wrote is still shared"
    );
    assert_eq!(doc.layers[0].channels[1].data[0], untouched_before);
}

#[test]
fn psd_rect_extent_saturates_instead_of_overflowing() {
    let rect = PsdRect {
        top: i32::MIN,
        left: i32::MIN,
        bottom: i32::MAX,
        right: i32::MAX,
    };
    assert_eq!((rect.width(), rect.height()), (i32::MAX, i32::MAX));
    let inverted = PsdRect {
        top: i32::MAX,
        left: i32::MAX,
        bottom: i32::MIN,
        right: i32::MIN,
    };
    assert_eq!(inverted.width(), i32::MIN);
}
