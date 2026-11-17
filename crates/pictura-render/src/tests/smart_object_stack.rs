use super::smart_object::{assert_close, embedded, payload_of, smart_layer};
use super::*;
use pictura_codec::write_psd;
use pictura_core::{Channel, CrsSettings, PicturaRawSettings, SmartObject};

// ---------------------------------------------------------------------------
// Stack modes
// ---------------------------------------------------------------------------

fn stack_payload(frames: &[((u8, u8, u8), bool)], size: u32) -> Vec<u8> {
    let mut d = Document::new(size, size, ColorMode::Rgb, BitDepth::Eight);
    let layers = frames
        .iter()
        .map(|(rgb, visible)| {
            let mut layer = solid("frame", full(size, size), *rgb, 255, BlendMode::Normal, 255);
            layer.visible = *visible;
            layer
        })
        .collect();
    d.layers = pictura_core::LayerStack::from_layers(layers);
    d.composite
        .replace(composite_rgba(&d))
        .expect("valid composite");
    write_psd(&d).expect("stack payload writes")
}

fn stack_object(payload: Vec<u8>, mode: Option<pictura_core::StackMode>) -> SmartObject {
    let mut so = embedded(payload);
    so.stack_mode = mode;
    so
}

fn stack_doc(
    payload: Vec<u8>,
    mode: Option<pictura_core::StackMode>,
    proxy: Vec<Channel>,
    size: u32,
) -> Document {
    let layer = smart_layer(
        "stack",
        full(size, size),
        stack_object(payload, mode),
        proxy,
    );
    doc(size, size, vec![layer])
}

fn proxy_rgb(rgb: (u8, u8, u8), size: u32) -> Vec<Channel> {
    solid("proxy", full(size, size), rgb, 255, BlendMode::Normal, 255).channels().clone()
}

#[test]
fn stack_mode_renders_the_per_pixel_median() {
    let payload = stack_payload(
        &[
            ((0, 0, 0), true),
            ((10, 10, 10), true),
            ((100, 100, 100), true),
        ],
        2,
    );
    let d = stack_doc(
        payload,
        Some(pictura_core::StackMode::Median),
        Vec::new(),
        2,
    );
    let out = composite_rgba(&d);
    assert_close(&out, 0, 0, [10, 10, 10, 255]);
    assert_close(&out, 1, 1, [10, 10, 10, 255]);
}

#[test]
fn stack_mode_overrides_the_proxy() {
    let payload = stack_payload(&[((0, 0, 0), true), ((255, 255, 255), true)], 2);
    let d = stack_doc(
        payload,
        Some(pictura_core::StackMode::Median),
        proxy_rgb((255, 0, 0), 2),
        2,
    );
    assert_close(&composite_rgba(&d), 0, 0, [128, 128, 128, 255]);
}

#[test]
fn clearing_a_stack_mode_restores_the_proxy() {
    let payload = stack_payload(&[((0, 255, 0), true), ((0, 255, 0), true)], 2);
    let mut d = stack_doc(
        payload,
        Some(pictura_core::StackMode::Maximum),
        proxy_rgb((255, 0, 0), 2),
        2,
    );
    assert_close(&composite_rgba(&d), 0, 0, [0, 255, 0, 255]);
    pictura_codec::edit_layer(d.layers[0].as_mut(), |parts| { parts.smart_object.as_mut()
        .expect("smart object")
        .stack_mode = None; });
    assert_close(&composite_rgba(&d), 0, 0, [255, 0, 0, 255]);
}

#[test]
fn stack_mode_applies_smart_filters_after_reduction() {
    let payload = stack_payload(
        &[
            ((10, 20, 30), true),
            ((40, 50, 60), true),
            ((70, 80, 90), true),
        ],
        2,
    );
    let mut d = stack_doc(
        payload,
        Some(pictura_core::StackMode::Median),
        proxy_rgb((255, 0, 0), 2),
        2,
    );
    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(1.0),
            clarity: Some(20.0),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(apply_pictura_raw(&mut d, "0", &settings));
    let so = d.layers[0].smart_object().expect("object");
    let src = render_smart_source(so, full(2, 2), full(2, 2)).expect("source");
    let expected = pictura_adjust::render_pictura_raw(&src, &settings).expect("filter");
    for channel in d.layers[0].channels_mut() {
        channel.data = vec![7u8; 4].into();
    }
    let out = composite_rgba(&d);
    assert_eq!(
        out.data.as_slice(),
        expected.data.as_slice(),
        "the filter applies once to the reduced source"
    );
}

