use super::*;
use pictura_core::{
    BitDepth, ColorMode, Document, Layer, LayerStack, Plane, PsdRect, Samples, SmartObject,
    SourceChannels, SourcePlanes,
};

fn depth16_doc() -> Document {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Sixteen);
    doc.source_depth = Some(BitDepth::Sixteen);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Sixteen,
        width: 2,
        height: 2,
        samples: Samples::U16(vec![1; 12].into()),
    });
    doc.layer_section_extra = vec![1, 2, 3].into();
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    };
    let mut layer = Layer {
        rect,
        source_channels: Some(SourceChannels::new(
            BitDepth::Sixteen,
            rect,
            vec![
                (0, Samples::U16(vec![1; 4].into())),
                (1, Samples::U16(vec![2; 4].into())),
            ],
        )),
        ..Default::default()
    };
    layer.smart_object = Some(SmartObject::default().with_payload(vec![9, 9, 9]));
    doc.layers = LayerStack::from_layers(vec![layer]);
    doc
}

fn u16_plane(doc: &Document, index: usize) -> &Plane<u16> {
    let Samples::U16(plane) = &doc.layers[0]
        .source_channels
        .as_ref()
        .expect("a native store")
        .planes[index]
        .1
    else {
        panic!("a u16 store")
    };
    plane
}

fn share_samples(a: &Samples, b: &Samples) -> bool {
    matches!((a, b), (Samples::U16(a), Samples::U16(b)) if a.shares(b))
}

fn share_u16(a: &Plane<u16>, b: &Plane<u16>) -> bool {
    a.shares(b)
}

#[test]
fn native_samples_and_selection_are_shared_across_captures() {
    let mut history = History::default();
    let mut doc = depth16_doc();
    let selection = Some(Selection::all(2, 2));
    let target = LayerTarget::default();
    for i in 0..5 {
        history.capture_live(&mut doc, &selection, &target, &format!("state {i}"));
    }
    let current = history.current.as_ref().expect("current");
    assert_eq!(
        current.selection.as_ref().expect("selection").data.len(),
        4,
        "the fixture must carry a non-empty selection"
    );
    for k in 0..history.states.len() {
        let (meta, sel, _) = history.meta_of(k);
        assert_eq!(
            sel.as_ref().expect("selection").data.len(),
            4,
            "state {k} must keep the non-empty selection"
        );
        assert!(
            share_samples(
                &current.doc.source_planes.as_ref().unwrap().samples,
                &meta.source_planes.as_ref().unwrap().samples
            ),
            "state {k} must share the document native samples"
        );
        assert!(
            share_u16(u16_plane(&current.doc, 0), u16_plane(meta, 0)),
            "state {k} must share the layer native plane"
        );
        assert!(
            sel.as_ref()
                .unwrap()
                .data
                .shares(&current.selection.as_ref().unwrap().data),
            "state {k} must share the selection"
        );
    }
}

#[test]
fn payloads_and_byte_blocks_are_shared_across_captures() {
    let mut history = History::default();
    let mut doc = depth16_doc();
    let selection = Some(Selection::all(2, 2));
    let target = LayerTarget::default();
    for i in 0..5 {
        history.capture_live(&mut doc, &selection, &target, &format!("state {i}"));
    }
    let current = history.current.as_ref().expect("current");
    let current_so = current.doc.layers[0].smart_object.as_ref().unwrap();
    assert!(
        current_so.payload().is_some(),
        "the fixture must carry an embedded payload"
    );
    for k in 0..history.states.len() {
        let (meta, _, _) = history.meta_of(k);
        assert!(current
            .doc
            .layer_section_extra
            .shares(&meta.layer_section_extra));
        let meta_so = meta.layers[0].smart_object.as_ref().unwrap();
        assert!(
            meta_so.payload().is_some(),
            "state {k} must keep the embedded payload"
        );
        assert!(
            current_so.payload_shares(meta_so),
            "state {k} must share the embedded payload"
        );
    }
}

