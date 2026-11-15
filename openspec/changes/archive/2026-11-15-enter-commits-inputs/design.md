# Design

## One application filter for the value-input types created everywhere

The value inputs are not one class: `NumericField` owns a line edit, Qt spin
boxes and editable combo boxes own private line edits, and dialogs create plain
`QLineEdit`s in dozens of files. Wiring `returnPressed` at every creation site
would miss the next field. Instead one idempotent application event filter
(`installValueInputEnter()`) handles Return/Enter before the focused widget sees
it.

The filter maps the event's receiver to a field:

- an ancestor `QAbstractSpinBox` → `interpretText()`;
- an ancestor editable `QComboBox` (including `QFontComboBox`) → take the
  completer's current completion when its popup is open, emit the line edit's
  `editingFinished` so the owner's commit path runs, and for a font combo apply
  the typed family when it names an installed one (otherwise restore the current
  family);
- a `QLineEdit` whose window is a `QDialog` → emit `editingFinished`;
- a `QLineEdit` inside a `NumericField` is left to the field, which already owns
  its commit path.

After the commit the filter clears focus, or on Shift+Enter selects the value
and keeps focus. It accepts the event, so it never reaches a tool handler, the
canvas, or `QDialog`'s default-button path.

Multi-line editors are not matched (they are not line edits), so Enter keeps
inserting a new line. Search, filter, and rename line edits live in the main
window rather than a dialog and are not `NumericField`/spin/combo children, so
they keep their own Enter handling.

## NumericField consumes its own Enter

`NumericField::eventFilter` already handles the edit's keys. It now commits on
Enter (invalid or empty text restores the previous value in `commitEdit`) and
clears focus, or selects the value on Shift+Enter. The filter above skips a line
edit with a `NumericField` ancestor to keep one commit path.

## Frame and canvas guards

The frame handler is where a bubbled Enter landed and applied Crop,
Perspective Crop, or finished a path. Both Enter blocks now require
`!isTextInput(QApplication::focusWidget())`; the canvas's transform Enter
branch does the same. The existing `text_input.h` predicate already names line
edits, spin boxes, editable combos, and multi-line editors, and is shared with
the Delete/Tab gates.

The guards are defence in depth: a value input consumes Enter first, so the
frame and canvas normally never see it. They also cover a platform or control
that ignores Enter without the filter.

## Dialogs

`QDialog` activates its default button from `keyPressEvent`, which only runs
when the focused widget ignores the key. The filter accepts the key while a
value input has focus, so the default button waits for the next Enter, pressed
with no editor focused. This is the same rule for every dialog, including plain
name fields.

## Shift+Enter

CS6's Character/Paragraph fields apply on Enter, apply and highlight on
Shift+Enter, and apply then move on Tab
(`docs/02-ui-ux/panels/character-and-paragraph.md`). Shift+Enter therefore
commits and calls `selectAll()` on the focused line edit instead of clearing
focus. Tab is untouched: the field loses focus and Qt moves to the next one.
