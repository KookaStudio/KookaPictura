//! A walk across a history segment reports the composite's real change.

use super::*;
use pictura_core::{BitDepth, Channel, ColorMode, Layer, LayerStack};

// Two 256² tiles a side, so one changed tile is a quarter of the document.
const SIDE: u32 = 512;

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
    live.composite.data[(300 * SIDE + 280) as usize] = 200;
    history.capture_live(&mut live, &selection, &target, "Resize");
    assert_eq!(history.anchor_count(), 2);

    // The damage is the box of the samples that changed, not the whole tile.
    let tile = PsdRect {
        top: 300,
        left: 280,
        bottom: 301,
        right: 281,
    };
    assert_eq!(
        history.undo_live(&mut live, &mut selection, &mut target),
        Some(Restored::Region(tile))
    );
    assert_eq!(live.composite.data[(300 * SIDE + 280) as usize], 1);
    assert_eq!(
        history.redo_live(&mut live, &mut selection, &mut target),
        Some(Restored::Region(tile))
    );
    assert_eq!(live.composite.data[(300 * SIDE + 280) as usize], 200);
}

#[test]
fn crossing_a_segment_with_a_changed_channel_stays_bounded() {
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

    let Some(Restored::Region(rect)) = history.undo_live(&mut live, &mut selection, &mut target)
    else {
        panic!("the restore is bounded");
    };
    assert!(
        rect.right <= 256 && rect.bottom <= 256,
        "one tile: {rect:?}"
    );
}

#[test]
fn undoing_a_resize_reports_the_whole_document() {
    let mut live = document();
    let (mut selection, mut target) = (None, LayerTarget::default());
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &target, "Open");
    pictura_render::resize_document(
        &mut live,
        SIDE / 2,
        SIDE / 2,
        pictura_render::Resample::Nearest,
    )
    .expect("resize");
    live.composite = pictura_render::composite_rgba(&live);
    history.capture_live(&mut live, &selection, &target, "Image Size");

    assert_eq!(
        history.undo_live(&mut live, &mut selection, &mut target),
        Some(Restored::Everywhere)
    );
}
