use super::*;

#[test]
fn live_restores_match_a_full_snapshot_history() {
    let (w, h) = (70u32, 45u32);
    for seed in 0..12u64 {
        let mut rng = Rng(seed);
        let mut live = doc(w, h, 3);
        live.composite
            .replace_data((0..(w * h * 3) as usize).map(|i| (i % 251) as u8).collect())
            .expect("valid composite plane");
        live.layers = pictura_core::LayerStack::from_layers(vec![model_layer("base", w, h, 1)]);
        let mut selection: Option<Selection> = None;
        let mut target = LayerTarget::default();
        let mut history = History::default();
        history.capture_live(&mut live, &selection, &target, "Open");
        let mut reference = vec![Snapshot {
            quick_mask: None,
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
                if let Some(Restored::Region(r) | Restored::Composite(r)) = restored {
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
                                live.layers[li].channels_mut()[c].data[yy * w as usize + xx] = v;
                                let k = (c % 3) * h as usize + yy;
                                live.composite
                                    .edit_data(|data| {
                                        data[k * w as usize + xx] = v ^ 0x5A;
                                    })
                                    .expect("pixel edit preserves geometry");
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
                    8 if live.layers.len() > 1 && rng.below(2) == 0 => {
                        let from = rng.below(live.layers.len());
                        let to = rng.below(live.layers.len() + 1);
                        let _ = live.layers.move_node(&[from], &[], to);
                    }
                    _ => selection = Some(Selection::rect(w, h, 2, 3, 1 + rng.below(20) as i32, 4)),
                }
                history.capture_live(&mut live, &selection, &target, "Edit");
                reference.truncate(cursor + 1);
                reference.push(Snapshot {
                    quick_mask: None,
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
