//! Growing a stroke's target layer (#1208).

use super::*;

/// A document whose only layer is an empty-rect pixel layer (#1208).
fn empty_layer_doc(w: u32, h: u32) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    let _ = doc.layers.push(Layer {
        name: "empty".into(),
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
        channels: [0i16, 1, 2, -1]
            .into_iter()
            .map(|id| Channel {
                id,
                data: Vec::new().into(),
            })
            .collect(),
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    });
    doc
}

fn pixel_at(doc: &Document, x: i32, y: i32, id: i16) -> u8 {
    let rect = doc.layers[0].rect;
    let w = rect.width() as usize;
    channel_data(&doc.layers[0], id).expect("channel")
        [(y - rect.top) as usize * w + (x - rect.left) as usize]
}

#[test]
fn paint_outside_rect_grows_layer() {
    let cfg = StrokeConfig {
        color: Rgba {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        },
        diameter: 6,
        hardness: 100,
        opacity: 100,
        flow: 100,
        ..StrokeConfig::default()
    }
    .sanitized();
    let mut grown = empty_layer_doc(32, 32);
    let mut reference = transparent_doc(32, 32);
    let mut a = Stroke::begin_at(&grown, "0", cfg).expect("a stroke");
    let mut b = Stroke::begin_at(&reference, "0", cfg).expect("a stroke");
    assert!(a.sample(&mut grown, sample(20.0, 20.0)));
    assert!(b.sample(&mut reference, sample(20.0, 20.0)));

    let rect = grown.layers[0].rect;
    assert!(rect.left <= 20 && rect.top <= 20 && rect.right > 20 && rect.bottom > 20);
    for id in [0i16, 1, 2, -1] {
        assert_eq!(
            pixel_at(&grown, 20, 20, id),
            pixel_at(&reference, 20, 20, id),
            "channel {id} matches a canvas-sized stroke"
        );
    }
    assert_eq!(
        pixel_at(&grown, rect.left, rect.top, -1),
        pixel_at(&reference, rect.left, rect.top, -1),
        "the grown edge is transparent"
    );
}

#[test]
fn fill_outside_rect_grows_layer() {
    use crate::fill::fill_layer_within;
    let mut doc = empty_layer_doc(32, 32);
    let box_ = PsdRect {
        top: 10,
        left: 12,
        bottom: 16,
        right: 18,
    };
    let dirty = fill_layer_within(
        &mut doc,
        "0",
        PaintMode::Normal,
        None,
        crate::Alpha::Blend,
        Some(box_),
        |x, y| {
            (x >= box_.left && x < box_.right && y >= box_.top && y < box_.bottom).then_some((
                Rgba {
                    r: 10,
                    g: 20,
                    b: 30,
                    a: 255,
                },
                1.0,
            ))
        },
    );
    assert!(dirty.is_some());
    assert_eq!(doc.layers[0].rect, box_, "the layer grows to its box");
    assert_eq!(pixel_at(&doc, 12, 10, 0), 10);
    assert_eq!(pixel_at(&doc, 17, 15, -1), 255);
}