#[test]
fn a_capture_forks_the_tree_container_but_shares_nodes() {
    let mut history = History::default();
    let mut doc = depth16_doc();
    let selection = Some(Selection::all(2, 2));
    let target = LayerTarget::default();
    history.capture_live(&mut doc, &selection, &target, "Open");
    history.capture_live(&mut doc, &selection, &target, "Noop");
    let current = history.current.as_ref().expect("current");
    let (meta, _, _) = history.meta_of(0);
    assert_ne!(
        current.doc.layers.as_slice().as_ptr(),
        meta.layers.as_slice().as_ptr(),
        "a capture forks the tree container"
    );
    assert!(
        share_u16(u16_plane(&current.doc, 0), u16_plane(meta, 0)),
        "the node's bulk stores stay shared"
    );
}

#[test]
fn a_native_plane_edit_forks_only_that_plane() {
    let mut history = History::default();
    let mut doc = depth16_doc();
    let selection = Some(Selection::all(2, 2));
    let target = LayerTarget::default();
    history.capture_live(&mut doc, &selection, &target, "Open");
    {
        let store = doc.layers[0].source_channels.as_mut().unwrap();
        let Samples::U16(plane) = &mut store.planes[0].1 else {
            panic!("a u16 store")
        };
        plane.as_mut_slice()[0] = 999;
    }
    history.capture_live(&mut doc, &selection, &target, "Edit");
    let (before, _, _) = history.meta_of(0);
    let (after, _, _) = history.meta_of(1);
    assert!(
        !share_u16(u16_plane(before, 0), u16_plane(after, 0)),
        "the edited plane forks"
    );
    assert!(
        share_u16(u16_plane(before, 1), u16_plane(after, 1)),
        "the untouched plane stays shared"
    );
}

#[test]
fn native_edit_undo_restores_old_bytes() {
    let mut history = History::default();
    let mut doc = depth16_doc();
    let mut selection = Some(Selection::all(2, 2));
    let mut target = LayerTarget::default();
    history.capture_live(&mut doc, &selection, &target, "Open");
    let old_sample = u16_plane(&doc, 0).to_vec();
    let old_section = doc.layer_section_extra.to_vec();
    let old_payload = doc.layers[0]
        .smart_object
        .as_ref()
        .unwrap()
        .payload()
        .unwrap()
        .to_vec();
    let old_selection = selection.as_ref().unwrap().data.to_vec();
    {
        let store = doc.layers[0].source_channels.as_mut().unwrap();
        let Samples::U16(plane) = &mut store.planes[0].1 else {
            panic!("a u16 store")
        };
        plane.as_mut_slice()[0] = 999;
    }
    doc.layer_section_extra.as_mut_slice()[0] = 42;
    doc.layers[0]
        .smart_object
        .as_mut()
        .unwrap()
        .set_payload(vec![7, 7, 7]);
    selection.as_mut().unwrap().data.as_mut_slice()[0] = 0;
    history.capture_live(&mut doc, &selection, &target, "Edit");
    history
        .undo_live(&mut doc, &mut selection, &mut target)
        .expect("undo after capture");
    assert_eq!(u16_plane(&doc, 0).as_slice(), old_sample.as_slice());
    assert_eq!(doc.layer_section_extra.as_slice(), old_section.as_slice());
    assert_eq!(
        doc.layers[0].smart_object.as_ref().unwrap().payload(),
        Some(old_payload.as_slice())
    );
    assert_eq!(
        selection.as_ref().unwrap().data.as_slice(),
        old_selection.as_slice()
    );
}

fn depth16_doc_sized(w: u32, h: u32) -> Document {
    let plane = w as usize * h as usize;
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Sixteen);
    doc.source_depth = Some(BitDepth::Sixteen);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Sixteen,
        width: w,
        height: h,
        samples: Samples::U16(vec![1; plane * 3].into()),
    });
    doc.layer_section_extra = vec![0u8; 8 * 1024 * 1024].into();
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    };
    let mut layer = Layer {
        rect,
        source_channels: Some(SourceChannels::new(
            BitDepth::Sixteen,
            rect,
            vec![(0, Samples::U16(vec![1; plane].into()))],
        )),
        ..Default::default()
    };
    layer.smart_object = Some(SmartObject::default().with_payload(vec![0u8; 8 * 1024 * 1024]));
    doc.layers = LayerStack::from_layers(vec![layer]);
    doc
}

