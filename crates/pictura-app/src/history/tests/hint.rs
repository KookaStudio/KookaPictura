use crate::history::History;
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, Channel, ColorMode, Document, LayerMask, PsdRect};

fn hint_doc() -> Document {
    let mut d = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    let rgba = vec![128u8; 16 * 16 * 4];
    let _ = pictura_render::add_raster_layer_from_rgba(&mut d, "px", 16, 16, &rgba);
    d.composite = pictura_render::composite_rgba(&d);
    d
}

fn write_composite(doc: &mut Document, x: usize, y: usize, value: u8) {
    let width = doc.width as usize;
    doc.composite.data.as_mut_slice()[y * width + x] = value;
}

fn open(history: &mut History, doc: &mut Document) {
    history.capture_live(doc, &None, &LayerTarget::default(), "Open");
}

fn captured(history: &History) -> &Document {
    &history.current.as_ref().expect("current state").doc
}

fn hint_rect() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: 8,
        right: 8,
    }
}

#[test]
fn hinted_and_unhinted_captures_match() {
    let mut hinted = hint_doc();
    let mut plain = hinted.clone();
    let mut h1 = History::default();
    let mut h2 = History::default();
    open(&mut h1, &mut hinted);
    open(&mut h2, &mut plain);
    write_composite(&mut hinted, 2, 2, 240);
    write_composite(&mut plain, 2, 2, 240);
    h1.capture_live_with(
        &mut hinted,
        &None,
        &LayerTarget::default(),
        "Edit",
        Some(&[hint_rect()]),
    );
    h2.capture_live(&mut plain, &None, &LayerTarget::default(), "Edit");
    assert_eq!(
        captured(&h1).composite.data.as_slice(),
        captured(&h2).composite.data.as_slice()
    );
}

#[test]
fn hinted_capture_skips_outside_tiles() {
    let mut doc = Document::new(128, 128, ColorMode::Rgb, BitDepth::Eight);
    let rgba = vec![128u8; 128 * 128 * 4];
    let _ = pictura_render::add_raster_layer_from_rgba(&mut doc, "px", 128, 128, &rgba);
    doc.composite = pictura_render::composite_rgba(&doc);
    let initial = doc.composite.data.as_slice().to_vec();
    let mut history = History::default();
    open(&mut history, &mut doc);
    write_composite(&mut doc, 100, 100, 240);
    history.capture_live_with(
        &mut doc,
        &None,
        &LayerTarget::default(),
        "Edit",
        Some(&[hint_rect()]),
    );
    assert_eq!(
        captured(&history).composite.data.as_slice()[100 * 128 + 100],
        initial[100 * 128 + 100],
        "a caller-violating hint skips the outside tile"
    );
    assert_ne!(
        captured(&history).composite.data.as_slice(),
        doc.composite.data.as_slice()
    );
}

#[test]
fn hinted_capture_undo_is_bit_identical() {
    let mut doc = hint_doc();
    let initial = doc.composite.data.as_slice().to_vec();
    let mut history = History::default();
    open(&mut history, &mut doc);
    write_composite(&mut doc, 2, 2, 240);
    history.capture_live_with(
        &mut doc,
        &None,
        &LayerTarget::default(),
        "Edit",
        Some(&[hint_rect()]),
    );
    let mut selection = None;
    let mut target = LayerTarget::default();
    assert!(history
        .undo_live(&mut doc, &mut selection, &mut target)
        .is_some());
    assert_eq!(doc.composite.data.as_slice(), initial.as_slice());
}

#[test]
fn hinted_capture_falls_back_on_geometry_change() {
    let mut doc = hint_doc();
    let mut history = History::default();
    open(&mut history, &mut doc);
    let rgba = vec![64u8; 16 * 16 * 4];
    let _ = pictura_render::add_raster_layer_from_rgba(&mut doc, "new", 16, 16, &rgba);
    history.capture_live_with(
        &mut doc,
        &None,
        &LayerTarget::default(),
        "Delete",
        Some(&[hint_rect()]),
    );
    assert_eq!(
        history.anchor_count(),
        2,
        "a geometry change starts a segment"
    );
}

#[test]
fn empty_hint_captures_everything() {
    let mut doc = hint_doc();
    let mut history = History::default();
    open(&mut history, &mut doc);
    write_composite(&mut doc, 12, 12, 240);
    history.capture_live_with(&mut doc, &None, &LayerTarget::default(), "Edit", Some(&[]));
    assert_eq!(
        captured(&history).composite.data.as_slice(),
        doc.composite.data.as_slice()
    );
}

#[test]
fn document_channel_hint_matches() {
    let mut doc = hint_doc();
    doc.channels.push(Channel {
        id: 0,
        data: vec![0u8; 16 * 16].into(),
    });
    doc.composite = pictura_render::composite_rgba(&doc);
    let mut plain = doc.clone();
    let mut h1 = History::default();
    let mut h2 = History::default();
    open(&mut h1, &mut doc);
    open(&mut h2, &mut plain);
    doc.channels[0].data.as_mut_slice()[2 * 16 + 2] = 200;
    plain.channels[0].data.as_mut_slice()[2 * 16 + 2] = 200;
    h1.capture_live_with(
        &mut doc,
        &None,
        &LayerTarget::default(),
        "Edit",
        Some(&[hint_rect()]),
    );
    h2.capture_live(&mut plain, &None, &LayerTarget::default(), "Edit");
    assert_eq!(
        captured(&h1).channels[0].data.as_slice(),
        captured(&h2).channels[0].data.as_slice()
    );
}

