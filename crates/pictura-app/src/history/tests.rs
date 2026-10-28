use super::planes::TILE;
use super::*;
use pictura_core::{BitDepth, Channel, CharacterOverrides, ColorMode, Layer, LayerMask, PsdRect};

fn doc(w: u32, h: u32, seed: u8) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![seed; (w * h * 3) as usize].into();
    doc
}

fn snap(seed: u8) -> Snapshot {
    Snapshot {
        doc: doc(1, 1, seed),
        selection: None,
    }
}

#[test]
fn capture_undo_round_trip_restores_doc_and_selection() {
    let mut history = History::default();
    let open = Snapshot {
        doc: doc(2, 1, 7),
        selection: None,
    };
    let selection = Selection {
        width: 2,
        height: 1,
        data: vec![255, 0],
    };
    let selected = Snapshot {
        doc: doc(2, 1, 7),
        selection: Some(selection.clone()),
    };
    history.capture(open, "Open");
    history.capture(selected, "Select All");

    let restored = history.undo().expect("undo after capture");
    assert_eq!((restored.doc.width, restored.doc.height), (2, 1));
    assert!(restored.selection.is_none());

    let forwarded = history.redo().expect("redo after undo");
    assert_eq!(forwarded.selection, Some(selection));
    assert_eq!(forwarded.doc.composite.data, doc(2, 1, 7).composite.data);
}

#[test]
fn style_sheet_edits_are_one_undoable_state_each() {
    let mut history = History::default();
    let mut doc = doc(1, 1, 7);
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );

    doc.text_styles
        .create_character_style(
            "Heading",
            CharacterOverrides {
                size: Some(10.0),
                ..Default::default()
            },
        )
        .unwrap();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "New Character Style",
    );

    doc.text_styles
        .edit_character_style(
            "Heading",
            CharacterOverrides {
                size: Some(24.0),
                ..Default::default()
            },
        )
        .unwrap();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Edit Character Style",
    );

    doc.text_styles.delete_character_style("Heading").unwrap();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Delete Character Style",
    );

    assert_eq!(history.count(), 4);
    assert_eq!(history.depth(), 3);
    assert_eq!(
        history
            .undo()
            .unwrap()
            .doc
            .text_styles
            .character_style("Heading")
            .unwrap()
            .attrs
            .size,
        Some(24.0)
    );
    assert_eq!(
        history
            .undo()
            .unwrap()
            .doc
            .text_styles
            .character_style("Heading")
            .unwrap()
            .attrs
            .size,
        Some(10.0)
    );
    assert!(history
        .undo()
        .unwrap()
        .doc
        .text_styles
        .character_style("Heading")
        .is_none());
    assert_eq!(
        history
            .redo()
            .unwrap()
            .doc
            .text_styles
            .character_style("Heading")
            .unwrap()
            .attrs
            .size,
        Some(10.0)
    );
}