#[test]
#[ignore = "profile: 20 captures on a 2000x2000 16-bit document"]
fn history_capture_profile() {
    let (w, h) = (2000, 2000);
    let mut history = History::default();
    let mut doc = depth16_doc_sized(w, h);
    let selection = Some(Selection::all(w, h));
    let target = LayerTarget::default();
    let started = std::time::Instant::now();
    for i in 0..20 {
        history.capture_live(&mut doc, &selection, &target, &format!("state {i}"));
    }
    let elapsed = started.elapsed();
    let rss = std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .unwrap_or("")
        .trim()
        .to_string();
    println!(
        "20 captures: {elapsed:?} ({:?}/capture), {rss}",
        elapsed / 20
    );
}

fn stamped_doc() -> Document {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    let channels = [0i16, 1, 2, -1]
        .map(|id| pictura_core::Channel {
            id,
            data: vec![9; 4].into(),
        })
        .to_vec();
    doc.layers = LayerStack::from_layers(vec![Layer {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        },
        channels,
        ..Default::default()
    }]);
    doc
}

fn layer_stamps(doc: &Document, layer: usize) -> Vec<u64> {
    doc.layers[layer]
        .channels
        .iter()
        .map(|c| c.data.stamp())
        .collect()
}

#[test]
fn a_capture_keeps_the_stamps_of_unchanged_planes() {
    let mut history = History::default();
    let mut doc = stamped_doc();
    let target = LayerTarget::default();
    history.capture_live(&mut doc, &None, &target, "Open");
    let before = layer_stamps(&doc, 0);
    assert!(before.iter().all(|&stamp| stamp != 0));

    doc.layers.get_mut(0).unwrap().channels[1]
        .data
        .as_mut_slice()[0] = 1;
    history.capture_live(&mut doc, &None, &target, "Edit");
    let after = layer_stamps(&doc, 0);
    assert_eq!(after[0], before[0]);
    assert_eq!(after[2], before[2]);
    assert_eq!(after[3], before[3]);
    assert!(after[1] != 0 && after[1] != before[1], "the written plane");
}

#[test]
fn undo_keeps_the_stamps_of_unchanged_planes() {
    let mut history = History::default();
    let mut doc = stamped_doc();
    let target = LayerTarget::default();
    history.capture_live(&mut doc, &None, &target, "Open");
    doc.layers.get_mut(0).unwrap().channels[1]
        .data
        .as_mut_slice()[0] = 1;
    history.capture_live(&mut doc, &None, &target, "Edit");
    let edited = layer_stamps(&doc, 0);

    let (mut selection, mut target) = (None, target);
    assert!(history
        .undo_live(&mut doc, &mut selection, &mut target)
        .is_some());
    let undone = layer_stamps(&doc, 0);
    assert_eq!(undone[0], edited[0]);
    assert_eq!(undone[2], edited[2]);
    assert_eq!(undone[3], edited[3]);
    assert_eq!(doc.layers[0].channels[1].data[0], 9);
}

#[test]
fn a_duplicated_layer_gets_unique_stable_stamps() {
    let mut history = History::default();
    let mut doc = stamped_doc();
    let target = LayerTarget::default();
    history.capture_live(&mut doc, &None, &target, "Open");
    let copy = doc.layers[0].clone();
    doc.layers.push(copy).unwrap();
    history.capture_live(&mut doc, &None, &target, "Duplicate");

    let stamps = [layer_stamps(&doc, 0), layer_stamps(&doc, 1)].concat();
    let unique: std::collections::HashSet<u64> = stamps.iter().copied().collect();
    assert_eq!(unique.len(), stamps.len());
    assert!(!unique.contains(&0));

    history.capture_live(&mut doc, &None, &target, "Again");
    assert_eq!(
        [layer_stamps(&doc, 0), layer_stamps(&doc, 1)].concat(),
        stamps
    );
}

// Tile sharing (#1215): states and snapshots hold references to tiles, and the
// budget counts each tile history keeps beyond the live document once.

const SIDE: u32 = 512;
const TILE_BYTES: u64 = (pictura_core::TILE * pictura_core::TILE) as u64;

