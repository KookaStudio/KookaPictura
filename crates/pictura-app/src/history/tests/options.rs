use super::*;

#[test]
fn nonlinear_capture_keeps_later_states() {
    let mut history = History::default();
    history.capture(snap(0), "Open");
    history.capture(snap(1), "A");
    history.capture(snap(2), "B");
    history.undo();
    history.set_non_linear(true);
    history.capture(snap(3), "C");

    assert_eq!(history.count(), 4);
    assert_eq!(history.label(2), "B");
    assert_eq!(history.label(3), "C");
    assert_eq!(history.index(), 3);
    assert_eq!(
        history.state_snapshot(2).unwrap().doc.composite.data,
        vec![2, 2, 2]
    );
}

#[test]
fn nonlinear_undo_follows_recording_order() {
    let mut history = History::default();
    history.capture(snap(0), "Open");
    history.capture(snap(1), "A");
    history.capture(snap(2), "B");
    history.undo();
    history.set_non_linear(true);
    history.capture(snap(3), "C");

    let previous = history.undo().expect("undo the appended state");
    assert_eq!(previous.doc.composite.data, vec![2, 2, 2]);
    let branch = history.undo().expect("undo to the branch point");
    assert_eq!(branch.doc.composite.data, vec![1, 1, 1]);
}

#[test]
fn materialize_middle_state_is_read_only() {
    let mut history = History::default();
    history.capture(snap(0), "Open");
    history.capture(snap(1), "A");
    history.capture(snap(2), "B");
    history.add_snapshot("Keep", snap(1), LayerTarget::default());
    let before_index = history.index();
    let before_count = history.count();
    let before_snapshots = history.snapshot_count();

    let state = history.state_snapshot(1).expect("state 1");
    assert_eq!(state.doc.composite.data, vec![1, 1, 1]);
    assert_eq!(history.index(), before_index);
    assert_eq!(history.count(), before_count);
    assert_eq!(history.snapshot_count(), before_snapshots);
    assert!(history.state_snapshot(99).is_none());
}

#[test]
fn clear_keeps_snapshots() {
    let mut history = History::default();
    history.capture(snap(0), "Open");
    history.capture(snap(1), "A");
    history.add_snapshot("Keep", snap(1), LayerTarget::default());

    history.purge_states();
    assert_eq!(history.count(), 1);
    assert_eq!(history.snapshot_count(), 1);
    assert!(!history.can_undo());
    assert_eq!(
        history.current_snapshot().unwrap().doc.composite.data,
        vec![1, 1, 1]
    );
}

#[test]
fn purge_drops_snapshots() {
    let mut history = History::default();
    history.capture(snap(0), "Open");
    history.capture(snap(1), "A");
    history.add_snapshot("Keep", snap(1), LayerTarget::default());

    history.purge();
    assert_eq!(history.count(), 1);
    assert_eq!(history.snapshot_count(), 0);
    assert!(!history.can_undo());
}

#[test]
fn nonlinear_jump_to_earlier_state_reverts_the_appended_stroke() {
    let mut history = History::default();
    let mut live = doc(4, 1, 7);
    let mut selection = None;
    let mut target = LayerTarget::default();
    let set_pixel = |live: &mut Document, i: usize, value: u8| {
        live.composite.edit_data_slice(|data| {
            for channel in 0..3 {
                data[i * 3 + channel] = value;
            }
        });
    };

    history.capture_live(&mut live, &selection, &target, "New");
    set_pixel(&mut live, 0, 10);
    history.capture_live(&mut live, &selection, &target, "Brush 1");
    set_pixel(&mut live, 1, 20);
    history.capture_live(&mut live, &selection, &target, "Brush 2");
    set_pixel(&mut live, 2, 30);
    history.capture_live(&mut live, &selection, &target, "Brush 3");

    history.jump_live(1, &mut live, &mut selection, &mut target);
    assert_eq!(&live.composite.data[0..3], &[10, 10, 10]);
    assert_eq!(&live.composite.data[3..6], &[7, 7, 7]);
    assert_eq!(&live.composite.data[9..12], &[7, 7, 7]);

    history.set_non_linear(true);
    set_pixel(&mut live, 3, 40);
    history.capture_live(&mut live, &selection, &target, "Brush 4");
    assert_eq!(history.count(), 5);

    history.jump_live(1, &mut live, &mut selection, &mut target);
    assert_eq!(&live.composite.data[0..3], &[10, 10, 10]);
    assert_eq!(&live.composite.data[3..6], &[7, 7, 7]);
    assert_eq!(&live.composite.data[6..9], &[7, 7, 7]);
    assert_eq!(&live.composite.data[9..12], &[7, 7, 7]);
}