#[test]
fn applying_a_style_is_one_undoable_state() {
    use pictura_core::{
        BitDepth, CharacterOverrides, ColorMode, Justify, ParagraphOverrides, TypeSpec,
    };
    use pictura_render::{add_type_layer, apply_type_style, type_layer_spec};

    let mut doc = Document::new(128, 64, ColorMode::Rgb, BitDepth::Eight);
    let path = add_type_layer(
        &mut doc,
        "",
        &TypeSpec::new("Style", "Liberation Sans", 20.0),
    );
    assert!(!path.is_empty(), "a type layer");
    doc.text_styles
        .create_character_style(
            "Big",
            CharacterOverrides {
                font_family: Some("Liberation Sans".into()),
                size: Some(40.0),
                ..Default::default()
            },
        )
        .unwrap();

    let mut history = History::default();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );
    assert_eq!(history.depth(), 0);

    assert!(apply_type_style(&mut doc, &path, "Big", false));
    assert_eq!(
        type_layer_spec(&doc.layers[0]).unwrap().character.size,
        40.0
    );
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Apply Type Style",
    );
    assert_eq!(history.depth(), 1, "one state for the apply");

    let undone = history.undo().expect("undo the apply");
    assert_eq!(
        type_layer_spec(&undone.doc.layers[0])
            .unwrap()
            .character
            .size,
        20.0
    );

    // A paragraph style resolves both its character and paragraph sets, but the
    // already-applied character style outranks its character defaults; an
    // unknown name leaves the layer alone.
    doc.text_styles
        .create_paragraph_style(
            "Heading",
            CharacterOverrides {
                font_family: Some("Liberation Sans".into()),
                size: Some(30.0),
                ..Default::default()
            },
            ParagraphOverrides {
                justify: Some(Justify::Center),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(apply_type_style(&mut doc, &path, "Heading", true));
    let spec = type_layer_spec(&doc.layers[0]).unwrap();
    assert_eq!(
        spec.character.size, 40.0,
        "the applied character style still outranks the paragraph default"
    );
    assert_eq!(spec.paragraph.justify, Justify::Center);

    let before = doc.layers[0].type_tool.clone();
    assert!(!apply_type_style(&mut doc, &path, "Ghost", false));
    assert_eq!(doc.layers[0].type_tool, before);
}

#[test]
fn selection_none_round_trips() {
    let mut history = History::default();
    let doc0 = doc(2, 2, 3);
    history.capture(
        Snapshot {
            doc: doc0.clone(),
            selection: Some(Selection {
                width: 2,
                height: 2,
                data: vec![255; 4],
            }),
        },
        "Select All",
    );
    history.capture(
        Snapshot {
            doc: doc(3, 3, 4),
            selection: None,
        },
        "Deselect",
    );

    let cleared = history.undo().expect("undo to selection");
    assert_eq!(cleared.selection.unwrap().data, vec![255; 4]);

    let restored = history.redo().expect("redo to none");
    assert_eq!(restored.doc, doc(3, 3, 4));
    assert!(restored.selection.is_none());
}

#[test]
fn empty_stacks_refuse_undo_and_redo() {
    let mut history = History::default();
    assert!(!history.can_undo());
    assert!(!history.can_redo());
    assert_eq!(history.depth(), 0);
    assert_eq!(history.count(), 0);

    assert!(history.undo().is_none());
    assert!(history.redo().is_none());
    assert!(history.jump(0).is_none());
    assert_eq!(history.depth(), 0);
    assert!(!history.can_undo());
    assert!(!history.can_redo());
}

#[test]
fn redo_returns_the_stashed_post_state() {
    let mut history = History::default();
    history.capture(snap(0), "open");
    history.capture(snap(1), "op");

    let restored = history.undo().expect("undo state");
    assert_eq!(restored.doc.composite.data, vec![0, 0, 0]);
    assert_eq!(history.depth(), 0);
    assert!(history.can_redo());

    let forwarded = history.redo().expect("redo state");
    assert_eq!(forwarded.doc.composite.data, vec![1, 1, 1]);
    assert_eq!(history.depth(), 1);
    assert!(history.can_undo());
    assert!(!history.can_redo());
}

#[test]
fn depth_bounded_at_20_and_drops_oldest() {
    let mut history = History::default();
    history.capture(snap(99), "initial");
    for i in 0..21u8 {
        history.capture(snap(i), "op");
    }
    // One state per undoable step plus the current state, capped at depth 20.
    assert_eq!(history.depth(), 20);
    assert_eq!(history.count(), 21);

    let mut last = None;
    for _ in 0..20 {
        last = history.undo();
    }
    let oldest_kept = last.expect("state 20");
    assert_eq!(oldest_kept.doc.composite.data, vec![0, 0, 0]);
    assert_eq!(history.depth(), 0);
    assert!(history.undo().is_none());
}

#[test]
fn new_capture_truncates_redo() {
    let mut history = History::default();
    history.capture(snap(0), "open");
    history.capture(snap(1), "op");
    assert!(history.undo().is_some());
    assert!(history.can_redo());

    history.capture(snap(2), "op2");
    assert!(!history.can_redo());
    assert_eq!(history.depth(), 1);
    assert!(history.can_undo());
}

#[test]
fn labels_track_captures_and_undo_redo_position() {
    let mut history = History::default();
    history.capture(snap(0), "Open");
    history.capture(snap(1), "Select All");
    history.capture(snap(2), "Filter");

    assert_eq!(history.count(), 3);
    assert_eq!(history.index(), 2);
    assert_eq!(history.label(0), "Open");
    assert_eq!(history.label(1), "Select All");
    assert_eq!(history.label(2), "Filter");
    assert_eq!(history.label(9), "");

    history.undo();
    assert_eq!(history.index(), 1);
    history.undo();
    assert_eq!(history.index(), 0);
    history.redo();
    assert_eq!(history.index(), 1);
}

#[test]
fn jump_restores_state_and_moves_position() {
    let mut history = History::default();
    history.capture(snap(0), "Open");
    history.capture(snap(1), "A");
    history.capture(snap(2), "B");

    let jumped = history.jump(0).expect("state 0");
    assert_eq!(jumped.doc.composite.data, vec![0, 0, 0]);
    assert_eq!(history.index(), 0);
    assert!(history.can_redo());
    assert!(!history.can_undo());

    let forward = history.jump(2).expect("state 2");
    assert_eq!(forward.doc.composite.data, vec![2, 2, 2]);
    assert_eq!(history.index(), 2);

    assert!(history.jump(9).is_none());
    assert_eq!(history.index(), 2);
}

#[test]
fn snapshots_are_capped_and_restorable() {
    let mut history = History::default();
    for i in 0..12u8 {
        history.add_snapshot(&format!("snap {i}"), snap(i), LayerTarget::default());
    }
    assert_eq!(history.snapshot_count(), 10);
    // The two oldest snapshots were dropped.
    assert_eq!(history.snapshot_label(0), "snap 2");
    assert_eq!(history.snapshot_label(9), "snap 11");
    assert_eq!(history.snapshot_label(10), "");

    let restored = history.snapshot(3).expect("snapshot 3");
    assert_eq!(restored.doc.composite.data, vec![5, 5, 5]);
    assert!(history.snapshot(99).is_none());
}

#[test]
fn the_brush_source_follows_its_state_and_is_pinned_before_it_is_dropped() {
    let mut history = History::default();
    history.capture(snap(0), "Open");
    assert_eq!(history.brush_source(), BrushSource::Oldest);
    assert_eq!(
        history.brush_source_doc().unwrap().doc.composite.data,
        vec![0, 0, 0]
    );
    for i in 1..=3u8 {
        history.capture(snap(i), "Brush");
    }
    assert!(history.set_brush_source(BrushSource::State(2)));
    assert!(!history.set_brush_source(BrushSource::State(9)));
    assert_eq!(history.brush_source(), BrushSource::State(2));
    // Depth pruning shifts the index down with its state.
    for i in 4..=22u8 {
        history.capture(snap(i), "Brush");
    }
    assert_eq!(history.brush_source(), BrushSource::State(0));
    assert_eq!(
        history.brush_source_doc().unwrap().doc.composite.data,
        vec![2, 2, 2]
    );
    // Dropping it pins a copy instead of reading another state.
    history.capture(snap(23), "Brush");
    assert_eq!(history.brush_source(), BrushSource::Pinned);
    assert_eq!(
        history.brush_source_doc().unwrap().doc.composite.data,
        vec![2, 2, 2]
    );

    // A redo state discarded by a new capture is pinned the same way.
    assert!(history.set_brush_source(BrushSource::State(20)));
    history.undo();
    history.undo();
    history.capture(snap(99), "Brush");
    assert_eq!(history.brush_source(), BrushSource::Pinned);
    assert_eq!(
        history.brush_source_doc().unwrap().doc.composite.data,
        vec![23, 23, 23]
    );

    for i in 0..11u8 {
        history.add_snapshot("s", snap(100 + i), LayerTarget::default());
        if i == 0 {
            assert!(history.set_brush_source(BrushSource::Snapshot(0)));
        }
    }
    assert_eq!(history.brush_source(), BrushSource::Pinned);
    assert_eq!(
        history.brush_source_doc().unwrap().doc.composite.data,
        vec![100, 100, 100]
    );
}

#[test]
fn region_deltas_retain_only_changed_tiles() {
    let mut history = History::default();
    let mut doc = doc(256, 256, 0);
    let full_state_bytes = doc.composite.data.len();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );

    // Ten edits, each touching only the first 64×64 tile.
    for i in 1..=10u8 {
        doc.composite.data[0] = i;
        history.capture(
            Snapshot {
                doc: doc.clone(),
                selection: None,
            },
            "Paint",
        );
    }

    assert_eq!(
        history.full_state_count(),
        1,
        "only the base is a full state"
    );
    let retained = history.retained_tile_bytes();
    assert!(
        retained <= 10 * TILE * TILE,
        "retained {retained} bytes exceeds the changed-tile bound"
    );
    assert!(
        retained < full_state_bytes,
        "retained {retained} bytes should be far below a full state ({full_state_bytes})"
    );

    // Every state still materializes its exact edited byte.
    for i in 0..=10u8 {
        let restored = history.jump(i as usize).expect("state");
        assert_eq!(restored.doc.composite.data[0], i);
    }
}

