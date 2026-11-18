use std::sync::Arc;

use super::tests::{centre, locked, nested_payload};
use super::*;
use crate::tests::{doc, rect, solid};
use pictura_core::{Layer, PixelBuffer, SmartObject, SmartObjectKind};

// ---------------------------------------------------------------------------
// Stack modes
// ---------------------------------------------------------------------------

fn stack_payload_two() -> Vec<u8> {
    let mut d = doc(
        4,
        4,
        vec![
            solid(
                "a",
                rect(0, 0, 4, 4),
                (10, 20, 30),
                255,
                pictura_core::BlendMode::Normal,
                255,
            ),
            solid(
                "b",
                rect(0, 0, 4, 4),
                (100, 120, 140),
                255,
                pictura_core::BlendMode::Normal,
                255,
            ),
        ],
    );
    d.composite
        .replace(crate::composite_rgba(&d))
        .expect("valid composite");
    pictura_codec::write_psd(&d).expect("stack payload")
}

fn stack_layer_at(
    payload: Vec<u8>,
    mode: Option<pictura_core::StackMode>,
    frame: pictura_core::PsdRect,
) -> Layer {
    let mut so = SmartObject::default();
    so.kind = SmartObjectKind::Embedded;
    so.stack_mode = mode;
    so.set_payload(payload);
    pictura_codec::layer_from_parts(pictura_core::LayerParts {
        name: "stack".into(),
        rect: frame,
        smart_object: Some(so),
        ..Default::default()
    })
}

#[test]
fn stack_mode_change_invalidates_the_cached_source() {
    let _guard = locked();
    let mut layer = stack_layer_at(
        stack_payload_two(),
        Some(pictura_core::StackMode::Median),
        rect(0, 0, 4, 4),
    );
    let first = embedded_source(layer.smart_object().expect("object")).expect("first");
    let second = embedded_source(layer.smart_object().expect("object")).expect("second");
    assert!(Arc::ptr_eq(&first, &second));

    pictura_codec::edit_layer(layer.as_mut(), |parts| {
        parts.smart_object.as_mut().expect("object").stack_mode =
            Some(pictura_core::StackMode::Maximum)
    });
    let third = embedded_source(layer.smart_object().expect("object")).expect("third");
    assert!(!Arc::ptr_eq(&first, &third));

    purge_source_cache();
    let fresh = embedded_source(layer.smart_object().expect("object")).expect("fresh");
    assert_eq!(third.data.as_slice(), fresh.data.as_slice());
}

#[test]
fn stack_mode_change_invalidates_the_composited_raster() {
    let _guard = locked();
    let mut layer = stack_layer_at(
        stack_payload_two(),
        Some(pictura_core::StackMode::Median),
        rect(0, 0, 4, 4),
    );
    let d = doc(4, 4, vec![layer.clone()]);
    let first = composited_source(&layer, &d, RasterRoute::SmartSource).expect("first");
    let second = composited_source(&layer, &d, RasterRoute::SmartSource).expect("second");
    assert!(Arc::ptr_eq(&first.buffer, &second.buffer));

    pictura_codec::edit_layer(layer.as_mut(), |parts| {
        parts.smart_object.as_mut().expect("object").stack_mode =
            Some(pictura_core::StackMode::Maximum)
    });
    let third = composited_source(&layer, &d, RasterRoute::SmartSource).expect("third");
    assert!(!Arc::ptr_eq(&first.buffer, &third.buffer));
}

#[test]
fn stack_frames_keep_the_embed_nesting_bound() {
    let _guard = locked();
    let levels = usize::from(MAX_EMBED_NESTING) - 1;
    let mut payload_doc = doc(
        4,
        4,
        vec![
            pictura_codec::layer_from_parts(pictura_core::LayerParts {
                name: "nested".into(),
                rect: rect(0, 0, 4, 4),
                smart_object: Some({
                    let mut so = SmartObject::default();
                    so.kind = SmartObjectKind::Embedded;
                    so.set_payload(nested_payload(levels));
                    so
                }),
                ..Default::default()
            }),
            solid(
                "solid",
                rect(0, 0, 4, 4),
                (0, 0, 10),
                255,
                pictura_core::BlendMode::Normal,
                255,
            ),
        ],
    );
    payload_doc
        .composite
        .replace(crate::composite_rgba(&payload_doc))
        .expect("valid composite");
    let payload = pictura_codec::write_psd(&payload_doc).expect("payload");
    let layer = stack_layer_at(
        payload,
        Some(pictura_core::StackMode::Maximum),
        rect(0, 0, 4, 4),
    );
    let out = crate::composite_rgba(&doc(4, 4, vec![layer]));
    assert_eq!(
        centre(&out),
        [0, 0, 10, 255],
        "a frame past the nesting bound contributes nothing"
    );
}

#[test]
fn stack_reduction_matches_the_whole_source() {
    let _guard = locked();
    const W: u32 = 4;
    const H: u32 = 300;
    let gradient = |offset: usize| -> Layer {
        let n = (W * H) as usize;
        let mut r = vec![0u8; n];
        let mut g = vec![0u8; n];
        let mut b = vec![0u8; n];
        let a = vec![255u8; n];
        for y in 0..H as usize {
            for x in 0..W as usize {
                let v = ((y + offset) % 256) as u8;
                let i = y * W as usize + x;
                r[i] = v;
                g[i] = v.wrapping_add(10);
                b[i] = v.wrapping_add(20);
            }
        }
        pictura_codec::layer_from_parts(pictura_core::LayerParts {
            rect: rect(0, 0, H as i32, W as i32),
            channels: vec![
                pictura_core::Channel {
                    id: 0,
                    data: r.into(),
                },
                pictura_core::Channel {
                    id: 1,
                    data: g.into(),
                },
                pictura_core::Channel {
                    id: 2,
                    data: b.into(),
                },
                pictura_core::Channel {
                    id: -1,
                    data: a.into(),
                },
            ],
            ..Default::default()
        })
    };
    let mut first = gradient(0);
    crate::set_layer_style_value(&mut first, "dropShadow.on", 1.0);
    crate::set_layer_style_value(&mut first, "dropShadow.size", 4.0);
    let payload_doc = doc(W, H, vec![first, gradient(100)]);
    let payload = pictura_codec::write_psd(&payload_doc).expect("payload");
    let frame_rect = rect(0, 0, H as i32, W as i32);
    let layer = stack_layer_at(
        payload.clone(),
        Some(pictura_core::StackMode::Mean),
        frame_rect,
    );
    let so = layer.smart_object().expect("object");
    let stacked = crate::render_smart_source(so, frame_rect, frame_rect).expect("stacked");

    let decoded = pictura_codec::read_psd(&payload).expect("decoded");
    let frames: Vec<PixelBuffer> = decoded
        .layers
        .iter()
        .filter(|layer| layer.visible)
        .map(|layer| crate::composite_region::composite_layer_region(layer, &decoded, 0, 0, W, H))
        .collect();
    let expected = pictura_core::reduce_stack(&frames, pictura_core::StackMode::Mean);
    assert_eq!(stacked.data.as_slice(), expected.data.as_slice());
}
