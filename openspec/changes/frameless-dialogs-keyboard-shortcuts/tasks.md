# Tasks: frameless-dialogs-keyboard-shortcuts

## 1. Dialog chrome

- [x] 1.1 `dialog_chrome.{h,cpp}`: app event filter, title bar, outline,
  margin shift, move, edge resize, native exemptions; theme rules.
- [x] 1.2 Install it in `main.cpp`.

## 2. Shortcut overrides

- [x] 2.1 `CommandRegistry` default/current binding lists, `setShortcuts`,
  `shortcutOverrides`.
- [x] 2.2 Tool letter overrides in the catalog; toolbox refresh; frame
  rebuilds per-letter shortcuts.
- [x] 2.3 Session store `shortcuts` / `toolShortcuts`; load on startup, save
  on OK.

## 3. Keyboard Shortcuts dialog

- [x] 3.1 `keyboard_shortcuts_dialog.{h,cpp}`: Application Menus / Tools,
  focused in-cell capture, Accept / Undo / Use Default / Add / Delete,
  several bindings per command, inline conflict notice; CS6 layout.
- [x] 3.2 Wire `Edit > Keyboard Shortcuts…` and `Window > Workspace >
  Keyboard Shortcuts & Menus…`.

## 4. Verification

- [x] 4.1 `tst_dialog_chrome`, `tst_keyboard_shortcuts`.
- [x] 4.2 `bash scripts/verify-full.sh`.