#[test]
fn every_state_matches_a_reference_copy() {
    let mut doc = doc(200, 130, 5);
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "paint".into(),
        rect: pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: 130,
            right: 200,
        },
        channels: vec![Channel {
            id: 0,
            data: vec![9u8; 200 * 130].into(),
        }],
        ..Default::default()
    }]);

    let mut history = History::default();
    let mut reference: Vec<Snapshot> = Vec::new();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );
    reference.push(Snapshot {
        doc: doc.clone(),
        selection: None,
    });

    // A mix of pixel edits, a metadata-only change, and the region path.
    let mut seed = 7u8;
    for step in 0..8usize {
        seed = seed.wrapping_mul(31).wrapping_add(step as u8);
        for k in 0..500usize {
            let idx = (seed as usize * 137 + k * 61) % doc.composite.data.len();
            doc.composite.data[idx] ^= (seed ^ k as u8) | 1;
        }
        let layer = &mut doc.layers[0];
        for k in 0..300usize {
            let idx = (seed as usize * 53 + k * 97) % layer.channels[0].data.len();
            layer.channels[0].data[idx] = seed.wrapping_add(k as u8);
        }
        if step == 3 {
            doc.layers[0].visible = false;
        }
        let selection = Some(Selection {
            width: 200,
            height: 130,
            data: vec![seed; 200 * 130],
        });
        doc.composite.data[0] = seed;
        let snapshot = Snapshot {
            doc: doc.clone(),
            selection,
        };
        history.capture(snapshot.clone(), "Edit");
        reference.push(snapshot);
    }

    for (i, expected) in reference.iter().enumerate() {
        let jumped = history.jump(i).expect("state");
        assert_eq!(&jumped.doc, &expected.doc, "doc differs at state {i}");
        assert_eq!(jumped.selection, expected.selection, "selection at {i}");
    }

    // Walk down and up through undo/redo, still byte-identical.
    while let Some(snapshot) = history.undo() {
        let i = history.index();
        assert_eq!(&snapshot.doc, &reference[i].doc);
    }
    while let Some(snapshot) = history.redo() {
        let i = history.index();
        assert_eq!(&snapshot.doc, &reference[i].doc);
    }
}

