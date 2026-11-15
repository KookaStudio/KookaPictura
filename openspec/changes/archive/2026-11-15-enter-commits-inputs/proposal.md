# Proposal

Tracking issue: #1324. Closes #926 and #834.

## Why

Enter is overloaded. When a value input has focus, Enter should finish that
edit; today it can reach the active tool and apply it instead.

- `PicturaMainWindow::keyPressEvent` commits Crop / Perspective Crop and
  finishes a path on Return/Enter with no focused-editor check
  (`crates/pictura-app/cpp/frame_keys.cpp`).
- `ImageView::keyPressEvent` commits an active Free Transform on Return/Enter
  with no focused-editor check (`crates/pictura-app/cpp/image_view.cpp`).
- `NumericField` connects only `editingFinished`; it has no `returnPressed`
  path. When an invalid value makes the line edit ignore Enter, the key bubbles
  to the window handler, and the field keeps focus.
- Many dialogs call `ok->setDefault(true)`, so Enter in a field can activate the
  default Apply/OK button.

## What Changes

- A shared application filter gives every keyboard-editable value input one
  Enter policy: commit the current value, move focus out, and consume the key.
  It covers `NumericField`, `QAbstractSpinBox`, editable combo boxes (including
  `QFontComboBox`), and text line edits inside dialogs.
- Shift+Enter commits and keeps focus with the value selected; invalid or empty
  input reverts to the previous valid value; a multi-line text editor keeps
  Enter for a new line.
- The frame and canvas Enter handlers run only when no value input has focus, so
  Enter with a field focused no longer commits Crop, Perspective Crop, Free
  Transform, or a path.
- A dialog's default Apply/OK button is activated by Enter only when no value
  input has focus.
- Search, filter, and rename fields that are not value inputs keep their own
  Enter handling.

## Capabilities

### New Capabilities

- `ui/value-input-enter`: the shared Enter/Shift+Enter policy for value inputs
  and the dialog default-button rule.

### Modified Capabilities

- `tools/canvas-tools`: the Crop tool commits on Enter only when no value input
  has focus.
- `tools/perspective-crop`: the Perspective Crop tool commits on Enter only when
  no value input has focus.
- `tools/pen-tool`: the Pen tool finishes a path on Enter only when no value
  input has focus.
- `document/free-transform`: the Free Transform session commits on Enter only
  when no value input has focus.

## Impact

- New `crates/pictura-app/cpp/value_input_enter.{h,cpp}` installed at startup
  and from `PicturaMainWindow`.
- `crates/pictura-app/cpp/panels/numeric_field.cpp`: Enter/Shift+Enter in the
  field.
- `crates/pictura-app/cpp/frame_keys.cpp` and `image_view.cpp`: focused-editor
  guards.
- Tests: a new `tst_value_input_enter` suite plus cases in `tst_crop_options`,
  `tst_perspective_crop`, `tst_transform_options`, `tst_pen_tools`,
  `tst_type_session_panels`, and `tst_type_panels`.
