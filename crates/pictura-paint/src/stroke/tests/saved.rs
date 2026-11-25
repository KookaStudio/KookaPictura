//! The stroke's undo copy holds only the tiles its dabs reached (#1603).

use super::*;

/// A textured layer over `rect` on a `side`² canvas, so no saved tile is
/// uniform.
fn textured(side: u32, rect: PsdRect) -> Document {
    let mut doc = offset_layer_doc(side, side, rect, BASE);
    for (id, c) in doc.layers[0].channels_mut().iter_mut().enumerate() {
        for (i, v) in c.data.as_mut_slice().iter_mut().enumerate() {
            *v = ((i * (7 + id) + id * 31) % 251) as u8;
        }
    }
    doc
}

fn pixels(doc: &Document) -> Vec<Vec<u8>> {
    [0i16, 1, 2, -1]
        .map(|id| channel_data(&doc.layers[0], id).expect("channel").to_vec())
        .to_vec()
}

fn hard(diameter: u32) -> StrokeConfig {
    StrokeConfig {
        color: red(),
        diameter,
        hardness: 100,
        ..StrokeConfig::default()
    }
    .sanitized()
}

// [bsm_undo_tiles]
#[test]
fn one_dab_saves_only_the_tiles_under_it() {
    let side = 1024;
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: side,
        right: side,
    };
    let mut doc = textured(side as u32, whole);
    let mut stroke = Stroke::begin_at(&doc, "0", hard(20)).expect("a stroke");
    // The centre of document tile (7, 4): a 20 px dab and its margin stay
    // inside it.
    assert!(
        stroke.sample(&mut doc, sample(480.0, 288.0)),
        "nothing painted"
    );
    let (tiles, bytes) = stroke.saved_tiles();
    assert_eq!(tiles, vec![(7, 4)], "only the tile under the dab is saved");
    assert_eq!(bytes, 4 * 64 * 64, "one 64² block per plane");
    let layer_bytes = 4 * (side * side) as usize;
    assert!(bytes * 200 < layer_bytes, "the copy is not layer-sized");
}

// [bsm_grow]
#[test]
fn a_grow_keeps_the_saved_tiles_and_cancel_restores() {
    // Off the 64 px grid, so the grow shifts the layer origin by a
    // non-multiple of the tile edge.
    let rect = PsdRect {
        top: 70,
        left: 70,
        bottom: 190,
        right: 190,
    };
    let original = textured(512, rect);
    let before = pixels(&original);
    let mut doc = original.clone();
    let mut stroke = Stroke::begin_at(&doc, "0", hard(10)).expect("a stroke");
    assert!(
        stroke.sample(&mut doc, sample(150.0, 150.0)),
        "nothing painted"
    );
    let (inside, _) = stroke.saved_tiles();
    assert_eq!(doc.layers[0].rect, rect, "the first dab stayed inside");
    assert!(!inside.is_empty());

    // Out past the layer's right and bottom edges, so the layer grows.
    for i in 1..=8 {
        let t = i as f32 / 8.0;
        stroke.sample(&mut doc, sample(150.0 + 150.0 * t, 150.0 + 150.0 * t));
    }
    let grown = doc.layers[0].rect;
    assert!(
        grown.right > rect.right && grown.bottom > rect.bottom,
        "the layer did not grow"
    );
    let (after, bytes) = stroke.saved_tiles();
    for tile in &inside {
        assert!(after.contains(tile), "the grow dropped saved tile {tile:?}");
    }
    assert!(
        bytes <= after.len() * 4 * 64 * 64,
        "a held tile is larger than its 64² grid tile"
    );
    // The stroke reached a diagonal band, not the grown layer's whole box.
    let grid = ((grown.width() + 63) / 64 + 1) * ((grown.height() + 63) / 64 + 1);
    assert!(
        (after.len() as i32) < grid,
        "the grow saved the whole layer"
    );

    stroke.cancel(&mut doc);
    assert_eq!(doc.layers[0].rect, rect, "cancel shrank the layer back");
    assert!(pixels(&doc) == before, "cancel left paint behind");
}