#[test]
fn geometry_change_uses_a_full_anchor_and_still_undoes() {
    let mut history = History::default();
    let open = doc(32, 32, 4);
    history.capture(
        Snapshot {
            doc: open.clone(),
            selection: None,
        },
        "Open",
    );

    let resized = doc(48, 48, 9);
    history.capture(
        Snapshot {
            doc: resized.clone(),
            selection: None,
        },
        "Resize",
    );

    let restored = history.undo().expect("undo across geometry");
    assert_eq!(restored.doc, open);
    let forwarded = history.redo().expect("redo across geometry");
    assert_eq!(forwarded.doc, resized);
}

#[test]
fn paint_commit_undo_restores_the_prior_pixels() {
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "base".into(),
        rect: pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: 64,
            right: 64,
        },
        channels: vec![
            Channel {
                id: 0,
                data: vec![10u8; 64 * 64].into(),
            },
            Channel {
                id: -1,
                data: vec![255u8; 64 * 64].into(),
            },
        ],
        ..Default::default()
    }]);
    let prior_layer = doc.layers[0].channels[0].data.clone();
    let prior_composite = doc.composite.data.clone();

    let mut history = History::default();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );

    // A paint commit: dab bytes into the layer plane and recomposite.
    let mut painted = doc.clone();
    for p in painted.layers[0].channels[0].data[..64 * 32].iter_mut() {
        *p = 200;
    }
    for p in painted.composite.data[..64 * 32].iter_mut() {
        *p = 200;
    }
    history.capture(
        Snapshot {
            doc: painted,
            selection: None,
        },
        "Brush",
    );

    let undone = history.undo().expect("undo the paint");
    assert_eq!(undone.doc.layers[0].channels[0].data, prior_layer);
    assert_eq!(undone.doc.composite.data, prior_composite);

    let redone = history.redo().expect("redo the paint");
    assert_eq!(redone.doc.layers[0].channels[0].data[0], 200);
    assert_eq!(redone.doc.layers[0].channels[0].data[32 * 64], 10);
    assert_eq!(redone.doc.composite.data[0], 200);
}

