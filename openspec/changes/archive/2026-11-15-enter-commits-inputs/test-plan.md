# Test Plan

One row per scenario in the delta spec. Every row is automated.

## ui/value-input-enter

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| vie_commit | In turn, focus a `NumericField`, a dialog `QSpinBox`, a dialog `QLineEdit`, the Character panel size combo, and the Glyphs panel family combo; type a value and press Enter. | The value is committed once, the input loses focus, and the key does not reach a tool or the dialog's default button. | `tst_value_input_enter::numericFieldEnterCommitsAndLeaves`, `tst_value_input_enter::spinBoxEnterCommitsAndLeaves`, `tst_value_input_enter::dialogLineEditEnterCommitsAndLeaves`, `tst_type_session_panels::enterInSizeFieldCommitsAndLeaves`, `tst_type_panels::glyphsFamilyEnterCommitsAndLeaves` |
| vie_invalid | Clear a numeric field and press Enter; type an out-of-range value into a dialog spin box and press Enter. | The previous valid value is restored, no commit is emitted, and focus leaves the input. | `tst_value_input_enter::numericFieldInvalidEnterRevertsAndLeaves`, `tst_value_input_enter::spinBoxInvalidEnterRevertsAndLeaves` |
| vie_multiline | Focus a `QPlainTextEdit` in a dialog with a default button, press Enter. | A newline is inserted, the default button is not activated, and the editor keeps focus. | `tst_value_input_enter::multiLineEditorKeepsEnterForNewline` |
| vie_shift_enter | Type a value into a `NumericField` and press Shift+Enter. | The value is committed, the field keeps focus, and its text is selected. | `tst_value_input_enter::numericFieldShiftEnterKeepsFocusAndHighlights` |
| vie_dialog_default | In a `FillDialog`, focus the Opacity spin box, type a value, press Enter; then focus the default OK button and press Enter. | The first Enter commits the value and leaves the dialog open; the second activates OK and accepts the dialog. | `tst_value_input_enter::dialogFieldEnterDoesNotPressDefaultButton` |
| vie_tool_guard | With an active Crop box, Perspective Crop quad, Free Transform, and Pen path, focus a value field and press Enter; then press Enter with no value input focused. | The field commits and the tool stays active; the second Enter commits or finishes it. | `tst_crop_options::enterInFieldKeepsCropActive`, `tst_perspective_crop::enterInFieldKeepsQuadStaged`, `tst_transform_options::enterInFieldKeepsTransformActive`, `tst_pen_tools::enterInFieldKeepsPathActive` |

## tools/canvas-tools

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| crop_enter_field_focus | Draw a Crop box, focus the options-bar Width field, type a value, press Enter; then press Enter with the canvas focused. | The field commits and the crop stays pending; the second Enter crops the document. | `tst_crop_options::enterInFieldKeepsCropActive` |

## tools/perspective-crop

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| perspective_crop_enter_field_focus | Stage a complete Perspective Crop quad, focus a value field, press Enter; then press Enter with the canvas focused. | The field commits and the quad stays staged; the second Enter commits the quad. | `tst_perspective_crop::enterInFieldKeepsQuadStaged` |

## tools/pen-tool

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| pen_path_enter_field_focus | Start a Pen subpath, focus a value field, press Enter; then press Enter with the canvas focused. | The field commits and the subpath stays open; the second Enter ends it open. | `tst_pen_tools::enterInFieldKeepsPathActive` |

## document/free-transform

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| transform_enter_field_focus | Begin Free Transform, focus a tool-options field, press Enter; then press Enter with the canvas focused. | The field commits and the session stays active; the second Enter commits one `Free Transform` state. | `tst_transform_options::enterInFieldKeepsTransformActive` |
