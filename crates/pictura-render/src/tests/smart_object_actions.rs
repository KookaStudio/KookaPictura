use super::smart_object::{embedded, green_proxy, payload_2x2, payload_of, smart_layer};
use super::*;

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
