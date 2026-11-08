# Test Plan — audit-default-shortcuts

One row per scenario in the change's spec deltas. Rows name a test this change
creates unless marked `(existing)`.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `ds_function_key_panels` | Focus the main window, press F6, F7, F8 in turn and read each panel toggle action | The Color, Layers, and Info actions exist with F6/F7/F8 and their checked state flips on each press | `tst_workspaces::functionKeysTogglePanels`; `pictura-script::cs6_function_key_panels` |
| `ds_unique` | Collect the default shortcut of every catalogue row | No non-empty chord appears on two rows | `pictura-script::default_shortcuts_are_unique` |
| `ds_proof_colors` | Read `view.proof.colors` and `edit.redo` | Proof Colors is `Ctrl+Y`; Redo has no default | `pictura-script::proof_colors_owns_ctrl_y` |
| `ds_stamp_visible` | Read `layer.stamp.visible` and `file.exportAs` | Stamp Visible is `Ctrl+Alt+Shift+E`; Export As has no default | `pictura-script::stamp_visible_owns_ctrl_alt_shift_e` |
| Controls track availability | Empty stack, capture an op, undo, and read the Undo action | Disabled, then enabled, then enabled with Redo enabled | `tst_edit_refresh::aCommittedEditShowsUndoState` (existing) |
| `eh_undo_toggles` | New document, make one edit, press Ctrl+Z to the initial state, then press Ctrl+Z again | The second press restores the undone edit (redo) | `tst_edit_shortcuts::undoTogglesToRedo` |
