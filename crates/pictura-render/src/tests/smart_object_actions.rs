use super::smart_object::{embedded, green_proxy, payload_2x2, payload_of, smart_layer};
use super::*;
use pictura_codec::write_psd;

// --- Reset Transform / Convert to Layers / New Smart Object via Copy ---

#[test]
fn reset_restores_native_placement_and_keeps_the_object() {
    let layer = smart_layer(
        "smart",
        rect(10, 10, 14, 14),
        embedded(payload_2x2([
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [255, 255, 255],
        ])),
        green_proxy(),
    );
    let mut d = doc(20, 20, vec![layer]);
    assert!(can_reset_smart_object_transform(&d, "0"));
    let so_before = d.layers[0].smart_object.clone();

    assert!(reset_smart_object_transform(&mut d, "0"));
    assert_eq!(d.layers[0].rect, rect(0, 0, 2, 2));
    assert!(d.layers[0].channels.is_empty());
    assert_eq!(d.layers[0].smart_object, so_before);
}

#[test]
fn reset_refuses_ineligible_and_unparseable() {
    let mut plain = doc(
        4,
        4,
        vec![solid(
            "R",
            full(4, 4),
            (1, 2, 3),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let before = plain.clone();
    assert!(!can_reset_smart_object_transform(&plain, "0"));
    assert!(!reset_smart_object_transform(&mut plain, "0"));
    assert_eq!(plain, before);

    let mut undecodable = doc(
        4,
        4,
        vec![smart_layer(
            "s",
            full(4, 4),
            embedded(vec![0, 1, 2]),
            Vec::new(),
        )],
    );
    let before = undecodable.clone();
    assert!(!can_reset_smart_object_transform(&undecodable, "0"));
    assert!(!reset_smart_object_transform(&mut undecodable, "0"));
    assert_eq!(undecodable, before);
}

#[test]
fn convert_to_layers_yields_source_at_the_object_position() {
    let mut d = doc(
        12,
        12,
        vec![solid(
            "L",
            rect(5, 7, 8, 9),
            (10, 20, 30),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    assert!(convert_to_smart_object(&mut d, "0"));
    let proxy = d.layers[0].channels.clone();
    assert!(can_convert_smart_object_to_layers(&d, "0"));

    assert!(convert_smart_object_to_layers(&mut d, "0"));
    assert_eq!(d.layers.len(), 1);
    assert_eq!(d.layers[0].rect, rect(5, 7, 8, 9));
    assert!(d.layers[0].smart_object.is_none());
    assert_eq!(d.layers[0].channels, proxy);
}

#[test]
fn convert_to_layers_scales_the_source_stack() {
    let mut d = doc(
        20,
        20,
        vec![solid(
            "L",
            full(4, 4),
            (10, 20, 30),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    assert!(convert_to_smart_object(&mut d, "0"));
    assert!(transform_layer(
        &mut d,
        "0",
        LayerTransform {
            scale_x: 2.0,
            scale_y: 2.0,
            angle_radians: 0.0,
            dx: 0.0,
            dy: 0.0,
        }
    ));
    let scaled_rect = d.layers[0].rect;
    assert_eq!(scaled_rect, rect(-2, -2, 6, 6));

    assert!(convert_smart_object_to_layers(&mut d, "0"));
    assert_eq!(d.layers.len(), 1);
    assert_eq!(d.layers[0].rect, scaled_rect);
    let channel = d.layers[0].channels.iter().find(|c| c.id == 0).unwrap();
    assert_eq!(channel.data.len(), 8 * 8);
}

#[test]
fn new_smart_object_via_copy_is_independent() {
    let mut d = doc(
        8,
        8,
        vec![solid(
            "L",
            full(4, 4),
            (10, 20, 30),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    assert!(convert_to_smart_object(&mut d, "0"));
    let original = d.layers[0]
        .smart_object
        .as_ref()
        .unwrap()
        .payload()
        .map(<[u8]>::to_vec)
        .unwrap();

    let path = new_smart_object_via_copy(&mut d, "0").expect("copy");
    assert_eq!(path, "1");
    assert_eq!(d.layers.len(), 2);
    assert_eq!(d.layers[1].name, "L copy");
    assert_eq!(
        d.layers[1].smart_object.as_ref().unwrap().payload(),
        Some(original.as_slice())
    );
    assert!(d.layers[1].smart_object.as_ref().unwrap().uuid.is_empty());
    assert_eq!(
        d.layers[0].smart_object.as_ref().unwrap().payload(),
        Some(original.as_slice())
    );

    // Editing the copy's source leaves the original's untouched.
    let replacement = payload_of([9, 9, 9]);
    assert!(replace_smart_object_contents(
        &mut d,
        "1",
        "new.psd",
        &replacement
    ));
    assert_eq!(
        d.layers[0].smart_object.as_ref().unwrap().payload(),
        Some(original.as_slice())
    );
    assert_eq!(
        d.layers[1].smart_object.as_ref().unwrap().payload(),
        Some(replacement.as_slice())
    );
}

#[test]
fn new_smart_object_via_copy_refuses_non_smart() {
    let mut d = doc(
        4,
        4,
        vec![solid(
            "R",
            full(4, 4),
            (1, 2, 3),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let before = d.clone();
    assert!(!can_new_smart_object_via_copy(&d, "0"));
    assert_eq!(new_smart_object_via_copy(&mut d, "0"), None);
    assert_eq!(d, before);
}

// --- Canvas fit and non-destructive smart-rect transform ---

fn placed_source(width: u32, height: u32) -> Vec<u8> {
    let source = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
    write_psd(&source).expect("source writes")
}

#[test]
fn fit_smart_object_scales_an_oversized_source() {
    let payload = placed_source(40, 20);
    let mut d = doc(
        10,
        10,
        vec![solid(
            "Base",
            full(10, 10),
            (0, 0, 0),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let path = place_smart_object(&mut d, "source.psd", &payload).expect("places");
    let object = d.layers[1].smart_object.clone();

    assert!(fit_smart_object_to_canvas(&mut d, &path));

    // 40 × 20 into 10 × 10: scale 0.25, fitted 10 × 5, centred at y = 2.
    assert_eq!(d.layers[1].rect, rect(2, 0, 7, 10));
    assert!(d.layers[1].channels.is_empty());
    assert_eq!(d.layers[1].smart_object, object);
    assert_eq!(
        d.layers[1].smart_object.as_ref().unwrap().payload(),
        Some(payload.as_slice())
    );
}

#[test]
fn fit_smart_object_centres_a_portrait_source() {
    let payload = placed_source(20, 40);
    let mut d = doc(
        10,
        10,
        vec![solid(
            "Base",
            full(10, 10),
            (0, 0, 0),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let path = place_smart_object(&mut d, "source.psd", &payload).expect("places");

    assert!(fit_smart_object_to_canvas(&mut d, &path));

    // 20 × 40 into 10 × 10: scale 0.25, fitted 5 × 10, centred at x = 2.
    assert_eq!(d.layers[1].rect, rect(0, 2, 10, 7));
}

#[test]
fn fit_smart_object_does_not_upscale() {
    let payload = placed_source(4, 3);
    let mut d = doc(
        10,
        10,
        vec![solid(
            "Base",
            full(10, 10),
            (0, 0, 0),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let path = place_smart_object(&mut d, "source.psd", &payload).expect("places");

    assert!(fit_smart_object_to_canvas(&mut d, &path));

    assert_eq!(d.layers[1].rect, rect(0, 0, 3, 4));
    assert_eq!(
        d.layers[1].smart_object.as_ref().unwrap().payload(),
        Some(payload.as_slice())
    );
}

#[test]
fn transform_smart_object_rect_keeps_the_source() {
    let payload = placed_source(8, 8);
    let mut d = doc(
        4,
        4,
        vec![solid(
            "Base",
            full(4, 4),
            (0, 0, 0),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let path = place_smart_object(&mut d, "source.psd", &payload).expect("places");
    assert!(fit_smart_object_to_canvas(&mut d, &path));
    let object = d.layers[1].smart_object.clone();

    assert!(transform_smart_object_rect(&mut d, &path, rect(0, 0, 8, 8)));

    assert_eq!(d.layers[1].rect, rect(0, 0, 8, 8));
    assert!(d.layers[1].channels.is_empty());
    assert_eq!(d.layers[1].smart_object, object);
    assert_eq!(
        d.layers[1].smart_object.as_ref().unwrap().payload(),
        Some(payload.as_slice())
    );
}

#[test]
fn transform_smart_object_rect_refuses_ineligible_targets() {
    let mut plain = doc(
        4,
        4,
        vec![solid(
            "R",
            full(4, 4),
            (1, 2, 3),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let before = plain.clone();
    assert!(!can_transform_smart_object_rect(&plain, "0"));
    assert!(!transform_smart_object_rect(
        &mut plain,
        "0",
        rect(0, 0, 2, 2)
    ));
    assert_eq!(plain, before);

    let payload = placed_source(4, 4);
    let mut locked = doc(
        4,
        4,
        vec![smart_layer(
            "s",
            rect(0, 0, 4, 4),
            embedded(payload),
            Vec::new(),
        )],
    );
    locked.layers[0].lock = LockFlags::default().with(LockFlags::POSITION, true);
    let before = locked.clone();
    assert!(!can_transform_smart_object_rect(&locked, "0"));
    assert!(!transform_smart_object_rect(
        &mut locked,
        "0",
        rect(0, 0, 2, 2)
    ));
    assert_eq!(locked, before);
}

#[test]
fn convert_to_smart_object_keeps_merged_transparency() {
    let alpha = [255u8, 0, 0, 255];
    let mut d = doc(
        2,
        2,
        vec![Layer {
            name: "R".into(),
            rect: full(2, 2),
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![10; 4].into(),
                },
                Channel {
                    id: 1,
                    data: vec![20; 4].into(),
                },
                Channel {
                    id: 2,
                    data: vec![30; 4].into(),
                },
                Channel {
                    id: -1,
                    data: alpha.to_vec().into(),
                },
            ],
            ..Default::default()
        }],
    );

    assert!(convert_to_smart_object(&mut d, "0"));
    let payload = smart_object_source_bytes(&d, "0").expect("embedded source");
    let embedded = pictura_codec::read_psd(&payload).expect("embedded source parses");
    assert_eq!(
        embedded.channels.first().map(|channel| &channel.data[..]),
        Some(&alpha[..])
    );

    let object = d.layers[0].smart_object.as_ref().expect("smart object");
    let rendered = crate::render_smart_source(object, d.layers[0].rect, d.layers[0].rect)
        .expect("embedded source renders");
    assert_eq!(&rendered.data[12..16], &alpha);
}

#[test]
fn transformed_channel_less_smart_object_round_trips_its_rect() {
    let payload = placed_source(8, 8);
    let mut d = doc(
        4,
        4,
        vec![solid(
            "Base",
            full(4, 4),
            (0, 0, 0),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let path = place_smart_object(&mut d, "source.psd", &payload).expect("places");
    assert!(fit_smart_object_to_canvas(&mut d, &path));
    assert!(transform_smart_object_rect(&mut d, &path, rect(0, 0, 6, 6)));

    let saved = write_psd(&d).expect("host writes");
    let back = pictura_codec::read_psd(&saved).expect("host reads");
    let layer = back
        .layers
        .iter()
        .find(|layer| layer.smart_object.is_some())
        .expect("placed layer round-trips");
    assert_eq!(layer.rect, rect(0, 0, 6, 6));
    assert_eq!(
        layer.smart_object.as_ref().unwrap().payload(),
        Some(payload.as_slice())
    );
}

#[test]
fn transform_smart_object_rect_refuses_stray_channels() {
    let payload = placed_source(4, 4);
    let mut alpha = doc(
        4,
        4,
        vec![smart_layer(
            "s",
            rect(0, 0, 4, 4),
            embedded(payload.clone()),
            vec![Channel {
                id: -1,
                data: vec![255; 16].into(),
            }],
        )],
    );
    let before = alpha.clone();
    assert!(!can_transform_smart_object_rect(&alpha, "0"));
    assert!(!transform_smart_object_rect(
        &mut alpha,
        "0",
        rect(0, 0, 2, 2)
    ));
    assert_eq!(alpha, before);

    let mut raw = doc(
        4,
        4,
        vec![smart_layer(
            "s",
            rect(0, 0, 4, 4),
            embedded(payload),
            Vec::new(),
        )],
    );
    raw.layers[0].raw_channels = vec![pictura_core::RawChannel {
        id: 3,
        data: vec![0, 0, 0, 0].into(),
    }];
    let before = raw.clone();
    assert!(!can_transform_smart_object_rect(&raw, "0"));
    assert!(!transform_smart_object_rect(
        &mut raw,
        "0",
        rect(0, 0, 2, 2)
    ));
    assert_eq!(raw, before);
}
