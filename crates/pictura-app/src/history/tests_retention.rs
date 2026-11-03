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
        let (meta, sel) = history.meta_of(k);
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
        let (meta, _) = history.meta_of(k);
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
    let (meta, _) = history.meta_of(0);
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
    let (before, _) = history.meta_of(0);
    let (after, _) = history.meta_of(1);
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
