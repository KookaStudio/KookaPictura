# Design: smart-guides-shortcut

The shortcut lives in the command table, so the menu action, the Edit >
Keyboard Shortcuts editor, and the control server all see the same binding.

`QKeySequence("Ctrl+Shift+;")` is portable: Qt maps `Qt::CTRL` to Command on
macOS, so the same declaration shows `Command+Shift+;` there. No platform
branch is needed, matching the existing `View > Show > Guides` (`Ctrl+;`) and
`View > Lock Guides` (`Alt+Ctrl+;`) bindings.

The test compares `QAction::shortcut()` with the parsed sequence. It runs on the
Linux CI, where the string round-trips exactly; the macOS rendering is Qt's
standard mapping.