/// A 512² document with one patterned layer: four dense 256² tiles a plane.
fn patterned_doc() -> Document {
    let mut doc = Document::new(SIDE, SIDE, ColorMode::Rgb, BitDepth::Eight);
    let rgba: Vec<u8> = (0..(SIDE * SIDE * 4) as usize)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add((i >> 11) as u8))
        .collect();
    let _ = pictura_render::add_raster_layer_from_rgba(&mut doc, "px", SIDE, SIDE, &rgba);
    doc.composite = pictura_render::composite_rgba(&doc);
    doc
}

/// Write `value` over `rect` of layer `layer`'s channel `channel`.
fn paint(
    doc: &mut Document,
    layer: usize,
    channel: usize,
    rect: (usize, usize, usize, usize),
    value: u8,
) {
    let (x, y, w, h) = rect;
    let width = doc.layers[layer].rect.width().max(0) as usize;
    let plane = &mut doc.layers[layer].channels[channel].data;
    plane.write_rect_from(x..x + w, y..y + h, width, &vec![value; w * h]);
}

fn record(history: &mut History, live: &mut Document, label: &str) {
    history.capture_live(live, &None, &LayerTarget::default(), label);
}

fn snapshot_of(live: &Document) -> Snapshot {
    Snapshot {
        doc: live.clone(),
        selection: None,
        quick_mask: None,
    }
}

#[test]
fn shared_tiles_counted_once() {
    let mut live = patterned_doc();
    let mut history = History::default();
    record(&mut history, &mut live, "Open");
    assert_eq!(
        history.held_bytes(),
        0,
        "an opened document costs nothing more"
    );

    paint(&mut live, 0, 0, (10, 10, 4, 4), 1);
    record(&mut history, &mut live, "A");
    assert_eq!(
        history.held_bytes(),
        TILE_BYTES,
        "A keeps the tile it replaced"
    );

    history.add_snapshot("At A", snapshot_of(&live), LayerTarget::default());
    assert_eq!(
        history.held_bytes(),
        TILE_BYTES,
        "a snapshot of the live document shares every tile with it"
    );

    paint(&mut live, 0, 0, (20, 20, 4, 4), 2);
    record(&mut history, &mut live, "B");
    assert_eq!(
        history.held_bytes(),
        2 * TILE_BYTES,
        "B's delta and the snapshot hold one tile, counted once"
    );
    assert_eq!(history.held_bytes(), history.recount_held_bytes());

    let (mut selection, mut target) = (None, LayerTarget::default());
    history.undo_live(&mut live, &mut selection, &mut target);
    assert_eq!(
        history.held_bytes(),
        2 * TILE_BYTES,
        "after the undo the snapshot's tile is live again and B keeps its redo tile"
    );
    history.redo_live(&mut live, &mut selection, &mut target);
    assert_eq!(history.held_bytes(), 2 * TILE_BYTES);
}

#[test]
fn snapshot_costs_no_pixels() {
    let mut live = patterned_doc();
    let mut history = History::default();
    record(&mut history, &mut live, "Open");
    paint(&mut live, 0, 1, (300, 300, 8, 8), 9);
    record(&mut history, &mut live, "Paint");
    let held = history.held_bytes();

    pictura_core::reset_materializations();
    let snapshot = snapshot_of(&live);
    history.add_snapshot("Snapshot 1", snapshot, LayerTarget::default());
    assert_eq!(
        history.held_bytes(),
        held,
        "taking a snapshot copies no tile"
    );

    paint(&mut live, 0, 2, (0, 0, 8, 8), 3);
    record(&mut history, &mut live, "Edit");
    let edited = history.held_bytes();
    assert_eq!(
        edited,
        held + TILE_BYTES,
        "the edit keeps the tile it replaced"
    );

    let restored = history.snapshot(0).expect("the snapshot");
    live = restored.doc;
    record(&mut history, &mut live, "Snapshot 1");
    assert_eq!(
        pictura_core::materializations(),
        0,
        "the snapshot and its restore materialize no plane"
    );
    assert_eq!(
        history.held_bytes(),
        edited,
        "the restore swaps references: the edited tile moves into the delta, no tile is copied"
    );
    let snap = history.snapshot(0).expect("the snapshot");
    let mut shared = true;
    let mut theirs = Vec::new();
    planes::each_plane(&snap.doc, &mut |p, _, _, _| theirs.push(p.clone()));
    let mut index = 0;
    planes::each_plane(&live, &mut |p, _, _, _| {
        shared &= p.shares(&theirs[index]);
        index += 1;
    });
    assert!(
        shared,
        "the restored document shares every tile with the snapshot"
    );
}

