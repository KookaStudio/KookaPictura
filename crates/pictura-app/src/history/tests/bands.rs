use super::*;

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

#[test]
fn opening_a_document_shares_bands_with_history() {
    let (w, h) = (32u32, 24u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    let current = &history.current.as_ref().expect("current").doc;
    assert!(
        shares_a_band(&live, current),
        "opening must share untouched bands with the history"
    );

    let before = live.clone();
    let plane = &mut live.layers[0].channels[0].data;
    plane.fracture(w as usize);
    plane.write_band(0).expect("a band")[0] ^= 0x55;
    history.capture_live(&mut live, &selection, &target, "Paint");

    let mut selection = None;
    let mut target = LayerTarget::default();
    assert!(history
        .undo_live(&mut live, &mut selection, &mut target)
        .is_some());
    assert_eq!(live, before, "undo restores the opened document");
}

#[test]
fn a_capture_scans_only_changed_bands() {
    let (w, h) = (32u32, 24u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    let plane = &mut live.layers[0].channels[0].data;
    plane.fracture(w as usize);
    plane.write_band(0).expect("a band")[0] ^= 0x11;
    history.capture_live(&mut live, &selection, &target, "Paint");

    let delta = history
        .states
        .last()
        .and_then(|entry| match &entry.stored {
            Stored::Delta(delta) => Some(delta),
            _ => None,
        })
        .expect("a paint capture is a delta");
    let layer = delta
        .tiles
        .iter()
        .find(|d| d.index == 1)
        .expect("the painted layer plane");
    assert!(!layer.tiles.is_empty());
    assert!(
        layer.tiles.iter().all(|t| t.y < pictura_core::BAND_ROWS),
        "the scan must stay inside the touched band"
    );
}

#[test]
fn a_contiguous_read_collapses_band_sharing() {
    let (w, h) = (32u32, 128u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    let plane = &mut live.layers[0].channels[0].data;
    plane.fracture(w as usize);
    assert_eq!(plane.band_count(), 2, "128 rows of 64-row bands");
    plane.write_band(0).expect("a band")[0] ^= 0x77;
    let _ = plane.as_slice();
    // The next write adopts the materialized view as the base and rebuilds
    // every band over it, so no band is the allocation history still holds.
    let _ = plane.write_band(0).expect("a band");

    let theirs = &history.current.as_ref().expect("current").doc.layers[0].channels[0].data;
    assert!(
        (0..plane.band_count()).all(|i| !plane.shares_band(i, theirs)),
        "a contiguous read collapses every band's sharing, so the capture scans fully"
    );

    history.capture_live(&mut live, &selection, &target, "Paint");
    let delta = history
        .states
        .last()
        .and_then(|entry| match &entry.stored {
            Stored::Delta(delta) => Some(delta),
            _ => None,
        })
        .expect("a paint capture is a delta");
    let layer = delta
        .tiles
        .iter()
        .find(|d| d.index == 1)
        .expect("the painted layer plane");
    assert!(
        !layer.tiles.is_empty(),
        "the touched band still records its tiles; unchanged bands scan but yield none"
    );
}

#[test]
fn a_live_write_forks_only_touched_bands() {
    let (w, h) = (32u32, 128u32);
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
    plane.fracture(w as usize);
    plane.write_band(1).expect("a band")[0] ^= 0x22;

    let current = &history.current.as_ref().unwrap().doc.layers[0].channels[0].data;
    assert!(
        plane.shares_band(0, current),
        "the untouched band stays shared"
    );
    assert!(!plane.shares_band(1, current), "the written band forked");
    assert_eq!(current.as_slice(), before.as_slice());
}

#[test]
fn a_written_band_is_not_shared_with_history() {
    let (w, h) = (32u32, 128u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");
    assert!(pictura_render::add_layer(&mut live, -1, "added") >= 0);
    history.capture_live(&mut live, &selection, &target, "New Layer");

    let plane = &mut live.layers[0].channels[0].data;
    plane.fracture(w as usize);
    plane.write_band(1).expect("a band")[0] ^= 0x44;
    let theirs = history_planes(&history);
    assert!(
        theirs.iter().all(|q| !plane.shares_band(1, q)),
        "a written band must not share with any history state"
    );
}

#[test]
fn a_stroke_path_materializes_no_plane_and_scans_only_touched_bands() {
    let (w, h) = (32u32, 24u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
    let mut history = History::default();
    let selection = None;
    let target = LayerTarget::default();
    history.capture_live(&mut live, &selection, &target, "Open");

    let plane = &mut live.layers[0].channels[0].data;
    plane.fracture(w as usize);
    plane.write_band(0).expect("a band")[0] ^= 0x66;

    pictura_core::reset_materializations();
    history.capture_live(&mut live, &selection, &target, "Paint");
    assert_eq!(
        pictura_core::materializations(),
        0,
        "the capture must not materialize a plane"
    );
    let delta = history
        .states
        .last()
        .and_then(|entry| match &entry.stored {
            Stored::Delta(delta) => Some(delta),
            _ => None,
        })
        .expect("a paint capture is a delta");
    assert!(delta
        .tiles
        .iter()
        .flat_map(|d| d.tiles.iter())
        .all(|t| t.y < pictura_core::BAND_ROWS));
}