#[test]
fn editing_a_style_through_the_operation_updates_every_applying_layer() {
    use pictura_core::{CharacterOverrides, TypeSpec};
    use pictura_render::{add_type_layer, apply_type_style, type_layer_spec};

    let mut doc = Document::new(256, 128, ColorMode::Rgb, BitDepth::Eight);
    let first = add_type_layer(&mut doc, "", &TypeSpec::new("Aa", "Liberation Sans", 20.0));
    let second = add_type_layer(&mut doc, "", &TypeSpec::new("Bb", "Liberation Sans", 20.0));
    assert!((first.as_str(), second.as_str()) == ("0", "1"));

    create_character_style(
        &mut doc,
        "Big",
        CharacterOverrides {
            size: Some(40.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(apply_type_style(&mut doc, &first, "Big", false));
    assert!(apply_type_style(&mut doc, &second, "Big", false));
    assert_eq!(
        type_layer_spec(&doc.layers[0]).unwrap().character.size,
        40.0
    );
    assert_eq!(
        type_layer_spec(&doc.layers[1]).unwrap().character.size,
        40.0
    );

    // One edit propagates to both layers, which still reference the style.
    edit_character_style(
        &mut doc,
        "Big",
        CharacterOverrides {
            size: Some(60.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        type_layer_spec(&doc.layers[0]).unwrap().character.size,
        60.0
    );
    assert_eq!(
        type_layer_spec(&doc.layers[1]).unwrap().character.size,
        60.0
    );
    assert_eq!(
        type_layer_spec(&doc.layers[0])
            .unwrap()
            .applied_character_style
            .as_deref(),
        Some("Big")
    );
}

#[test]
fn character_style_reads_resolve_defaults() {
    let mut doc = doc(1, 1, 0);
    create_character_style(
        &mut doc,
        "Quote",
        CharacterOverrides {
            size: Some(48.0),
            ..Default::default()
        },
    )
    .unwrap();
    let resolved = doc.text_styles.resolve(
        &pictura_core::StyleOverrides::default(),
        Some("Quote"),
        None,
    );
    assert_eq!(resolved.character.size, 48.0);
    assert_eq!(resolved.character.language, pictura_core::DEFAULT_LANGUAGE);
}

#[test]
fn renaming_an_applied_style_keeps_it_applied() {
    use pictura_core::TypeSpec;
    use pictura_render::{add_type_layer, apply_type_style};

    let mut doc = Document::new(128, 64, ColorMode::Rgb, BitDepth::Eight);
    let path = add_type_layer(&mut doc, "", &TypeSpec::new("Aa", "Liberation Sans", 20.0));
    create_character_style(
        &mut doc,
        "Quote",
        CharacterOverrides {
            size: Some(40.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(apply_type_style(&mut doc, &path, "Quote", false));
    rename_character_style(&mut doc, "Quote", "Quote 2").unwrap();
    assert_eq!(
        doc.layers[0].applied_character_style.as_deref(),
        Some("Quote 2")
    );
    assert!(doc.text_styles.character_style("Quote").is_none());
    assert!(doc.text_styles.character_style("Quote 2").is_some());
    assert_eq!(
        pictura_render::type_layer_spec(&doc.layers[0])
            .unwrap()
            .character
            .size,
        40.0
    );
}

#[test]
fn a_style_operation_records_one_undoable_state_each() {
    let mut history = History::default();
    let mut doc = doc(1, 1, 7);
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );

    create_character_style(
        &mut doc,
        "Heading",
        CharacterOverrides {
            size: Some(10.0),
            ..Default::default()
        },
    )
    .unwrap();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "New Character Style",
    );
    assert_eq!(history.depth(), 1, "one state for the create");

    edit_character_style(
        &mut doc,
        "Heading",
        CharacterOverrides {
            size: Some(24.0),
            ..Default::default()
        },
    )
    .unwrap();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Edit Character Style",
    );
    assert_eq!(history.depth(), 2, "one state for the edit");
    assert_eq!(
        history
            .undo()
            .unwrap()
            .doc
            .text_styles
            .character_style("Heading")
            .unwrap()
            .attrs
            .size,
        Some(10.0)
    );

    delete_character_style(&mut doc, "Heading").unwrap();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Delete Character Style",
    );
    assert_eq!(history.depth(), 2, "one state for the delete");
    assert!(history
        .undo()
        .unwrap()
        .doc
        .text_styles
        .character_style("Heading")
        .is_some());
}

/// A deterministic generator for the model test (splitmix64).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn model_layer(name: &str, w: u32, h: u32, seed: u8) -> Layer {
    let n = (w * h) as usize;
    Layer {
        name: name.into(),
        rect: pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        },
        channels: (0..4)
            .map(|c| Channel {
                id: c - 1,
                data: (0..n)
                    .map(|i| {
                        (i as u8)
                            .wrapping_mul(7)
                            .wrapping_add(seed.wrapping_add(c as u8))
                    })
                    .collect(),
            })
            .collect(),
        ..Default::default()
    }
}

/// Every plane of `a` is a different allocation from every plane of `b`.
fn shares_no_plane(a: &Document, b: &Document) -> bool {
    let mut theirs = Vec::new();
    planes::each_plane(b, &mut |p, _, _, _| theirs.push(p.clone()));
    let mut ok = true;
    planes::each_plane(a, &mut |p, _, _, _| {
        ok &= p.is_empty() || theirs.iter().all(|q| !p.shares(q));
    });
    ok
}

#[test]
fn live_restores_match_a_full_snapshot_history() {
    let (w, h) = (70u32, 45u32);
    for seed in 0..12u64 {
        let mut rng = Rng(seed);
        let mut live = doc(w, h, 3);
        live.composite.data = (0..(w * h * 3) as usize).map(|i| (i % 251) as u8).collect();
        live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
        let mut selection: Option<Selection> = None;
        let mut target = LayerTarget::default();
        let mut history = History::default();
        history.capture_live(&mut live, &selection, &target, "Open");
        let mut reference = vec![Snapshot {
            doc: live.clone(),
            selection: selection.clone(),
        }];
        let mut cursor = 0usize;

        for step in 0..160 {
            let op = rng.below(14);
            let restore = op >= 9;
            if restore {
                let before = live.composite.data.clone();
                let restored = match op {
                    9 => history.undo_live(&mut live, &mut selection, &mut target),
                    10 => history.redo_live(&mut live, &mut selection, &mut target),
                    11 => {
                        let i = rng.below(reference.len());
                        history.jump_live(i, &mut live, &mut selection, &mut target)
                    }
                    _ => history.undo().map(|s| {
                        live = s.doc;
                        selection = s.selection;
                        Restored::Everywhere
                    }),
                };
                let target = match op {
                    9 => cursor.checked_sub(1),
                    10 => (cursor + 1 < reference.len()).then_some(cursor + 1),
                    11 => Some(history.index()),
                    _ => cursor.checked_sub(1),
                };
                assert_eq!(
                    restored.is_some(),
                    target.is_some(),
                    "seed {seed} step {step}"
                );
                if let Some(t) = target {
                    cursor = t;
                }
                if let Some(Restored::Region(r)) = restored {
                    let hh = h as usize;
                    for (i, (a, b)) in before.iter().zip(live.composite.data.iter()).enumerate() {
                        let (x, y) = ((i % w as usize) as i32, ((i / w as usize) % hh) as i32);
                        let inside = x >= r.left && x < r.right && y >= r.top && y < r.bottom;
                        assert!(
                            a == b || inside,
                            "seed {seed} step {step}: composite changed outside {r:?}"
                        );
                    }
                }
            } else {
                match op {
                    0..=4 => {
                        let li = rng.below(live.layers.len());
                        let c = rng.below(4);
                        let (x, y) = (rng.below(w as usize), rng.below(h as usize));
                        let (rw, rh) = (1 + rng.below(30), 1 + rng.below(20));
                        let v = rng.next() as u8;
                        for yy in y..(y + rh).min(h as usize) {
                            for xx in x..(x + rw).min(w as usize) {
                                live.layers[li].channels[c].data[yy * w as usize + xx] = v;
                                let k = (c % 3) * h as usize + yy;
                                live.composite.data[k * w as usize + xx] = v ^ 0x5A;
                            }
                        }
                    }
                    5 => {
                        let _ = live
                            .layers
                            .push(model_layer("added", w, h, rng.next() as u8));
                    }
                    6 if live.layers.len() > 1 => {
                        let _ = live.layers.pop();
                    }
                    7 => live.layers[0].opacity = rng.next() as u8,
                    _ => selection = Some(Selection::rect(w, h, 2, 3, 1 + rng.below(20) as i32, 4)),
                }
                history.capture_live(&mut live, &selection, &target, "Edit");
                reference.truncate(cursor + 1);
                reference.push(Snapshot {
                    doc: live.clone(),
                    selection: selection.clone(),
                });
                if reference.len() > MAX_DEPTH + 1 {
                    reference.remove(0);
                }
                cursor = reference.len() - 1;
            }
            assert_eq!(history.index(), cursor, "seed {seed} step {step}");
            assert_eq!(history.count(), reference.len(), "seed {seed} step {step}");
            assert!(
                live == reference[cursor].doc,
                "seed {seed} step {step} op {op}: live document differs from the reference"
            );
            assert!(
                selection == reference[cursor].selection,
                "seed {seed} step {step}: selection differs"
            );
            if op < 12 {
                let current = &history.current.as_ref().expect("current").doc;
                assert!(
                    shares_no_plane(&live, current),
                    "seed {seed} step {step} op {op}: live shares a plane with history"
                );
            }
            assert!(
                history.full_state_count() <= history.anchor_count() + 1,
                "seed {seed} step {step}: the cursor's anchor must be hollow"
            );
            // Deltas keep one side each; every state must still rebuild
            // without moving the cursor, whichever way the walks went.
            for (i, expected) in reference.iter().enumerate() {
                let snap = history.materialize(i);
                assert!(
                    snap.doc == expected.doc && snap.selection == expected.selection,
                    "seed {seed} step {step}: state {i} rebuilds wrong"
                );
            }
        }
        for (i, expected) in reference.iter().enumerate() {
            let snap = history.jump(i).expect("state");
            assert!(snap.doc == expected.doc, "seed {seed}: jump {i} differs");
        }
    }
}

#[test]
fn undoing_a_reorder_of_equal_layers_reports_the_whole_document() {
    let (w, h) = (40u32, 30u32);
    let mut live = doc(w, h, 3);
    live.layers = pictura_core::LayerStack::from_layers(vec![
        model_layer("a", w, h, 1),
        model_layer("b", w, h, 90),
    ]);
    let mut selection: Option<Selection> = None;
    let mut target = LayerTarget::default();
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &target, "Open");
    let open = live.clone();

    assert!(live.layers.move_node(&[0], &[], 2).is_ok());
    history.capture_live(&mut live, &selection, &target, "Reorder");
    assert_eq!(history.anchor_count(), 2, "a reorder starts a new segment");

    let restored = history.undo_live(&mut live, &mut selection, &mut target);
    assert_eq!(restored, Some(Restored::Everywhere));
    assert!(live == open, "undo restores the original order");
}

fn layered_doc(count: usize) -> Document {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    let layers = (0..count)
        .map(|index| Layer {
            name: format!("layer {index}"),
            rect: pictura_core::PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            },
            ..Layer::default()
        })
        .collect();
    doc.layers = pictura_core::LayerStack::from_layers(layers);
    doc
}

