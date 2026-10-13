use super::*;
use pictura_codec::{decode_pictura_raw_settings, read_psd, write_psd};
use pictura_core::{
    Channel, CrsSettings, LayerBlock, PicturaRawSettings, SmartObject, SmartObjectKind,
};

fn section_has_uuid(bytes: &[u8], uuid: &str) -> bool {
    !uuid.is_empty() && bytes.windows(uuid.len()).any(|w| w == uuid.as_bytes())
}

fn solid_doc(w: u32, h: u32, rgb: [u8; 3]) -> Document {
    let n = (w * h) as usize;
    let mut d = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    for i in 0..n {
        d.composite.data[i] = rgb[0];
        d.composite.data[n + i] = rgb[1];
        d.composite.data[2 * n + i] = rgb[2];
    }
    d
}

pub(super) fn payload_of(rgb: [u8; 3]) -> Vec<u8> {
    write_psd(&solid_doc(2, 2, rgb)).expect("payload writes")
}

pub(super) fn payload_2x2(colors: [[u8; 3]; 4]) -> Vec<u8> {
    let mut d = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    for (i, c) in colors.iter().enumerate() {
        d.composite.data[i] = c[0];
        d.composite.data[4 + i] = c[1];
        d.composite.data[8 + i] = c[2];
    }
    write_psd(&d).expect("payload writes")
}

fn absent_composite_layers_payload(rgb: (u8, u8, u8)) -> Vec<u8> {
    let mut d = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    d.layers = vec![solid("green", full(4, 4), rgb, 255, BlendMode::Normal, 255)];
    let bytes = write_psd(&d).expect("payload writes");
    let section_len = u32::from_be_bytes(bytes[34..38].try_into().unwrap()) as usize;
    bytes[..38 + section_len].to_vec()
}

pub(super) fn embedded(payload: Vec<u8>) -> SmartObject {
    SmartObject {
        filename: "source.psd".into(),
        kind: SmartObjectKind::Embedded,
        payload: Some(payload),
        ..Default::default()
    }
}

pub(super) fn smart_layer(
    name: &str,
    r: PsdRect,
    so: SmartObject,
    channels: Vec<Channel>,
) -> Layer {
    Layer {
        name: name.into(),
        rect: r,
        channels,
        smart_object: Some(so),
        ..Default::default()
    }
}

pub(super) fn green_proxy() -> Vec<Channel> {
    vec![
        Channel {
            id: 0,
            data: vec![0; 16].into(),
        },
        Channel {
            id: 1,
            data: vec![255; 16].into(),
        },
        Channel {
            id: 2,
            data: vec![0; 16].into(),
        },
        Channel {
            id: -1,
            data: vec![255; 16].into(),
        },
    ]
}

fn assert_close(buf: &PixelBuffer, x: u32, y: u32, expected: [u8; 4]) {
    let got = px(buf, x, y);
    for i in 0..4 {
        assert!(
            (got[i] as i32 - expected[i] as i32).abs() <= 1,
            "at {x},{y}: got {got:?}, want {expected:?}"
        );
    }
}

fn assert_all_alpha_zero(buf: &PixelBuffer) {
    let plane = (buf.width * buf.height) as usize;
    for i in 0..plane {
        assert_eq!(buf.data[3 * plane + i], 0, "pixel {i} is not transparent");
    }
}

fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[test]
fn embedded_source_renders_without_proxy() {
    let layer = smart_layer(
        "smart",
        full(4, 4),
        embedded(payload_of([255, 0, 0])),
        Vec::new(),
    );
    let out = composite_rgba(&doc(4, 4, vec![layer]));
    for y in 0..4 {
        for x in 0..4 {
            assert_close(&out, x, y, [255, 0, 0, 255]);
        }
    }
}

#[test]
fn proxy_wins_over_source() {
    let layer = smart_layer(
        "smart",
        full(4, 4),
        embedded(payload_of([255, 0, 0])),
        green_proxy(),
    );
    let out = composite_rgba(&doc(4, 4, vec![layer]));
    for y in 0..4 {
        for x in 0..4 {
            assert_close(&out, x, y, [0, 255, 0, 255]);
        }
    }
}

#[test]
fn undecodable_payload_is_a_noop() {
    let so = SmartObject {
        kind: SmartObjectKind::Embedded,
        payload: Some(vec![0, 1, 2]),
        ..Default::default()
    };
    let layer = smart_layer("smart", full(4, 4), so, Vec::new());
    let out = composite_rgba(&doc(4, 4, vec![layer]));
    assert_all_alpha_zero(&out);
}

#[test]
fn empty_payload_is_a_noop() {
    let so = SmartObject {
        kind: SmartObjectKind::Embedded,
        payload: Some(Vec::new()),
        ..Default::default()
    };
    let layer = smart_layer("smart", full(4, 4), so, Vec::new());
    let out = composite_rgba(&doc(4, 4, vec![layer]));
    assert_all_alpha_zero(&out);
}

#[test]
fn absent_composite_falls_back_to_layers() {
    let layer = smart_layer(
        "smart",
        full(4, 4),
        embedded(absent_composite_layers_payload((0, 255, 0))),
        Vec::new(),
    );
    let backdrop = solid("blue", full(4, 4), (0, 0, 255), 255, BlendMode::Normal, 255);
    let out = composite_rgba(&doc(4, 4, vec![backdrop, layer]));
    for y in 0..4 {
        for x in 0..4 {
            assert_close(&out, x, y, [0, 255, 0, 255]);
        }
    }
}

