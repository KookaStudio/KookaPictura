//! A whole-layer change moves through the history by sharing planes, not by
//! copying its tiles twice (#1149).

use super::*;

/// Every byte of the layer's channels, flipped, as a full-canvas fill does.
fn repaint(doc: &mut Document) {
    for channel in doc.layers[0].channels.iter_mut() {
        for v in channel.data.as_mut_slice() {
            *v ^= 0x5a;
        }
    }
}

fn shares_layer_planes(a: &Document, b: &Document) -> bool {
    a.layers[0]
        .channels
        .iter()
        .zip(&b.layers[0].channels)
        .all(|(a, b)| {
            // A swapped plane's tiles have diverged one by one, so they are
            // compared one by one.
            let (a, b) = (&a.data, &b.data);
            a.shares(b) || (a.tile_count() > 0 && (0..a.tile_count()).all(|i| a.shares_tile(i, b)))
        })
}

#[test]
fn a_whole_layer_change_is_shared_and_undoes_in_place() {
    // 200 rows: three whole bands and a short last one.
    let (w, h) = (40u32, 200u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let (mut selection, mut target) = (None, LayerTarget::default());
    history.capture_live(&mut live, &selection, &target, "Open");
    let opened = live.clone();

    repaint(&mut live);
    let painted = live.clone();
    history.capture_live(&mut live, &selection, &target, "Paint Bucket");
    let current = &history.current.as_ref().expect("current").doc;
    assert!(
        shares_layer_planes(current, &live),
        "the captured state shares the live planes"
    );

    assert!(history
        .undo_live(&mut live, &mut selection, &mut target)
        .is_some());
    assert_eq!(live, opened, "undo restores the opened layer");
    let current = &history.current.as_ref().expect("current").doc;
    assert!(
        shares_layer_planes(current, &live),
        "undo shares the restored planes with the live document"
    );

    assert!(history
        .redo_live(&mut live, &mut selection, &mut target)
        .is_some());
    assert_eq!(live, painted, "redo restores the fill");
    assert!(history
        .undo_live(&mut live, &mut selection, &mut target)
        .is_some());
    assert_eq!(live, opened, "a second undo restores the opened layer");
}

#[test]
fn a_hinted_capture_shares_its_fully_hinted_tiles() {
    let (w, h) = (40u32, 200u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let (mut selection, mut target) = (None, LayerTarget::default());
    history.capture_live(&mut live, &selection, &target, "Open");
    let opened = live.clone();

    repaint(&mut live);
    let hint = [PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }];
    history.capture_live_with(&mut live, &selection, &target, "Fill", Some(&hint));
    let current = &history.current.as_ref().expect("current").doc;
    // The hint covers every tile, so each changed tile is taken from live
    // whole, sharing its allocation instead of copying its deltas (#1213).
    assert!(shares_layer_planes(current, &live));
    assert_eq!(current.layers[0].channels, live.layers[0].channels);

    assert!(history
        .undo_live(&mut live, &mut selection, &mut target)
        .is_some());
    assert_eq!(live, opened);
}
