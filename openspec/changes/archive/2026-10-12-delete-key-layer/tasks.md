# Tasks: delete-key-layer

## 1. Key routing

- [x] 1.1 Shared `isTextInput` predicate (`text_input.h`), used by the Type key
  filter and the frame.
- [x] 1.2 `PicturaMainWindow::handleDeleteKey()` with the CS6 precedence;
  `dispatchCommandKey` delegates to it; the frame's duplicate tool-hook block
  is removed.
- [x] 1.3 Window-level Delete/Backspace shortcuts (no auto-repeat), disabled
  while an editable field has focus.

## 2. Verification

- [x] 2.1 `tst_edit_shortcuts` cases; full `ctest -R '^tst_'` and the app
  self-test.
- [x] 2.2 `bash scripts/verify-full.sh`.