#[test]
fn channel_less_non_smart_layer_still_paints_black() {
    let layer = Layer {
        name: "bare".into(),
        rect: full(4, 4),
        channels: Vec::new(),
        ..Default::default()
    };
    let out = composite_rgba(&doc(4, 4, vec![layer]));
    for y in 0..4 {
        for x in 0..4 {
            assert_close(&out, x, y, [0, 0, 0, 255]);
        }
    }
}

#[test]
fn external_and_unresolved_do_not_render() {
    for kind in [
        SmartObjectKind::External,
        SmartObjectKind::Alias,
        SmartObjectKind::Unresolved,
    ] {
        let so = SmartObject {
            kind,
            payload: Some(payload_of([255, 0, 0])),
            ..Default::default()
        };
        let layer = smart_layer("smart", full(4, 4), so, Vec::new());
        let out = composite_rgba(&doc(4, 4, vec![layer]));
        assert_all_alpha_zero(&out);
    }
}

#[test]
fn source_scales_into_layer_rect() {
    let colors = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]];
    let layer = smart_layer(
        "smart",
        full(4, 4),
        embedded(payload_2x2(colors)),
        Vec::new(),
    );
    let out = composite_rgba(&doc(4, 4, vec![layer]));
    let expected = [
        (0, 0, [255, 0, 0, 255]),
        (2, 0, [0, 255, 0, 255]),
        (0, 2, [0, 0, 255, 255]),
        (2, 2, [255, 255, 255, 255]),
    ];
    for (x, y, want) in expected {
        assert_close(&out, x, y, want);
    }
}

#[test]
fn author_and_render_round_trip() {
    let layer = smart_layer(
        "smart",
        full(4, 4),
        embedded(payload_of([255, 0, 0])),
        Vec::new(),
    );
    let bytes = write_psd(&doc(4, 4, vec![layer])).expect("host writes");
    let back = read_psd(&bytes).expect("host reads");
    let out = composite_rgba(&back);
    for y in 0..4 {
        for x in 0..4 {
            assert_close(&out, x, y, [255, 0, 0, 255]);
        }
    }
}

