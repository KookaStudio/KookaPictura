# Tasks

## 1. Tests

- [x] 1.1 Add `tst_liquify_dialog::undoAndRedoStepThroughStrokes`: stroke A,
  stroke B, Ctrl+Z keeps A, Ctrl+Shift+Z restores B, and a new stroke after undo
  discards the redo path.
- [x] 1.2 Add `tst_liquify_dialog::undoRestoresTheFreezeMask`: Mask All, Invert
  All, and Freeze Mask painting each undo back to the previous mask.
- [x] 1.3 Add `tst_liquify_dialog::emptyUndoAndRedoAreNoOps`: both shortcuts
  leave the preview unchanged with no history.
- [x] 1.4 Add `pictura-app::history_undoes_and_redoes_one_action_at_a_time` and
  `pictura-app::history_keeps_a_bounded_undo_stack` for the session history.
- [x] 1.5 Write `test-plan.md` mapping every scenario to its automated test.

## 2. Session history

- [x] 2.1 Add the bounded `History` snapshot stack and the `stroke_base`
  snapshot to the Rust Liquify session.
- [x] 2.2 Record one undo entry per stroke, and one per whole-mesh action, only
  when the mesh changed; clear the redo stack on every new action.
- [x] 2.3 Expose `liquify_undo` and `liquify_redo` over the bridge.

## 3. Dialog

- [x] 3.1 Bind Ctrl+Z and Ctrl+Shift+Z in the Liquify dialog and refresh the
  preview after a change.

## 4. Gates

- [x] 4.1 Build and run `tst_liquify_dialog` and the `pictura-app` Liquify unit
  tests.
- [x] 4.2 `python3 scripts/check-test-plan.py`, `openspec validate --all
  --strict`.
- [x] 4.3 Archive with `openspec archive liquify-per-stroke-undo`.