fn hint_layers_doc() -> Document {
    let mut d = hint_doc();
    let rgba = vec![90u8; 4 * 4 * 4];
    let _ = pictura_render::add_raster_layer_from_rgba(&mut d, "a", 4, 4, &rgba);
    let _ = pictura_render::add_raster_layer_from_rgba(&mut d, "b", 4, 4, &rgba);
    d.layers[1].mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        default_color: 0,
        data: Some(vec![255u8; 16].into()),
        density: 255,
        ..Default::default()
    });
    d.composite = pictura_render::composite_rgba(&d);
    d
}

fn assert_hint_matches(op: impl Fn(&mut Document) -> Option<Vec<PsdRect>>) {
    let mut hinted = hint_layers_doc();
    let mut plain = hinted.clone();
    let mut h1 = History::default();
    let mut h2 = History::default();
    open(&mut h1, &mut hinted);
    open(&mut h2, &mut plain);
    let hint = op(&mut hinted);
    let _ = op(&mut plain);
    h1.capture_live_with(
        &mut hinted,
        &None,
        &LayerTarget::default(),
        "Edit",
        hint.as_deref(),
    );
    h2.capture_live(&mut plain, &None, &LayerTarget::default(), "Edit");
    assert_eq!(captured(&h1), captured(&h2));
}

#[test]
fn each_bounded_op_hint_matches() {
    assert_hint_matches(|doc| {
        let before = pictura_render::damage::influence(doc, "1");
        doc.layers[1].visible = false;
        doc.composite = pictura_render::composite_rgba(doc);
        before.map(|rect| vec![rect])
    });
    assert_hint_matches(|doc| {
        let before = pictura_render::damage::influence(doc, "1");
        assert_eq!(pictura_render::delete_paths(doc, &["1"]), 1);
        doc.composite = pictura_render::composite_rgba(doc);
        before.map(|rect| vec![rect])
    });
    assert_hint_matches(|doc| {
        let before = pictura_render::damage::influence(doc, "1");
        let created = pictura_render::duplicate_layer(doc, 1);
        let after = pictura_render::damage::influence(doc, &created.to_string());
        doc.composite = pictura_render::composite_rgba(doc);
        pictura_render::damage::union(before, after).map(|rect| vec![rect])
    });
    assert_hint_matches(|doc| {
        let before = pictura_render::damage::influence_paths(doc, &["1", "2"]);
        let group = pictura_render::group_paths(doc, &["1", "2"]).expect("group");
        let after = pictura_render::damage::influence(doc, &group);
        doc.composite = pictura_render::composite_rgba(doc);
        match (before, after) {
            (Some(mut rects), Some(after)) => {
                rects.push(after);
                Some(rects)
            }
            _ => None,
        }
    });
    assert_hint_matches(|doc| {
        let before = pictura_render::damage::influence(doc, "1");
        assert!(pictura_render::move_path(doc, "1", 1));
        let after = pictura_render::damage::influence(doc, "2");
        doc.composite = pictura_render::composite_rgba(doc);
        pictura_render::damage::union(before, after).map(|rect| vec![rect])
    });
    assert_hint_matches(|doc| {
        let before = pictura_render::damage::influence_paths(doc, &["1", "2"]);
        assert!(pictura_render::set_opacity_paths(doc, &["1", "2"], 128) > 0);
        let after = pictura_render::damage::influence_paths(doc, &["1", "2"]);
        doc.composite = pictura_render::composite_rgba(doc);
        match (before, after) {
            (Some(mut rects), Some(after)) => {
                rects.extend(after);
                Some(rects)
            }
            _ => None,
        }
    });
    assert_hint_matches(|doc| {
        doc.layers[0].channels[0].data.as_mut_slice()[16 + 1] = 200;
        doc.composite = pictura_render::composite_rgba(doc);
        Some(vec![hint_rect()])
    });
    assert_hint_matches(|doc| {
        doc.layers[1]
            .mask
            .as_mut()
            .expect("mask")
            .data
            .as_mut()
            .expect("mask data")
            .as_mut_slice()[4 + 1] = 20;
        doc.composite = pictura_render::composite_rgba(doc);
        Some(vec![hint_rect()])
    });
}

#[test]
fn random_hinted_and_unhinted_captures_match() {
    let mut rng = 0x1234_5678_9abc_def0u64;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let mut doc = hint_doc();
    let mut plain = doc.clone();
    let mut h1 = History::default();
    let mut h2 = History::default();
    open(&mut h1, &mut doc);
    open(&mut h2, &mut plain);
    for _ in 0..16 {
        let x = (next() % 8) as usize;
        let y = (next() % 8) as usize;
        let value = (next() & 0xff) as u8;
        write_composite(&mut doc, x, y, value);
        write_composite(&mut plain, x, y, value);
        let layer_byte = y * 16 + x;
        doc.layers[0].channels[0].data.as_mut_slice()[layer_byte] = value;
        plain.layers[0].channels[0].data.as_mut_slice()[layer_byte] = value;
        h1.capture_live_with(
            &mut doc,
            &None,
            &LayerTarget::default(),
            "Edit",
            Some(&[hint_rect()]),
        );
        h2.capture_live(&mut plain, &None, &LayerTarget::default(), "Edit");
    }
    assert_eq!(captured(&h1), captured(&h2));
}
