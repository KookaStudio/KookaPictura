# Design: pen-gesture-undo

## Context

Pen edits are applied to the live document as they happen: a press appends an
anchor and a move drags handles, and the handler records one document history
state on release (`path_commit_anchor` / `path_commit_convert` /
`path_add_freeform`). An in-progress gesture therefore has no history state to
step back, and `PictureView::undo()` restores the previous committed state
instead. The handler kept no pre-press path, so the release after an undo
recorded a state for a gesture the document no longer held.

## Approach

Treat the live gesture as a modal tool session step, as the Crop tool does for
its session history:

- On press the handler captures a **gesture backup** through the bridge:
  `path_gesture_begin` stores `(ActivePath, VectorPath)` — the active path row
  and its content — in the view. This covers `begin_drawing` replacing the Work
  Path when no row was active.
- `PenToolHandler` overrides `toolUndo()` / `canToolUndo()`: with a live
  gesture it restores the backup (`path_gesture_abort`), clears its gesture
  state, refreshes the overlay, and returns true so Edit > Undo consumes the
  step; with no live gesture it returns false and the frame steps back the
  document history. `toolRedo()` stays false: an aborted gesture is not
  redoable.
- `path_gesture_end` drops the backup after every commit path (release,
  deactivate) so it never outlives the gesture.
- On release after an abort the handler sees `pressView_ == nullptr` and
  commits nothing.

## Alternatives

- Recording the press as a document state and letting undo remove it: violates
  "a drag is one state" and leaves an aborted step in the History panel.
- Committing the live gesture before undo and then undoing it: same problem,
  plus a visible history entry for an edit the user cancelled.
- A full document snapshot per press: the path is the only live mutation the
  gesture makes, so the path backup is sufficient and cheap.

## Verification

- `tst_pen_tools`: the committed Ctrl+Z walk, mid-gesture cancellation (anchor,
  first anchor, convert handle, freeform), and no state after release.
- `cargo nextest run --workspace`, `ctest -R '^tst_'`, the app self-test,
  `scripts/guard.sh`.
