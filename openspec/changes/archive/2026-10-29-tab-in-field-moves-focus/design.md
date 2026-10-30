# Design

## Gate the shortcut, not the key

`QShortcut` matching runs before the focused widget sees the key. A field could
accept `ShortcutOverride` for `Tab` itself, but that would need doing in every
field class (`NumericField`, the Qt spin boxes, line edits in dialogs docked in
panels). Disabling the two frame shortcuts while `isTextInput(focusWidget)` is
true covers all of them in one place and matches the existing Delete/Backspace
gate in `frame_build.cpp`.

With the shortcut disabled, Qt's own handling applies: `QWidget::event` turns
`Tab`/`Backtab` into `focusNextPrevChild`, the field loses focus, and its
`editingFinished` commits the typed value. `QTextEdit`/`QPlainTextEdit` keep
their default of inserting a tab.

Each shortcut's focus-changed connection uses the shortcut as its context
object, so Qt drops the connection when the shortcut is destroyed and no handler
runs against a torn-down frame. This needs no stored connection on the frame.

## The Delete gate's argument order

`QApplication::focusChanged(old, now)` passes the old widget first. The Delete
gate took the first argument as the new widget, so it was inverted. No key
press shows it: the frame's `keyPressEvent` fallback calls `handleDeleteKey`
when the shortcut is wrongly off, and `QLineEdit` claims Delete in its shortcut
override when it is wrongly on. The test therefore asserts the shortcut's
enabled state as well as the deletion.
