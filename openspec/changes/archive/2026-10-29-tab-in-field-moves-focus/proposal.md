# Proposal

Tracking issue: #1028.

## Why

Pressing `Tab` to commit a value typed into a Properties panel field applies
the value but also hides the Tools panel and every panel column. The frame
registers `Tab` and `Shift+Tab` as window shortcuts for Show/Hide Panels, and
a line edit or spin box does not claim `Tab` in its shortcut override, so the
shortcut wins over focus traversal. CS6 numeric fields treat `Tab` as "apply
and move to the next field" (`docs/02-ui-ux/panels/character-and-paragraph.md`).

## What Changes

- While an editable text widget (line edit, spin box, text edit, editable combo
  box) has focus, the `Tab` and `Shift+Tab` panel shortcuts are disabled, so the
  key reaches the widget: a single-line field commits on focus out and focus
  moves to the next or previous field; a multi-line text edit keeps Qt's default
  and inserts a tab. This holds in every screen mode.
- The shortcuts are gated the same way the Delete/Backspace shortcuts are,
  through `isTextInput` and the application's focus-changed signal.
- The Delete/Backspace gate read the focus-changed signal's arguments in the
  wrong order (Qt passes the old widget first), so it disabled the shortcuts on
  leaving a field rather than on entering one. The frame's key fallback and
  `QLineEdit`'s own shortcut override hid the effect; the gate now follows the
  newly focused widget.

Out of scope: Enter committing and leaving a field (#926, #834).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/workspace-persistence`: adds the rule that `Tab` in a focused text
  field moves focus instead of toggling the panels.
- `ui/delete-key`: adds the rule that the field gate follows the newly focused
  widget.

## Impact

- `crates/pictura-app/cpp/frame.cpp`: the Tab shortcuts follow the
  focused widget.
- `crates/pictura-app/cpp/frame_build.cpp`: the Delete gate reads the new
  focus widget.
- `crates/pictura-app/cpp/tests/tst_screen_modes.cpp`: two cases;
  `tst_edit_shortcuts.cpp`: one case.