#[test]
fn stack_mode_unknown_filter_keeps_the_proxy() {
    let payload = stack_payload(&[((0, 255, 0), true), ((0, 255, 0), true)], 2);
    let mut d = stack_doc(
        payload,
        Some(pictura_core::StackMode::Maximum),
        proxy_rgb((255, 0, 0), 2),
        2,
    );
    pictura_codec::edit_layer(d.layers[0].as_mut(), |parts| { parts.smart_object.as_mut()
        .expect("object")
        .smart_filters = vec![pictura_core::SmartFilter {
        filter_id: 999,
        name: "unknown".into(),
        enabled: true,
        options: Vec::new().into(),
    }]; });
    assert_close(&composite_rgba(&d), 0, 0, [255, 0, 0, 255]);
}

#[test]
fn set_stack_mode_sets_and_clears() {
    let payload = stack_payload(&[((0, 0, 0), true), ((10, 10, 10), true)], 2);
    let mut d = stack_doc(payload, None, Vec::new(), 2);
    assert!(set_smart_object_stack_mode(
        &mut d,
        "0",
        Some(pictura_core::StackMode::Median)
    ));
    assert_eq!(
        smart_object_stack_mode(&d, "0"),
        Some(pictura_core::StackMode::Median)
    );
    assert!(
        set_smart_object_stack_mode(&mut d, "0", Some(pictura_core::StackMode::Median)),
        "setting the active mode is a successful no-op"
    );
    assert!(set_smart_object_stack_mode(&mut d, "0", None));
    assert_eq!(smart_object_stack_mode(&d, "0"), None);
}

#[test]
fn set_stack_mode_refuses_fewer_than_two_frames() {
    let payload = stack_payload(&[((0, 0, 0), true)], 2);
    let mut d = stack_doc(payload, None, Vec::new(), 2);
    assert!(!set_smart_object_stack_mode(
        &mut d,
        "0",
        Some(pictura_core::StackMode::Mean)
    ));
    assert_eq!(smart_object_stack_mode(&d, "0"), None);
}

#[test]
fn set_stack_mode_refuses_ineligible_targets() {
    let payload = stack_payload(&[((0, 0, 0), true), ((10, 10, 10), true)], 2);

    let group = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        is_group: true,
        smart_object: Some(stack_object(payload.clone(), None)),
        ..Default::default()
    });
    let mut group_doc = doc(2, 2, vec![group]);
    assert!(!set_smart_object_stack_mode(
        &mut group_doc,
        "0",
        Some(pictura_core::StackMode::Mean)
    ));

    let adjustment = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        adjustment: Some(pictura_core::AdjustmentData {
            key: *b"nvrt",
            data: Vec::new().into(),
        }),
        smart_object: Some(stack_object(payload.clone(), None)),
        ..Default::default()
    });
    let mut adjustment_doc = doc(2, 2, vec![adjustment]);
    assert!(!set_smart_object_stack_mode(
        &mut adjustment_doc,
        "0",
        Some(pictura_core::StackMode::Mean)
    ));

    let mut plain_doc = doc(2, 2, vec![pictura_core::Layer::default()]);
    assert!(!set_smart_object_stack_mode(
        &mut plain_doc,
        "0",
        Some(pictura_core::StackMode::Mean)
    ));

    let mut unreadable = stack_doc(vec![0, 1, 2], None, Vec::new(), 2);
    assert!(!set_smart_object_stack_mode(
        &mut unreadable,
        "0",
        Some(pictura_core::StackMode::Mean)
    ));
    assert!(!set_smart_object_stack_mode(
        &mut unreadable,
        "99",
        Some(pictura_core::StackMode::Mean)
    ));
}

#[test]
fn stack_frames_render_a_nested_smart_object() {
    let nested = stack_object(payload_of([5, 6, 7]), None);
    let mut payload_doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    payload_doc.layers = pictura_core::LayerStack::from_layers(vec![
        smart_layer("nested", full(2, 2), nested, Vec::new()),
        solid("solid", full(2, 2), (0, 0, 10), 255, BlendMode::Normal, 255),
    ]);
    payload_doc
        .composite
        .replace(composite_rgba(&payload_doc))
        .expect("valid composite");
    let payload = write_psd(&payload_doc).expect("payload writes");
    let d = stack_doc(
        payload,
        Some(pictura_core::StackMode::Maximum),
        Vec::new(),
        2,
    );
    assert_close(&composite_rgba(&d), 0, 0, [5, 6, 10, 255]);
}

