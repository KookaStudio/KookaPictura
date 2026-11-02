//! A walk across a history segment reports the composite's real change.

use super::*;
use pictura_core::{BitDepth, Channel, ColorMode, Layer, LayerStack};

const SIDE: u32 = 128;

fn layer(side: u32) -> Layer {
    let side_i = side as i32;
    Layer {
        name: "text".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: side_i,
            right: side_i,
        },
        channels: (-1..3)
            .map(|id| Channel {
                id,
                data: vec![9; (side * side) as usize].into(),
            })
            .collect(),
        ..Default::default()
    }
}

fn document() -> Document {
    let mut doc = Document::new(SIDE, SIDE, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![1; (SIDE * SIDE * 4) as usize].into();
    doc.layers = LayerStack::from_layers(vec![layer(SIDE)]);
    doc
}

fn resize_layer(doc: &mut Document) {
    doc.layers = LayerStack::from_layers(vec![layer(SIDE / 2)]);
}

#[test]
fn crossing_a_segment_reports_the_changed_tile() {
    let mut live = document();
    let (mut selection, mut target) = (None, LayerTarget::default());
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    resize_layer(&mut live);
    live.composite.data[(90 * SIDE + 70) as usize] = 200;
    history.capture_live(&mut live, &selection, &target, "Resize");
    assert_eq!(history.anchor_count(), 2);

    let tile = PsdRect {
        top: 64,
        left: 64,
        bottom: 128,
        right: 128,
    };
    assert_eq!(
        history.undo_live(&mut live, &mut selection, &mut target),
        Some(Restored::Region(tile))
    );
    assert_eq!(live.composite.data[(90 * SIDE + 70) as usize], 1);
    assert_eq!(
        history.redo_live(&mut live, &mut selection, &mut target),
        Some(Restored::Region(tile))
    );
    assert_eq!(live.composite.data[(90 * SIDE + 70) as usize], 200);
}

#[test]
fn crossing_a_segment_with_a_changed_channel_reports_everything() {
    let mut live = document();
    live.channels = vec![Channel {
        id: 3,
        data: vec![0; (SIDE * SIDE) as usize].into(),
    }];
    let (mut selection, mut target) = (None, LayerTarget::default());
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    resize_layer(&mut live);
    live.composite.data[0] = 200;
    live.channels[0].data[0] = 255;
    history.capture_live(&mut live, &selection, &target, "Resize");

    assert_eq!(
        history.undo_live(&mut live, &mut selection, &mut target),
        Some(Restored::Everywhere)
    );
}
