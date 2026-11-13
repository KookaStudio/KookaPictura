use super::*;
use pictura_core::TILE;

fn history_planes(history: &History) -> Vec<pictura_core::Plane<u8>> {
    let mut out = Vec::new();
    let mut collect = |doc: &Document| {
        planes::each_plane(doc, &mut |p, _, _, _| {
            if !p.is_empty() {
                out.push(p.clone());
            }
        });
    };
    for entry in &history.states {
        collect(match &entry.stored {
            Stored::Full(s) | Stored::Hollow(s) => &s.doc,
            Stored::Delta(d) => &d.meta,
        });
    }
    if let Some(current) = &history.current {
        collect(&current.doc);
    }
    out
}

/// Flip sample `(0, y)` of a `w × h` plane by `xor`, tiling it first.
fn flip(plane: &mut pictura_core::Plane<u8>, w: u32, h: u32, y: usize, xor: u8) {
    plane.tile(w as usize, h as usize);
    let v = plane.sample(y * w as usize).expect("an in-plane sample");
    plane.write_rect_from(0..1, y..y + 1, w as usize, &[v ^ xor]);
}

fn last_delta(history: &History) -> &Delta {
    history
        .states
        .last()
        .and_then(|entry| match &entry.stored {
            Stored::Delta(delta) => Some(delta),
            _ => None,
        })
        .expect("a paint capture is a delta")
}

#[test]
fn opening_a_document_shares_tiles_with_history() {
    let (w, h) = (32u32, 24u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    let current = &history.current.as_ref().expect("current").doc;
    assert!(
        shares_a_tile(&live, current),
        "opening must share untouched tiles with the history"
    );

    let before = live.clone();
    flip(&mut live.layers[0].channels[0].data, w, h, 0, 0x55);
    history.capture_live(&mut live, &selection, &target, "Paint");

    let mut selection = None;
    let mut target = LayerTarget::default();
    assert!(history
        .undo_live(&mut live, &mut selection, &mut target)
        .is_some());
    assert_eq!(live, before, "undo restores the opened document");
}

#[test]
fn a_capture_scans_only_changed_tiles() {
    let (w, h) = (32u32, 600u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    flip(&mut live.layers[0].channels[0].data, w, h, 0, 0x11);
    history.capture_live(&mut live, &selection, &target, "Paint");

    let layer = last_delta(&history)
        .tiles
        .iter()
        .find(|d| d.index == 1)
        .expect("the painted layer plane");
    assert!(!layer.tiles.is_empty());
    assert!(
        layer.tiles.iter().all(|t| t.y < TILE),
        "the scan must stay inside the touched tile"
    );
}

#[test]
fn a_contiguous_write_collapses_tile_sharing() {
    let (w, h) = (32u32, 600u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    let plane = &mut live.layers[0].channels[0].data;
    plane.tile(w as usize, h as usize);
    assert_eq!(plane.tile_count(), 3, "600 rows of 256-row tiles");
    // A whole-plane write flattens the plane: nothing is shared any more.
    plane.as_mut_slice()[0] ^= 0x77;
    let theirs = &history.current.as_ref().expect("current").doc.layers[0].channels[0].data;
    assert!(!plane.shares(theirs));

    history.capture_live(&mut live, &selection, &target, "Paint");
    let layer = last_delta(&history)
        .tiles
        .iter()
        .find(|d| d.index == 1)
        .expect("the painted layer plane");
    assert!(
        !layer.tiles.is_empty(),
        "the touched tile still records its deltas; unchanged tiles scan but yield none"
    );
    assert!(layer.tiles.iter().all(|t| t.y < TILE));
}

#[test]
fn a_live_write_forks_only_touched_tiles() {
    let (w, h) = (32u32, 600u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    let before = history.current.as_ref().unwrap().doc.layers[0].channels[0]
        .data
        .clone();
    let plane = &mut live.layers[0].channels[0].data;
    flip(plane, w, h, TILE, 0x22);

    let current = &history.current.as_ref().unwrap().doc.layers[0].channels[0].data;
    assert!(
        plane.shares_tile(0, current),
        "the untouched tile stays shared"
    );
    assert!(!plane.shares_tile(1, current), "the written tile forked");
    assert_eq!(*current, before);
}

#[test]
fn a_written_tile_is_not_shared_with_history() {
    let (w, h) = (32u32, 600u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");
    assert!(pictura_render::add_layer(&mut live, -1, "added") >= 0);
    history.capture_live(&mut live, &selection, &target, "New Layer");

    let plane = &mut live.layers[0].channels[0].data;
    flip(plane, w, h, TILE, 0x44);
    let theirs = history_planes(&history);
    assert!(
        theirs.iter().all(|q| !plane.shares_tile(1, q)),
        "a written tile must not share with any history state"
    );
}

// [tp_history]
#[test]
fn untouched_tile_keeps_its_arc() {
    let (w, h) = (600u32, 600u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");
    let opened = history.current.as_ref().unwrap().doc.layers[0].channels[0]
        .data
        .clone();

    flip(&mut live.layers[0].channels[0].data, w, h, 0, 0x33);
    history.capture_live(&mut live, &selection, &target, "Paint");

    let current = &history.current.as_ref().unwrap().doc.layers[0].channels[0].data;
    let live_plane = &live.layers[0].channels[0].data;
    for i in 1..current.tile_count() {
        assert!(
            current.shares_tile(i, &opened),
            "tile {i} kept its allocation"
        );
        assert!(
            live_plane.shares_tile(i, current),
            "tile {i} is shared with live"
        );
    }
    assert!(!current.shares_tile(0, &opened), "the written tile is new");
}

#[test]
fn a_stroke_path_materializes_no_plane_and_scans_only_touched_tiles() {
    let (w, h) = (32u32, 600u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    flip(&mut live.layers[0].channels[0].data, w, h, 0, 0x66);

    pictura_core::reset_materializations();
    history.capture_live(&mut live, &selection, &target, "Paint");
    assert_eq!(
        pictura_core::materializations(),
        0,
        "the capture must not materialize a plane"
    );
    assert!(last_delta(&history)
        .tiles
        .iter()
        .flat_map(|d| d.tiles.iter())
        .all(|t| t.y < TILE));
}

/// #1290: a tile away from RAM on either side is recorded as wholly changed,
/// and the capture does not read it back.
#[test]
fn an_away_tile_is_wholly_changed_without_a_restore() {
    use pictura_core::memory::eviction::run_pass;
    use pictura_core::{scratch, Plane, Residency};
    let (w, h) = (300usize, 300usize);
    let mut s = 0x2545_f491_4f6c_dd1du64;
    let noise: Vec<u8> = (0..w * h)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s as u8
        })
        .collect();
    let mut before = Plane::from(noise);
    assert!(before.tile(w, h));
    let mut after = before.clone();
    let v = after.sample(7 * w + 5).expect("in the plane");
    after.write_rect_from(5..6, 7..8, w, &[v ^ 1]);
    assert_eq!(
        planes::changed_or_away(&before, &after, 0),
        Some((5, 7, 1, 1))
    );

    let dir = scratch::default_volume()
        .with_file_name("test-scratch")
        .join(format!("history-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    scratch::set_volumes(&[dir]);
    for _ in 0..4 {
        run_pass(u64::MAX);
    }
    let away = |p: &Plane<u8>| p.tile_residency(0) != Some(Residency::Resident);
    assert!(away(&before) && away(&after), "both tiles evicted");
    assert_eq!(
        planes::changed_or_away(&before, &after, 0),
        before.tile_rect(0)
    );
    assert!(
        away(&before) && away(&after),
        "the capture read neither back"
    );
}
