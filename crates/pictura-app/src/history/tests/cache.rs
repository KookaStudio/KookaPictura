use super::*;
use pictura_core::{BitDepth, Channel, ColorMode, Document, Layer, PsdRect};
use pictura_paint::healing::RgbaImage;
use pictura_paint::stamp::{layer_surface, StampSource};
use pictura_paint::{Stroke, StrokeConfig, StrokeSample};
use std::sync::Arc;

fn layered(seed: u8, layers: usize) -> Document {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    for i in 0..layers {
        let _ = doc.layers.push(Layer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            },
            channels: [(0, 0), (1, 1), (2, 2), (-1, 3)]
                .map(|(id, c)| Channel {
                    id,
                    data: vec![seed.wrapping_add(i as u8).wrapping_add(c as u8); 64].into(),
                })
                .into(),
            ..Default::default()
        });
    }
    doc
}

fn paint(source: Arc<RgbaImage>) -> Document {
    let doc = layered(10, 2);
    let cfg = StrokeConfig {
        diameter: 4,
        ..StrokeConfig::default()
    };
    let mut stroke =
        Stroke::begin_source(&doc, "0", cfg, StampSource::new(source, (0, 0))).expect("begin");
    let mut out = doc.clone();
    for x in [2.0, 4.0, 6.0] {
        stroke.sample(
            &mut out,
            StrokeSample {
                x,
                y: 4.0,
                pressure: 1.0,
            },
        );
    }
    stroke.finish().expect("painted");
    out
}

fn two_states() -> (History, Document, LayerTarget) {
    let mut history = History::default();
    let mut doc = layered(10, 2);
    let target = LayerTarget::default();
    history.capture_live(&mut doc, &None, &target, "Open");
    for i in 0..2 {
        doc.layers[i].channels[0].data = vec![99; 64].into();
    }
    history.capture_live(&mut doc, &None, &target, "Edit");
    (history, doc, target)
}

#[test]
fn brush_source_surface_is_reused() {
    let (mut history, mut doc, target) = two_states();
    assert!(history.set_brush_source(BrushSource::State(0)));
    let first = history.brush_source_surface("0").expect("first surface");
    // A committed stroke adds a state; the source state itself is unchanged.
    doc.layers[0].channels[1].data = vec![7; 64].into();
    history.capture_live(&mut doc, &None, &target, "Stroke");
    let second = history.brush_source_surface("0").expect("second surface");
    assert!(
        Arc::ptr_eq(&first, &second),
        "a commit must not invalidate the source"
    );
    assert_eq!(*first, *second);
    assert_eq!(paint(first.clone()), paint(second.clone()));
}

#[test]
fn brush_source_surface_reuses_a_pinned_source() {
    let (mut history, mut doc, mut target) = two_states();
    assert!(history.set_brush_source(BrushSource::State(1)));
    let first = history.brush_source_surface("0").expect("first surface");
    let mut selection = None;
    assert!(history
        .undo_live(&mut doc, &mut selection, &mut target)
        .is_some());
    history.capture_live(&mut doc, &selection, &target, "After undo");
    let second = history.brush_source_surface("0").expect("pinned surface");
    assert!(
        Arc::ptr_eq(&first, &second),
        "a source pinned by truncation must keep its surface"
    );
    assert_eq!(paint(first.clone()), paint(second.clone()));
}

#[test]
fn brush_source_surface_reuses_a_source_dropped_by_depth() {
    let (mut history, mut doc, target) = two_states();
    assert!(history.set_brush_source(BrushSource::State(0)));
    let first = history.brush_source_surface("0").expect("first surface");
    for i in 0..(crate::history::MAX_DEPTH + 2) {
        doc.layers[0].channels[0].data = vec![i as u8; 64].into();
        history.capture_live(&mut doc, &None, &target, "Step");
    }
    let second = history.brush_source_surface("0").expect("pinned surface");
    assert!(
        Arc::ptr_eq(&first, &second),
        "the pinned source must stay cached when the stack drops it"
    );
}

#[test]
fn brush_source_surface_invalidates_on_history_change() {
    let (mut history, _doc, target) = two_states();
    assert!(history.set_brush_source(BrushSource::State(0)));
    let first = history.brush_source_surface("0").expect("first surface");

    // A different source state.
    assert!(history.set_brush_source(BrushSource::State(1)));
    let other = history.brush_source_surface("0").expect("other surface");
    assert!(!Arc::ptr_eq(&first, &other));
    let expected = layer_surface(&history.brush_source_doc().unwrap().doc, "0").unwrap();
    assert_eq!(*other, expected);

    // A different layer path.
    let path = history.brush_source_surface("1").expect("path surface");
    assert!(!Arc::ptr_eq(&other, &path));
    let expected = layer_surface(&history.brush_source_doc().unwrap().doc, "1").unwrap();
    assert_eq!(*path, expected);

    // Purging the states drops state 0 and resets the source to the kept state.
    assert!(history.set_brush_source(BrushSource::State(0)));
    let before_purge = history.brush_source_surface("0").expect("before purge");
    history.purge_states();
    let after_purge = history.brush_source_surface("0").expect("after purge");
    assert!(!Arc::ptr_eq(&before_purge, &after_purge));
    let expected = layer_surface(&history.brush_source_doc().unwrap().doc, "0").unwrap();
    assert_eq!(*after_purge, expected);

    // Purging snapshots resets a snapshot source to the oldest state.
    history.add_snapshot(
        "snap",
        Snapshot {
            doc: layered(50, 2),
            selection: None,
        },
        target.clone(),
    );
    assert!(history.set_brush_source(BrushSource::Snapshot(0)));
    let snapshot = history.brush_source_surface("0").expect("snapshot surface");
    let expected = layer_surface(&history.brush_source_doc().unwrap().doc, "0").unwrap();
    assert_eq!(*snapshot, expected);
    history.purge_snapshots();
    let after_snapshots = history.brush_source_surface("0").expect("after snapshots");
    assert!(!Arc::ptr_eq(&snapshot, &after_snapshots));
    let expected = layer_surface(&history.brush_source_doc().unwrap().doc, "0").unwrap();
    assert_eq!(*after_snapshots, expected);
}
