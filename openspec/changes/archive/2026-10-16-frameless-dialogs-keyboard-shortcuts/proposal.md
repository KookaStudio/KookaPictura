# Proposal: frameless-dialogs-keyboard-shortcuts

## Why

Issue #85. CS6 draws its own dialog chrome, but every Kooka dialog is a default
window-manager framed `QDialog`, so the dialogs read as foreign to the dark
workspace. `Edit > Keyboard Shortcuts…` and `Window > Workspace > Keyboard
Shortcuts & Menus…` are disabled stubs: there is no way to see or change a
shortcut. (Color Picker and Gradient Editor, the issue's other two dialogs,
already shipped.)

## What Changes

- Every Qt-drawn dialog SHALL be a frameless window with Kooka's own chrome: a
  title strip with the dialog title and a close button, a 1 px outline, drag to
  move, and edge-drag resize for resizable dialogs. One application-wide event
  filter applies it, so existing dialogs, `QMessageBox`, and `QInputDialog` gain
  it unchanged. A native or portal file picker keeps its system frame.
- Inline warning and info text in dialogs SHALL be dark text on yellow (a
  theme `notice` style with constant `noticeBg` / `noticeText` tokens).
- A Keyboard Shortcuts dialog SHALL list the application-menu commands (by
  menu) and the tools, laid out as CS6's. Selecting a row puts a focused key
  field in its Shortcut cell, so any binding can be remapped by pressing the
  new chord and accepting it. A command may hold several bindings (Add
  Shortcut / Delete Shortcut), Use Default restores them, and Undo steps back.
  A chord already in use is reported in the dialog, as CS6 does, with Accept
  and Go To Conflict and Undo Changes. OK applies the changes live and
  persists only the differences from the defaults in the session store;
  Cancel discards them.
- Tool shortcuts become overridable single letters. A letter still selects
  exactly one tool group, so taking another group's letter removes it from
  that group.

## Capabilities

### New Capabilities

- `ui/dialog-chrome`: the frameless chrome every Qt-drawn dialog wears.
- `ui/keyboard-shortcuts-editor`: the Keyboard Shortcuts dialog, conflict
  handling, and persisted shortcut overrides for menu commands and tools.

## Impact

- New `dialog_chrome.{h,cpp}`, `keyboard_shortcuts_dialog.{h,cpp}`; edits to
  `commands.{h,cpp}`, `tool_catalog.cpp`, `tools.h`, `toolbox.{h,cpp}`,
  `frame_build.cpp`, `frame_menus_edit.cpp`, `frame.{h,cpp}`,
  `frame_session.cpp`, `session.{h,cpp}`, `theme.cpp`, `main.cpp`,
  `CMakeLists.txt`.
- Tests: `tests/tst_dialog_chrome.cpp`, `tests/tst_keyboard_shortcuts.cpp`.
- No new dependency.

## Out of scope (ceilings)

Panel Menus shortcuts, named shortcut sets (`.kys`), Summarize, Use Legacy
Channel Shortcuts, and the Menus tab (`Edit > Menus…` stays a stub).

## Provenance

The shortcut editor is adapted from photorust's `KeyboardShortcutsDialog`
(GPL-3.0-or-later, https://github.com/perfecto25/photorust). The chrome is
independent: photorust's dialogs are framed. CS6 parity is behavioral only
(no oracle for dialog chrome or the shortcut editor).
