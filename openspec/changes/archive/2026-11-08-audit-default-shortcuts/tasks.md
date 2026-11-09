# Tasks

## 1. Tests

- [x] 1.1 `command_catalogue::tests::default_shortcuts_are_unique`.
- [x] 1.2 `command_catalogue::tests::cs6_function_key_panels`.
- [x] 1.3 `command_catalogue::tests::proof_colors_owns_ctrl_y` and
      `stamp_visible_owns_ctrl_alt_shift_e`.
- [x] 1.4 `tst_workspaces::functionKeysTogglePanels` (F6/F7/F8).
- [x] 1.5 `tst_edit_shortcuts::undoTogglesToRedo`.
- [x] 1.6 Update `tst_quick_mask_regressions::keyboardUndoRepaintsMask` to redo
      with `Ctrl+Shift+Z`.

## 2. Catalogue

- [x] 2.1 Bind `F6`/`F7`/`F8` to Color/Layers/Info.
- [x] 2.2 Remove `Ctrl+Y` from Redo and `Ctrl+Alt+Shift+E` from Export As.
- [x] 2.3 Update the five `tests/golden/catalogue.tsv` rows.

## 3. Spec

- [x] 3.1 Record the CS6 Undo/Redo contract in the `document/edit-history`
      delta.

## 4. Verification

- [x] 4.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 4.2 `cargo nextest run -p pictura-script`.
- [x] 4.3 CMake build and the affected Qt suites (`tst_workspaces`,
      `tst_edit_shortcuts`, `tst_quick_mask_regressions`).
- [x] 4.4 `openspec validate --all --strict`; archive the change.
