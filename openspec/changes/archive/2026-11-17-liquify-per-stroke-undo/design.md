# Design

## Snapshots in the session

`LiquifyMesh` already clones and compares, so the history stores whole-mesh
snapshots: one `LiquifyMesh` carries both the displacement and the freeze mask.
`Session` gains a `History` (a bounded `VecDeque` past stack plus a future
stack) and a `stroke_base` snapshot. Snapshotting the whole mesh keeps the
change local to the session; the stack is bounded to CS6's default 20 History
States.

## One entry per action

- `liquify_stroke_begin` captures the mesh before the first dab. The first dab
  that changes anything (at begin, a later `stroke_to`, or a Brush Rate
  `hold`) commits that snapshot to the undo stack; later dabs of the same
  stroke are not recorded, so a drag is one undo step.
- Reconstruct, Restore All, Mask None/All/Invert, and a user's Load Last Mesh
  clone the mesh, apply, and record the clone only when the mesh changed. The
  dialog's initial Last Filter mesh is the baseline, not an action, so it
  clears the history instead of recording.
- Every record clears the redo stack, so a new action after undo discards the
  redo path.

## Undo and redo

`liquify_undo` and `liquify_redo` swap the top of the corresponding stack with
the live mesh and return whether anything changed. The dialog binds
`QKeySequence::Undo` and `QKeySequence::Redo` (Ctrl+Z and Ctrl+Shift+Z on
Linux) to them and calls `LiquifyCanvas::refreshAll` when they report a change.
The shortcuts live on the dialog, so they work wherever focus is inside it.

## Boundaries

The history belongs to the dialog's session. `liquify_finish` drops it with
the session: OK still produces one document history state through the ordinary
filter path, and Cancel discards everything. No document-history change is
needed.