#[test]
fn convert_authors_embedded_object_and_keeps_proxy() {
    let layer = solid(
        "Raster",
        full(4, 4),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let mut d = doc(4, 4, vec![layer]);
    let before = d.clone();
    let before_comp = composite_rgba(&d);

    assert!(can_convert_to_smart_object(&d, "0"));
    assert!(convert_to_smart_object(&mut d, "0"));

    let so = d.layers[0].smart_object.as_ref().expect("smart object");
    assert_eq!(so.kind, SmartObjectKind::Embedded);
    assert!(so.payload.as_deref().is_some_and(|p| !p.is_empty()));
    assert_eq!(so.filename, "Raster.psd");
    assert_eq!(so.filetype, *b"8BPB");
    assert_eq!(so.creator, *b"8BIM");
    assert_eq!(d.layers[0].channels, before.layers[0].channels);
    assert_eq!(composite_rgba(&d), before_comp);

    let embedded = read_psd(so.payload.as_deref().unwrap()).expect("payload reads");
    assert_eq!(embedded.width, 4);
    assert_eq!(embedded.height, 4);
    assert_eq!(embedded.composite.channels, 3);
    for i in 0..16 {
        assert_eq!(embedded.composite.data[i], 10);
        assert_eq!(embedded.composite.data[16 + i], 20);
        assert_eq!(embedded.composite.data[32 + i], 30);
    }
}

#[test]
fn converted_layer_round_trips() {
    let layer = solid(
        "Raster",
        full(4, 4),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let mut d = doc(4, 4, vec![layer]);
    assert!(convert_to_smart_object(&mut d, "0"));
    let payload = d.layers[0]
        .smart_object
        .as_ref()
        .unwrap()
        .payload
        .clone()
        .unwrap();

    let bytes = write_psd(&d).expect("host writes");
    let back = read_psd(&bytes).expect("host reads");
    let so = back.layers[0].smart_object.as_ref().expect("smart object");
    assert_eq!(so.kind, SmartObjectKind::Embedded);
    assert_eq!(so.payload.as_deref(), Some(payload.as_slice()));
    assert_eq!(back.layers[0].channels, d.layers[0].channels);
    assert_eq!(composite_rgba(&back), composite_rgba(&d));
}

#[test]
fn convert_refuses_ineligible_targets() {
    let cases: Vec<(&str, Document)> = vec![
        (
            "group",
            doc(4, 4, vec![group("G", BlendMode::Normal, 255, None, vec![])]),
        ),
        (
            "adjustment",
            doc(
                4,
                4,
                vec![adjustment_layer("Adj", *b"inv ", vec![0], 255, None)],
            ),
        ),
        ("background", {
            let mut d = doc(
                4,
                4,
                vec![solid(
                    "Background",
                    full(4, 4),
                    (0, 0, 0),
                    255,
                    BlendMode::Normal,
                    255,
                )],
            );
            d.layers[0].background = true;
            d
        }),
        (
            "zero-size",
            doc(
                4,
                4,
                vec![solid(
                    "Zero",
                    rect(0, 0, 0, 0),
                    (1, 2, 3),
                    255,
                    BlendMode::Normal,
                    255,
                )],
            ),
        ),
    ];
    for (name, mut d) in cases {
        let before = d.clone();
        assert!(!can_convert_to_smart_object(&d, "0"), "can: {name}");
        assert!(!convert_to_smart_object(&mut d, "0"), "convert: {name}");
        assert_eq!(d, before, "mutated: {name}");
    }

    let mut d = doc(
        4,
        4,
        vec![solid(
            "Raster",
            full(4, 4),
            (1, 2, 3),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let before = d.clone();
    assert!(!convert_to_smart_object(&mut d, "99"));
    assert!(!convert_to_smart_object(&mut d, "bad"));
    assert_eq!(d, before);

    assert!(convert_to_smart_object(&mut d, "0"));
    let converted = d.clone();
    assert!(!convert_to_smart_object(&mut d, "0"));
    assert_eq!(d, converted);
}

#[test]
fn sixteen_bit_document_refuses_conversion() {
    let mut d = Document::new(4, 4, ColorMode::Rgb, BitDepth::Sixteen);
    d.layers = vec![solid(
        "Raster",
        full(4, 4),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    )];
    let before = d.clone();
    assert!(!can_convert_to_smart_object(&d, "0"));
    assert!(!convert_to_smart_object(&mut d, "0"));
    assert_eq!(d, before);
}

#[test]
fn grayscale_layer_converts_to_grayscale_source() {
    let mut d = Document::new(2, 2, ColorMode::Grayscale, BitDepth::Eight);
    d.layers = vec![Layer {
        name: "Gray".into(),
        rect: full(2, 2),
        channels: vec![
            Channel {
                id: 0,
                data: vec![128; 4].into(),
            },
            Channel {
                id: -1,
                data: vec![255; 4].into(),
            },
        ],
        ..Default::default()
    }];
    assert!(convert_to_smart_object(&mut d, "0"));
    let payload = d.layers[0]
        .smart_object
        .as_ref()
        .unwrap()
        .payload
        .clone()
        .unwrap();
    let embedded = read_psd(&payload).expect("payload reads");
    assert_eq!(embedded.mode, ColorMode::Grayscale);
    assert!(embedded.composite.data.iter().all(|&b| b == 128));
}

#[test]
fn rasterize_keeps_proxy_channels_and_drops_config() {
    let mut layer = solid(
        "Raster",
        full(4, 4),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    layer.smart_object = Some(embedded(payload_of([1, 2, 3])));
    layer.extra_blocks = vec![LayerBlock {
        key: *b"SoLd",
        data: vec![9, 9, 9, 9],
    }];
    let mut d = doc(4, 4, vec![layer]);
    let before = d.clone();
    let before_comp = composite_rgba(&d);

    assert!(can_rasterize_smart_object(&d, "0"));
    assert!(rasterize_smart_object(&mut d, "0"));

    assert_eq!(d.layers[0].channels, before.layers[0].channels);
    assert!(d.layers[0].smart_object.is_none());
    assert!(!d.layers[0].extra_blocks.iter().any(|b| &b.key == b"SoLd"));
    assert!(!can_rasterize_smart_object(&d, "0"));
    assert_eq!(composite_rgba(&d), before_comp);
}

#[test]
fn rasterize_materializes_source_into_channels() {
    let layer = smart_layer(
        "smart",
        full(4, 4),
        embedded(payload_of([255, 0, 0])),
        Vec::new(),
    );
    let mut d = doc(4, 4, vec![layer]);
    let before_comp = composite_rgba(&d);

    assert!(can_rasterize_smart_object(&d, "0"));
    assert!(rasterize_smart_object(&mut d, "0"));

    let channel = |id: i16| d.layers[0].channels.iter().find(|c| c.id == id).unwrap();
    assert_eq!(channel(0).data, vec![255u8; 16]);
    assert_eq!(channel(1).data, vec![0u8; 16]);
    assert_eq!(channel(2).data, vec![0u8; 16]);
    assert_eq!(channel(-1).data, vec![255u8; 16]);
    assert!(d.layers[0].smart_object.is_none());
    assert_eq!(composite_rgba(&d), before_comp);
}

#[test]
fn rasterize_refuses_ineligible_targets() {
    let mut plain = doc(
        4,
        4,
        vec![solid(
            "Raster",
            full(4, 4),
            (1, 2, 3),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let before = plain.clone();
    assert!(!can_rasterize_smart_object(&plain, "0"));
    assert!(!rasterize_smart_object(&mut plain, "0"));
    assert!(!rasterize_smart_object(&mut plain, "bad"));
    assert_eq!(plain, before);

    let mut grouped = doc(
        4,
        4,
        vec![{
            let mut g = group("G", BlendMode::Normal, 255, None, vec![]);
            g.smart_object = Some(embedded(payload_of([1, 2, 3])));
            g
        }],
    );
    let before = grouped.clone();
    assert!(!can_rasterize_smart_object(&grouped, "0"));
    assert!(!rasterize_smart_object(&mut grouped, "0"));
    assert_eq!(grouped, before);

    let mut adjustment = doc(
        4,
        4,
        vec![{
            let mut a = adjustment_layer("Adj", *b"inv ", vec![0], 255, None);
            a.smart_object = Some(embedded(payload_of([1, 2, 3])));
            a
        }],
    );
    let before = adjustment.clone();
    assert!(!can_rasterize_smart_object(&adjustment, "0"));
    assert!(!rasterize_smart_object(&mut adjustment, "0"));
    assert_eq!(adjustment, before);

    let undecodable = smart_layer("smart", full(4, 4), embedded(vec![0, 1, 2]), Vec::new());
    let mut d = doc(4, 4, vec![undecodable]);
    let before = d.clone();
    assert!(can_rasterize_smart_object(&d, "0"));
    assert!(!rasterize_smart_object(&mut d, "0"));
    assert_eq!(d, before);

    let empty = smart_layer("smart", full(4, 4), embedded(Vec::new()), Vec::new());
    let mut d = doc(4, 4, vec![empty]);
    let before = d.clone();
    assert!(!rasterize_smart_object(&mut d, "0"));
    assert_eq!(d, before);
}

#[test]
fn oversized_rect_source_is_clipped_to_canvas() {
    let layer = smart_layer(
        "smart",
        rect(0, 0, 100, 100),
        embedded(payload_of([7, 8, 9])),
        Vec::new(),
    );
    let out = composite_rgba(&doc(4, 4, vec![layer]));
    for y in 0..4 {
        for x in 0..4 {
            assert_close(&out, x, y, [7, 8, 9, 255]);
        }
    }

    // The output buffer is bounded by the requested region, not the layer rect.
    let so = embedded(payload_of([7, 8, 9]));
    let buf = render_smart_source(&so, rect(-50, -50, 100_000, 100_000), rect(0, 0, 4, 4))
        .expect("region renders");
    assert_eq!((buf.width, buf.height), (4, 4));
}

#[test]
fn rasterize_preserves_layout_fields() {
    let mut layer = smart_layer(
        "smart",
        rect(-1, -2, 4, 5),
        embedded(payload_of([255, 0, 0])),
        Vec::new(),
    );
    layer.blend = BlendMode::Multiply;
    layer.opacity = 200;
    layer.fill = 128;
    layer.mask = Some(LayerMask::default());
    let mut d = doc(8, 8, vec![layer]);
    let before = d.layers[0].clone();

    assert!(rasterize_smart_object(&mut d, "0"));

    let after = &d.layers[0];
    assert_eq!(after.name, before.name);
    assert_eq!(after.rect, before.rect);
    assert_eq!(after.blend, before.blend);
    assert_eq!(after.opacity, before.opacity);
    assert_eq!(after.fill, before.fill);
    assert_eq!(after.mask, before.mask);
    assert!(after.smart_object.is_none());
}

#[test]
fn rasterize_drops_preserved_config_and_linked_record_round_trip() {
    let layer = solid(
        "Raster",
        full(4, 4),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let mut d = doc(4, 4, vec![layer]);
    assert!(convert_to_smart_object(&mut d, "0"));
    let saved = write_psd(&d).expect("first write");

    let mut back = read_psd(&saved).expect("first read");
    let uuid = back.layers[0]
        .smart_object
        .as_ref()
        .expect("resolved")
        .uuid
        .clone();
    assert_eq!(uuid.len(), 36);
    assert!(back.layers[0]
        .extra_blocks
        .iter()
        .any(|b| &b.key == b"SoLd"));
    assert!(section_has_uuid(&back.layer_section_extra, &uuid));

    assert!(rasterize_smart_object(&mut back, "0"));
    assert!(back.layers[0].smart_object.is_none());
    assert!(!back.layers[0]
        .extra_blocks
        .iter()
        .any(|b| matches!(&b.key, b"SoLd" | b"SoLE" | b"plLd" | b"PlLd")));
    assert!(!section_has_uuid(&back.layer_section_extra, &uuid));

    let resaved = write_psd(&back).expect("second write");
    let reread = read_psd(&resaved).expect("second read");
    assert!(reread.layers[0].smart_object.is_none());
    assert!(!section_has_uuid(&reread.layer_section_extra, &uuid));
}

#[test]
fn convert_keeps_a_linked_record_a_duplicate_still_references() {
    let layer = solid("SO", full(4, 4), (10, 20, 30), 255, BlendMode::Normal, 255);
    let mut d = doc(4, 4, vec![layer]);
    assert!(convert_to_smart_object(&mut d, "0"));
    let saved = write_psd(&d).expect("first write");
    let mut back = read_psd(&saved).expect("first read");
    let uuid = back.layers[0]
        .smart_object
        .as_ref()
        .expect("resolved")
        .uuid
        .clone();
    assert!(section_has_uuid(&back.layer_section_extra, &uuid));

    // A duplicate shares the uuid, hence the one linked record.
    assert_eq!(duplicate_layer(&mut back, 0), 1);
    assert_eq!(back.layers[1].smart_object.as_ref().unwrap().uuid, uuid);

    // Converting one instance must leave the record for the sibling.
    assert!(convert_smart_object_to_layers(&mut back, "0"));
    assert!(
        section_has_uuid(&back.layer_section_extra, &uuid),
        "the record survives while the duplicate references it"
    );

    // Converting the last reference drops it.
    let remaining = back
        .layers
        .iter()
        .position(|layer| layer.smart_object.is_some())
        .expect("the duplicate is still a smart object");
    assert!(convert_smart_object_to_layers(
        &mut back,
        &remaining.to_string()
    ));
    assert!(
        !section_has_uuid(&back.layer_section_extra, &uuid),
        "the record is gone once no layer references it"
    );
}

#[test]
fn place_appends_topmost_channel_less_embedded_object() {
    let payload = write_psd(&solid_doc(2, 2, [10, 20, 30])).expect("source writes");
    let base = solid("Base", full(4, 4), (0, 0, 0), 255, BlendMode::Normal, 255);
    let mut d = doc(4, 4, vec![base.clone()]);

    let path = place_smart_object(&mut d, "source.psd", &payload).expect("places");

    assert_eq!(path, "1");
    assert_eq!(d.layers.len(), 2);
    let layer = &d.layers[1];
    assert_eq!(layer.name, "source.psd");
    assert_eq!(layer.rect, rect(0, 0, 2, 2));
    assert!(layer.channels.is_empty());
    assert!(layer.visible);
    assert_eq!(layer.blend, BlendMode::Normal);
    assert_eq!(layer.opacity, 255);
    assert_eq!(layer.fill, 255);
    let so = layer.smart_object.as_ref().expect("smart object");
    assert_eq!(so.kind, SmartObjectKind::Embedded);
    assert_eq!(so.payload.as_deref(), Some(payload.as_slice()));
    assert_eq!(so.filename, "source.psd");
    assert_eq!(so.filetype, *b"8BPB");
    assert_eq!(so.creator, *b"8BIM");
    assert_eq!(d.layers[0], base);
}

#[test]
fn placed_source_renders_inside_its_rect() {
    let payload = write_psd(&solid_doc(2, 2, [10, 20, 30])).expect("source writes");
    let base = solid("Base", full(4, 4), (0, 0, 0), 255, BlendMode::Normal, 255);
    let mut d = doc(4, 4, vec![base]);
    place_smart_object(&mut d, "source.psd", &payload).expect("places");

    let out = composite_rgba(&d);
    assert_close(&out, 0, 0, [10, 20, 30, 255]);
    assert_close(&out, 1, 1, [10, 20, 30, 255]);
    assert_close(&out, 2, 2, [0, 0, 0, 255]);
    assert_close(&out, 3, 3, [0, 0, 0, 255]);
}

#[test]
fn placed_source_larger_than_canvas_clips() {
    let payload = write_psd(&solid_doc(8, 8, [7, 8, 9])).expect("source writes");
    let base = solid("Base", full(4, 4), (0, 0, 0), 255, BlendMode::Normal, 255);
    let mut d = doc(4, 4, vec![base]);
    place_smart_object(&mut d, "big.psd", &payload).expect("places");

    let out = composite_rgba(&d);
    assert_eq!((out.width, out.height), (4, 4));
    for y in 0..4 {
        for x in 0..4 {
            assert_close(&out, x, y, [7, 8, 9, 255]);
        }
    }
}

#[test]
fn placed_layer_round_trips() {
    let payload = write_psd(&solid_doc(2, 2, [10, 20, 30])).expect("source writes");
    let base = solid("Base", full(4, 4), (0, 0, 0), 255, BlendMode::Normal, 255);
    let mut d = doc(4, 4, vec![base]);
    place_smart_object(&mut d, "source.psd", &payload).expect("places");
    let placed = d.layers[1]
        .smart_object
        .as_ref()
        .unwrap()
        .payload
        .clone()
        .unwrap();

    let bytes = write_psd(&d).expect("host writes");
    let back = read_psd(&bytes).expect("host reads");
    let so = back.layers[1].smart_object.as_ref().expect("resolved");
    assert_eq!(so.kind, SmartObjectKind::Embedded);
    assert_eq!(so.payload.as_deref(), Some(placed.as_slice()));
}

#[test]
fn place_refuses_malformed_source() {
    let base = solid("Base", full(4, 4), (0, 0, 0), 255, BlendMode::Normal, 255);
    let mut d = doc(4, 4, vec![base]);
    let before = d.clone();

    assert_eq!(place_smart_object(&mut d, "bad.psd", &[0, 1, 2, 3]), None);

    assert_eq!(d, before);
}

#[test]
fn replace_swaps_source_and_drops_preserved_blocks() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/test_with_smart_object01.psd");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipping: {} not found", path.display());
        return;
    };
    let mut d = read_psd(&bytes).expect("fixture parses");
    let index = (0..d.layers.len())
        .find(|&i| can_replace_smart_object_contents(&d, &i.to_string()))
        .expect("a replaceable smart object");
    let path = index.to_string();
    d.layers[index].mask = Some(LayerMask {
        rect: rect(0, 0, 2, 2),
        default_color: 0,
        disabled: true,
        flags: 0,
        data: Some(vec![0, 64, 128, 255].into()),
        extra: Vec::new(),
    });
    d.layers[index].color = ColorLabel::Violet;
    d.layers[index].lock = LockFlags::default()
        .with(LockFlags::TRANSPARENCY, true)
        .with(LockFlags::POSITION, true);
    let before = d.layers[index].clone();
    let old_uuid = before.smart_object.as_ref().unwrap().uuid.clone();
    assert!(!old_uuid.is_empty());
    assert!(section_has_uuid(&d.layer_section_extra, &old_uuid));
    assert!(before.extra_blocks.iter().any(|b| &b.key == b"SoLd"));
    assert!(before.smart_object.as_ref().unwrap().payload.is_some());

    let new_payload = payload_of([10, 20, 30]);
    let new_hash = hash(&new_payload);
    assert!(can_replace_smart_object_contents(&d, &path));
    assert!(replace_smart_object_contents(
        &mut d,
        &path,
        "replacement.psd",
        &new_payload
    ));

    let after = &d.layers[index];
    let so = after.smart_object.as_ref().expect("still smart");
    assert_eq!(so.payload.as_deref(), Some(new_payload.as_slice()));
    assert_eq!(hash(so.payload.as_deref().unwrap()), new_hash);
    read_psd(so.payload.as_deref().unwrap()).expect("payload parses");
    assert_eq!(so.filename, "replacement.psd");
    assert_eq!(so.filetype, *b"8BPB");
    assert_eq!(so.creator, *b"8BIM");
    assert!(so.uuid.is_empty());
    assert!(after.channels.is_empty());
    assert_eq!(after.name, before.name);
    assert_eq!(after.rect, before.rect);
    assert_eq!(after.blend, before.blend);
    assert_eq!(after.opacity, before.opacity);
    assert_eq!(after.fill, before.fill);
    assert_eq!(after.mask, before.mask);
    assert_eq!(after.color, before.color);
    assert_eq!(after.lock, before.lock);
    assert_eq!(
        after.mask.as_ref().and_then(|m| m.data.as_deref()),
        Some(&[0, 64, 128, 255][..])
    );
    assert!(!after
        .extra_blocks
        .iter()
        .any(|b| matches!(&b.key, b"SoLd" | b"SoLE" | b"plLd" | b"PlLd")));
    assert!(!section_has_uuid(&d.layer_section_extra, &old_uuid));

    let out = composite_rgba(&d);
    let cx = ((before.rect.left + before.rect.right) / 2).max(0) as u32;
    let cy = ((before.rect.top + before.rect.bottom) / 2).max(0) as u32;
    assert_close(&out, cx, cy, [10, 20, 30, 255]);

    let saved = write_psd(&d).expect("replaced document writes");
    let back = read_psd(&saved).expect("replaced document reads");
    let reread = back.layers[index].smart_object.as_ref().expect("resolves");
    assert_eq!(reread.kind, SmartObjectKind::Embedded);
    assert_eq!(hash(reread.payload.as_deref().unwrap()), new_hash);
}

#[test]
fn replace_refuses_ineligible_and_malformed_targets() {
    let valid = payload_of([1, 2, 3]);

    let mut plain = doc(
        4,
        4,
        vec![solid(
            "Raster",
            full(4, 4),
            (1, 2, 3),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let before = plain.clone();
    assert!(!can_replace_smart_object_contents(&plain, "0"));
    assert!(!replace_smart_object_contents(
        &mut plain, "0", "x.psd", &valid
    ));
    assert_eq!(plain, before);

    let mut grouped = doc(
        4,
        4,
        vec![{
            let mut g = group("G", BlendMode::Normal, 255, None, vec![]);
            g.smart_object = Some(embedded(valid.clone()));
            g
        }],
    );
    let before = grouped.clone();
    assert!(!replace_smart_object_contents(
        &mut grouped,
        "0",
        "x.psd",
        &valid
    ));
    assert_eq!(grouped, before);

    let mut adjustment = doc(
        4,
        4,
        vec![{
            let mut a = adjustment_layer("Adj", *b"inv ", vec![0], 255, None);
            a.smart_object = Some(embedded(valid.clone()));
            a
        }],
    );
    let before = adjustment.clone();
    assert!(!replace_smart_object_contents(
        &mut adjustment,
        "0",
        "x.psd",
        &valid
    ));
    assert_eq!(adjustment, before);

    let mut empty = doc(
        4,
        4,
        vec![smart_layer(
            "smart",
            full(4, 4),
            embedded(Vec::new()),
            Vec::new(),
        )],
    );
    let before = empty.clone();
    assert!(!can_replace_smart_object_contents(&empty, "0"));
    assert!(!replace_smart_object_contents(
        &mut empty, "0", "x.psd", &valid
    ));
    assert_eq!(empty, before);

    let mut smart = doc(
        4,
        4,
        vec![smart_layer(
            "smart",
            full(4, 4),
            embedded(valid.clone()),
            Vec::new(),
        )],
    );
    let before = smart.clone();
    assert!(can_replace_smart_object_contents(&smart, "0"));
    assert!(!replace_smart_object_contents(
        &mut smart,
        "0",
        "x.psd",
        &[0, 1, 2, 3]
    ));
    assert_eq!(smart, before);

    let before = smart.clone();
    assert!(!replace_smart_object_contents(
        &mut smart, "99", "x.psd", &valid
    ));
    assert!(!replace_smart_object_contents(
        &mut smart, "bad", "x.psd", &valid
    ));
    assert_eq!(smart, before);
}

#[test]
fn replace_clears_proxy_and_renders_new_source() {
    let mut d = doc(
        4,
        4,
        vec![solid(
            "Raster",
            full(4, 4),
            (10, 20, 30),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    assert!(convert_to_smart_object(&mut d, "0"));
    assert!(d.layers[0].channels.iter().any(|c| c.id == 0));
    let before_comp = composite_rgba(&d);
    assert!(can_replace_smart_object_contents(&d, "0"));

    let new_payload = payload_of([200, 100, 50]);
    assert!(replace_smart_object_contents(
        &mut d,
        "0",
        "new.psd",
        &new_payload
    ));

    assert!(d.layers[0].channels.is_empty());
    let so = d.layers[0].smart_object.as_ref().unwrap();
    assert_eq!(so.payload.as_deref(), Some(new_payload.as_slice()));
    assert_eq!(so.filename, "new.psd");
    assert!(so.uuid.is_empty());
    let after_comp = composite_rgba(&d);
    assert_ne!(after_comp.data, before_comp.data);
    assert_close(&after_comp, 0, 0, [200, 100, 50, 255]);
}

#[test]
fn open_as_smart_object_builds_one_embedded_layer() {
    let bytes = write_psd(&solid_doc(2, 2, [10, 20, 30])).expect("source writes");
    let source = read_psd(&bytes).expect("source reads");

    let d = open_as_smart_object("source", &bytes).expect("opens");

    assert_eq!((d.width, d.height), (source.width, source.height));
    assert_eq!(d.mode, source.mode);
    assert_eq!(d.depth, source.depth);
    assert_eq!(d.composite, source.composite);
    assert_eq!(d.layers.len(), 1);
    let layer = &d.layers[0];
    assert_eq!(layer.name, "source");
    assert!(layer.channels.is_empty());
    assert_eq!(layer.rect, rect(0, 0, 2, 2));
    let so = layer.smart_object.as_ref().expect("smart object");
    assert_eq!(so.kind, SmartObjectKind::Embedded);
    assert_eq!(so.payload.as_deref(), Some(bytes.as_slice()));
    assert_eq!(so.filename, "source");
    assert_eq!(so.filetype, *b"8BPB");
    assert_eq!(so.creator, *b"8BIM");

    let out = composite_rgba(&d);
    assert_close(&out, 0, 0, [10, 20, 30, 255]);
    assert_close(&out, 1, 1, [10, 20, 30, 255]);
}

#[test]
fn open_as_smart_object_refuses_malformed_source() {
    assert!(open_as_smart_object("bad", &[0, 1, 2, 3]).is_none());
}

#[test]
fn source_bytes_returns_placed_payload_unchanged() {
    let payload = write_psd(&solid_doc(2, 2, [10, 20, 30])).expect("source writes");
    let base = solid("Base", full(4, 4), (0, 0, 0), 255, BlendMode::Normal, 255);
    let mut d = doc(4, 4, vec![base]);
    let path = place_smart_object(&mut d, "source.psd", &payload).expect("places");
    let before = d.clone();

    assert_eq!(smart_object_source_bytes(&d, &path), Some(payload));
    assert_eq!(d, before);
}

#[test]
fn source_bytes_refuses_ineligible_targets() {
    let valid = payload_of([1, 2, 3]);

    let plain = doc(
        4,
        4,
        vec![solid(
            "Raster",
            full(4, 4),
            (1, 2, 3),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    assert_eq!(smart_object_source_bytes(&plain, "0"), None);

    let grouped = doc(
        4,
        4,
        vec![{
            let mut g = group("G", BlendMode::Normal, 255, None, vec![]);
            g.smart_object = Some(embedded(valid.clone()));
            g
        }],
    );
    assert_eq!(smart_object_source_bytes(&grouped, "0"), None);

    let adjustment = doc(
        4,
        4,
        vec![{
            let mut a = adjustment_layer("Adj", *b"inv ", vec![0], 255, None);
            a.smart_object = Some(embedded(valid.clone()));
            a
        }],
    );
    assert_eq!(smart_object_source_bytes(&adjustment, "0"), None);

    let empty = doc(
        4,
        4,
        vec![smart_layer(
            "smart",
            full(4, 4),
            embedded(Vec::new()),
            Vec::new(),
        )],
    );
    assert_eq!(smart_object_source_bytes(&empty, "0"), None);

    assert_eq!(smart_object_source_bytes(&plain, "99"), None);
    assert_eq!(smart_object_source_bytes(&plain, "bad"), None);
}

#[test]
fn apply_pictura_raw_bakes_the_proxy_and_keeps_the_settings() {
    let layer = solid(
        "Raster",
        full(4, 4),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    let mut d = doc(4, 4, vec![layer]);
    assert!(convert_to_smart_object(&mut d, "0"));
    let before_channels = d.layers[0].channels.clone();

    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(1.0),
            temperature: Some(40.0),
            clarity: Some(-30.0),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(apply_pictura_raw(&mut d, "0", &settings));

    let after_channels = &d.layers[0].channels;
    assert_ne!(after_channels, &before_channels, "the proxy pixels change");
    let alpha = |channels: &[Channel]| {
        channels
            .iter()
            .find(|c| c.id == -1)
            .expect("alpha channel")
            .data
            .clone()
    };
    assert_eq!(alpha(after_channels), alpha(&before_channels), "alpha kept");

    let filter = &d.layers[0].smart_object.as_ref().unwrap().smart_filters[0];
    assert_eq!(filter.filter_id, 2683);
    assert_eq!(decode_pictura_raw_settings(&filter.options), settings);

    let back = read_psd(&write_psd(&d).expect("writes")).expect("re-reads");
    let filter = &back.layers[0].smart_object.as_ref().unwrap().smart_filters[0];
    assert_eq!(filter.filter_id, 2683);
    assert_eq!(decode_pictura_raw_settings(&filter.options), settings);
}

#[test]
fn apply_pictura_raw_refuses_without_mutation() {
    let layer = solid(
        "Raster",
        full(4, 4),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let mut d = doc(4, 4, vec![layer]);
    let before = d.clone();
    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(2.0),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(
        !apply_pictura_raw(&mut d, "0", &settings),
        "a raster layer is refused"
    );
    assert!(
        !apply_pictura_raw(&mut d, "nope", &settings),
        "a missing path is refused"
    );
    assert_eq!(d, before);
}

#[test]
fn smart_filter_chain_composites_the_filtered_source() {
    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(1.0),
            ..Default::default()
        },
        ..Default::default()
    };
    let plain = smart_layer(
        "smart",
        full(4, 4),
        embedded(payload_of([200, 100, 50])),
        Vec::new(),
    );
    let mut filtered = plain.clone();
    pictura_codec::attach_pictura_raw_filter(&mut filtered, &settings).expect("filter attaches");
    let so = filtered.smart_object.clone().unwrap();
    assert_eq!(so.smart_filters.len(), 1);

    let unfiltered = composite_rgba(&doc(4, 4, vec![plain]));
    let out = composite_rgba(&doc(4, 4, vec![filtered]));
    assert_ne!(out.data, unfiltered.data, "the chain changes the composite");

    // The chain runs on the freshly rendered source, so the composite equals a
    // direct camera-raw render of that source (over the transparent backdrop).
    let src = render_smart_source(&so, full(4, 4), full(4, 4)).expect("source renders");
    let expected = pictura_adjust::render_pictura_raw(&src, &settings).expect("raw renders");
    assert_eq!(out.data, expected.data, "composite matches a source render");
}

#[test]
fn disabled_smart_filter_matches_no_filter() {
    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(2.0),
            ..Default::default()
        },
        ..Default::default()
    };
    let plain = smart_layer(
        "smart",
        full(4, 4),
        embedded(payload_of([10, 20, 30])),
        Vec::new(),
    );
    let mut disabled = plain.clone();
    pictura_codec::attach_pictura_raw_filter(&mut disabled, &settings).expect("filter attaches");
    disabled.smart_object.as_mut().unwrap().smart_filters[0].enabled = false;

    // Composite equality alone cannot distinguish "the chain skipped a disabled
    // filter" from "there is no chain": both fall back to the unfiltered source.
    // Assert the typed and render outcomes explicitly.
    assert!(
        plain
            .smart_object
            .as_ref()
            .unwrap()
            .smart_filters
            .is_empty(),
        "the plain layer carries no filter"
    );
    let so = disabled.smart_object.as_ref().unwrap();
    assert_eq!(
        so.smart_filters.len(),
        1,
        "the disabled layer still carries one"
    );
    assert!(!so.smart_filters[0].enabled);

    let a = composite_rgba(&doc(4, 4, vec![plain]));
    let b = composite_rgba(&doc(4, 4, vec![disabled.clone()]));
    assert_eq!(
        a.data, b.data,
        "a disabled filter is byte-identical to none"
    );

    // The chain itself is what skips the filter: with the group enabled it
    // returns `base` unchanged for the disabled filter and changes the pixels
    // once the filter is enabled.
    let base = render_smart_source(so, full(4, 4), full(4, 4)).expect("source renders");
    let mut enabled = so.smart_filters.clone();
    enabled[0].enabled = true;
    let skipped = apply_smart_filter_chain(&base, full(4, 4), &so.smart_filters, None, true);
    let applied = apply_smart_filter_chain(&base, full(4, 4), &enabled, None, true);
    assert_eq!(skipped.data, base.data, "the disabled filter is skipped");
    assert_ne!(
        applied.data, base.data,
        "an enabled filter changes the pixels"
    );
}

#[test]
fn smart_filter_chain_ignores_a_baked_proxy() {
    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(1.0),
            clarity: Some(20.0),
            ..Default::default()
        },
        ..Default::default()
    };
    let layer = solid(
        "Raster",
        full(4, 4),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    let mut d = doc(4, 4, vec![layer]);
    assert!(convert_to_smart_object(&mut d, "0"));
    assert!(apply_pictura_raw(&mut d, "0", &settings));

    let out = composite_rgba(&d);
    let so = d.layers[0].smart_object.as_ref().unwrap();
    let src = render_smart_source(so, full(4, 4), full(4, 4)).unwrap();
    let expected = pictura_adjust::render_pictura_raw(&src, &settings).unwrap();
    assert_eq!(
        out.data, expected.data,
        "the already-baked proxy is not filtered a second time"
    );
}
