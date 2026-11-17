# Test Plan

One row per scenario in the delta spec. Every row is automated.

## imaging/liquify-history

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lq_undo_stroke` | Draw stroke A, then B; press Ctrl+Z; press Ctrl+Shift+Z; undo again and draw C, then press Ctrl+Shift+Z. | Ctrl+Z restores the preview after A; redo restores B; after C the redo path is gone. | `tst_liquify_dialog::undoAndRedoStepThroughStrokes` |
| `lq_undo_mask` | Show Mask, apply Mask All, Invert All, and paint with the Freeze Mask tool, undoing after each. | The preview returns to its unfrozen state after each undo. | `tst_liquify_dialog::undoRestoresTheFreezeMask` |
| `lq_undo_empty` | Open Liquify with no actions and press Ctrl+Z and Ctrl+Shift+Z; reopen with Last Filter Settings and press Ctrl+Z; then draw and accept. | The preview and document stay unchanged, the initial mesh is not undone; OK adds one history state. | `tst_liquify_dialog::emptyUndoAndRedoAreNoOps`, `tst_liquify_dialog::lastFilterSettingsReopensWithTheMesh`, `tst_liquify_dialog::okCommitsOneFilterState` |
| `lq_undo_bounded` | Record more snapshots than the history limit, then undo until it refuses. | Exactly the limit's most recent actions undo, then undo is a no-op. | `pictura-app::history_keeps_a_bounded_undo_stack` |