#[test]
fn stack_frames_are_visible_top_level_layers() {
    let mut payload_doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    let mut hidden = solid(
        "hidden",
        full(2, 2),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    hidden.visible = false;
    let bottom = solid(
        "bottom",
        full(2, 2),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    let top = solid("top", full(2, 2), (0, 0, 255), 255, BlendMode::Normal, 255);
    let group = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        name: "group".into(),
        rect: full(2, 2),
        is_group: true,
        children: pictura_core::LayerStack::from_layers(vec![bottom, top]),
        ..Default::default()
    });
    let raster = solid(
        "raster",
        full(2, 2),
        (10, 10, 10),
        255,
        BlendMode::Normal,
        255,
    );
    payload_doc.layers = pictura_core::LayerStack::from_layers(vec![hidden, group, raster]);
    let payload = write_psd(&payload_doc).expect("payload writes");
    let d = stack_doc(
        payload,
        Some(pictura_core::StackMode::Maximum),
        Vec::new(),
        2,
    );
    assert_close(&composite_rgba(&d), 0, 0, [10, 10, 255, 255]);
}

#[test]
fn stack_mode_mean_then_maximum_is_not_cumulative() {
    let payload = stack_payload(
        &[((0, 0, 0), true), ((0, 0, 0), true), ((3, 3, 3), true)],
        2,
    );
    let mut d = stack_doc(
        payload.clone(),
        Some(pictura_core::StackMode::Mean),
        Vec::new(),
        2,
    );
    let mean = composite_rgba(&d);
    assert!(set_smart_object_stack_mode(
        &mut d,
        "0",
        Some(pictura_core::StackMode::Maximum)
    ));
    let after = composite_rgba(&d);
    let fresh = stack_doc(
        payload,
        Some(pictura_core::StackMode::Maximum),
        Vec::new(),
        2,
    );
    let expected = composite_rgba(&fresh);
    assert_eq!(after.data.as_slice(), expected.data.as_slice());
    assert_ne!(mean.data.as_slice(), after.data.as_slice());
}

#[test]
fn stack_mode_falls_back_for_an_unusable_stack() {
    let unreadable = stack_doc(
        vec![0, 1, 2],
        Some(pictura_core::StackMode::Median),
        proxy_rgb((255, 0, 0), 2),
        2,
    );
    assert_close(&composite_rgba(&unreadable), 0, 0, [255, 0, 0, 255]);

    let one = stack_doc(
        stack_payload(&[((0, 255, 0), true)], 2),
        Some(pictura_core::StackMode::Median),
        proxy_rgb((255, 0, 0), 2),
        2,
    );
    assert_close(&composite_rgba(&one), 0, 0, [255, 0, 0, 255]);

    let one_no_proxy = stack_doc(
        stack_payload(&[((0, 255, 0), true)], 2),
        Some(pictura_core::StackMode::Median),
        Vec::new(),
        2,
    );
    assert_close(&composite_rgba(&one_no_proxy), 0, 0, [0, 255, 0, 255]);
}

#[test]
fn rasterize_keeps_the_proxy_when_the_stack_does_not_apply() {
    let proxy = proxy_rgb((255, 0, 0), 2);
    let mut one = stack_doc(
        stack_payload(&[((0, 255, 0), true)], 2),
        Some(pictura_core::StackMode::Median),
        proxy.clone(),
        2,
    );
    assert!(rasterize_smart_object(&mut one, "0"));
    assert_eq!(one.layers[0].channels(), proxy);
    assert!(one.layers[0].smart_object().is_none());

    let proxy = proxy_rgb((255, 0, 0), 2);
    let mut unreadable = stack_doc(
        vec![0, 1, 2],
        Some(pictura_core::StackMode::Median),
        proxy.clone(),
        2,
    );
    assert!(rasterize_smart_object(&mut unreadable, "0"));
    assert_eq!(unreadable.layers[0].channels(), proxy);
    assert!(unreadable.layers[0].smart_object().is_none());
}

#[test]
fn rasterize_bakes_the_stack_render() {
    let payload = stack_payload(&[((200, 50, 0), true), ((0, 0, 10), true)], 2);
    let mut d = stack_doc(
        payload,
        Some(pictura_core::StackMode::Maximum),
        proxy_rgb((255, 0, 0), 2),
        2,
    );
    assert!(rasterize_smart_object(&mut d, "0"));
    let layer = &d.layers[0];
    assert!(layer.smart_object().is_none());
    let channel = |id: i16| {
        layer
            .channels()
            .iter()
            .find(|channel| channel.id == id)
            .expect("channel")
    };
    assert_eq!(channel(0).data.as_slice(), &[200u8; 4]);
    assert_eq!(channel(1).data.as_slice(), &[50u8; 4]);
    assert_eq!(channel(2).data.as_slice(), &[10u8; 4]);
    assert_eq!(channel(-1).data.as_slice(), &[255u8; 4]);
}