#[test]
fn undo_restores_layer_target() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Open");
    history.capture_live(&mut live, &selection, &LayerTarget::only("0"), "Edit");
    let mut target = LayerTarget::only("0");
    let mut no_selection: Option<Selection> = None;
    history.undo_live(&mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("1"));
}

#[test]
fn undo_keeps_a_later_selection() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Open");
    history.capture_live(&mut live, &selection, &LayerTarget::only("0"), "Edit");
    let mut target = LayerTarget::only("2");
    let mut no_selection: Option<Selection> = None;
    history.undo_live(&mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("2"));
}

#[test]
fn redo_restores_post_operation_target() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Open");
    history.capture_live(&mut live, &selection, &LayerTarget::only("0"), "Edit");
    let mut target = LayerTarget::only("0");
    let mut no_selection: Option<Selection> = None;
    history.undo_live(&mut live, &mut no_selection, &mut target);
    history.redo_live(&mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("0"));
}

#[test]
fn jump_restores_target_across_multiple_states() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("0"), "Open");
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Edit 1");
    history.capture_live(&mut live, &selection, &LayerTarget::only("2"), "Edit 2");
    let mut target = LayerTarget::only("2");
    let mut no_selection: Option<Selection> = None;
    let _ = history.jump_live(0, &mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("0"));
    let _ = history.jump_live(2, &mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("2"));
}

#[test]
fn named_snapshot_restores_layer_target() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Open");
    history.add_snapshot("Named", snap(3), LayerTarget::only("1"));
    assert_eq!(history.snapshot_target(0), Some(LayerTarget::only("1")));
}

#[test]
fn undo_restores_structural_target() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("2"), "Open");
    live.layers =
        pictura_core::LayerStack::from_layers(live.layers.iter().take(2).cloned().collect());
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Delete");
    let mut target = LayerTarget::only("1");
    let mut no_selection: Option<Selection> = None;
    history.undo_live(&mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("2"));
}

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
        doc.layers[0].channels[0].data.as_mut_slice()[1 * 16 + 1] = 200;
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
            .as_mut_slice()[1 * 4 + 1] = 20;
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
