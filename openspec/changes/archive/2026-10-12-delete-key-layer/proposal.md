# Proposal: delete-key-layer

## Why

Issue #304: with a layer selected, Delete does nothing unless the canvas holds
focus, because the key is handled on the canvas/tool input path
(`ImageView::keyPressEvent` → `PicturaMainWindow::keyPressEvent` →
`dispatchCommandKey`). Clicking the Layers panel or the toolbox and pressing
Delete leaves the layer untouched, while CS6 deletes the active layer from
anywhere in the workspace.

## What Changes

- Delete and Backspace SHALL be routed through application-level shortcuts on
  the main window, so focus location is irrelevant.
- The shortcut SHALL keep CS6's precedence: a type-editing session or an
  editable text field keeps the key; an active tool that consumes Delete (a
  lasso trace, a selected slice or path component) keeps the key; with a pixel
  selection the key clears the selection (`Edit > Clear`, #186); otherwise it
  deletes the selected layer(s) exactly as `Layer > Delete Layer` does.
- An open dialog SHALL keep blocking the key, as it already does for the
  window's other shortcuts.

## Capabilities

### New Capabilities

- `ui/delete-key`: the Delete/Backspace key's focus-independent, contextual
  routing (selection clear, tool hook, or active-layer deletion).

## Impact

- `crates/pictura-app/cpp/frame_build.cpp` (window shortcuts),
  `frame_menus_edit.cpp` + `frame.h` (`handleDeleteKey`), `frame.cpp`
  (drop the duplicate tool-hook block), `tool_type.cpp` (shared text-input
  predicate), new `text_input.h`.
- Tests: `crates/pictura-app/cpp/tests/tst_edit_shortcuts.cpp`.
- No new dependency.

## Provenance

Independent fix; CS6 parity is behavioral only (no key-event oracle).