#[test]
fn purge_releases_history_tiles() {
    let mut live = patterned_doc();
    let mut history = History::default();
    record(&mut history, &mut live, "Open");
    paint(&mut live, 0, 0, (0, 0, 4, 4), 1);
    record(&mut history, &mut live, "A");
    history.add_snapshot("At A", snapshot_of(&live), LayerTarget::default());
    paint(&mut live, 0, 0, (0, 0, 4, 4), 2);
    record(&mut history, &mut live, "B");
    assert_eq!(history.held_bytes(), 2 * TILE_BYTES);

    history.purge_states();
    assert!(!history.can_undo(), "a purge is not undoable");
    assert_eq!(history.count(), 1);
    assert_eq!(
        history.held_bytes(),
        TILE_BYTES,
        "only the snapshot's tile is left"
    );
    history.purge_snapshots();
    assert_eq!(history.held_bytes(), 0);

    paint(&mut live, 0, 0, (0, 0, 4, 4), 3);
    record(&mut history, &mut live, "C");
    history.add_snapshot("At C", snapshot_of(&live), LayerTarget::default());
    paint(&mut live, 0, 0, (0, 0, 4, 4), 4);
    record(&mut history, &mut live, "D");
    assert!(history.held_bytes() > 0);
    history.purge();
    assert_eq!(
        history.held_bytes(),
        0,
        "Purge All releases every history tile"
    );
    assert_eq!(history.held_bytes(), history.recount_held_bytes());
    assert!(!history.can_undo());
    assert_eq!(history.snapshot_count(), 0);
}

/// A fixed-seed linear congruential generator.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % bound.max(1)
    }
}

#[test]
fn undo_redo_identity_seeded() {
    let mut rng = Lcg(0x1215);
    let mut live = patterned_doc();
    let mut history = History::default();
    record(&mut history, &mut live, "Open");
    let mut states = vec![live.clone()];
    for step in 0..16 {
        match rng.below(5) {
            0 => {
                let rgba = vec![rng.below(256) as u8; 64 * 64 * 4];
                let _ = pictura_render::add_raster_layer_from_rgba(&mut live, "new", 64, 64, &rgba);
            }
            1 => {
                let layer = rng.below(live.layers.len());
                live.layers[layer].visible = !live.layers[layer].visible;
            }
            _ => {
                let layer = rng.below(live.layers.len());
                let width = live.layers[layer].rect.width().max(0) as usize;
                let height = live.layers[layer].rect.height().max(0) as usize;
                let (w, h) = (
                    1 + rng.below(width.min(300)),
                    1 + rng.below(height.min(300)),
                );
                let (x, y) = (rng.below(width - w + 1), rng.below(height - h + 1));
                let channel = rng.below(live.layers[layer].channels.len());
                paint(
                    &mut live,
                    layer,
                    channel,
                    (x, y, w, h),
                    rng.below(256) as u8,
                );
            }
        }
        record(&mut history, &mut live, &format!("Edit {step}"));
        states.push(live.clone());
    }
    let (mut selection, mut target) = (None, LayerTarget::default());
    for expected in states.iter().rev().skip(1) {
        history
            .undo_live(&mut live, &mut selection, &mut target)
            .expect("an undo");
        assert!(live == *expected, "undo restores the captured bytes");
        let current = &history.current.as_ref().expect("current").doc;
        assert!(
            planes::planes_agree(current, &live),
            "stamps agree after undo"
        );
    }
    for expected in states.iter().skip(1) {
        history
            .redo_live(&mut live, &mut selection, &mut target)
            .expect("a redo");
        assert!(live == *expected, "redo restores the captured bytes");
        let current = &history.current.as_ref().expect("current").doc;
        assert!(
            planes::planes_agree(current, &live),
            "stamps agree after redo"
        );
    }
    assert_eq!(history.held_bytes(), history.recount_held_bytes());
}
