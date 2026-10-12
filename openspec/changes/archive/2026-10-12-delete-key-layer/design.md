# Design: delete-key-layer

## Context

Delete/Backspace were handled on the canvas/tool input path
(`ImageView::keyPressEvent` → `PicturaMainWindow::keyPressEvent` →
`dispatchCommandKey`), so the key only reached the frame when the focused
widget ignored it. The window's menu actions are `Qt::WindowShortcut` by
default, which is the existing focus-independent mechanism; the Type session
and the `runDialog` input blocker already gate it.

## Approach

- Add two window-level `QShortcut`s (Delete, Backspace) in `frame_build.cpp`,
  `setAutoRepeat(false)` so a held key cannot delete layer after layer, wired
  to a new `PicturaMainWindow::handleDeleteKey()` in `frame_menus_edit.cpp`.
- `handleDeleteKey()` owns the precedence: a type session, a focused editable
  text field, then the active tool's Delete hook (`removeLassoPoint`), then
  `Edit > Clear` when a pixel selection exists, else the Layers panel's
  `deleteSelection()` (the same route `Layer > Delete Layer` uses).
- A focused text field disables the shortcuts (`focusChanged`) so the key
  reaches the field rather than being swallowed by the shortcut map.
- `dispatchCommandKey` delegates its Delete branch to `handleDeleteKey`, and
  the frame's duplicate `removeLassoPoint` block is deleted, so real and
  synthetic key events share one implementation.

## Alternatives

- Binding Delete to the `Layer > Delete Layer` QAction: the menu would show a
  Delete shortcut CS6 does not show there, and the action's enabled state is
  panel-selection driven, not key-context driven.
- Keeping the canvas key path and adding more focus checks: the issue is
  precisely that focus location decides whether the key works.

## Verification

- `tst_edit_shortcuts`: deletion with the Layers panel focused, selection clear
  with a selection, and a focused text field keeping the key.
- Existing `tst_type_tools` (Backspace edits), `tst_path_selection_tools`
  (synthetic Delete), the Magnetic Lasso / slice self-tests, and
  `tst_command_tree` (Layer > Delete Layer) all stay green.
- `bash scripts/verify-full.sh`.
