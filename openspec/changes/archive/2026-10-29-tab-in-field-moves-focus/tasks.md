# Tasks

## 1. App

- [x] 1.1 Disable the `Tab`/`Shift+Tab` panel shortcuts while a text input has focus (`frame.cpp`).
- [x] 1.2 Read the new focus widget in the Delete/Backspace gate (`frame_build.cpp`).
- [x] 1.3 Add `tst_edit_shortcuts::deleteKeyActsAfterAFieldLosesFocus`; add `tst_screen_modes::tabInFieldMovesFocus` and `tst_screen_modes::tabOnCanvasTogglesPanels`.

## 2. Gates

- [x] 2.1 Build and run `tst_screen_modes` and `tst_edit_shortcuts`.
- [x] 2.2 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 2.3 Archive with `openspec archive tab-in-field-moves-focus`.
