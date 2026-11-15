# Spec Delta

## ADDED Requirements

### Requirement: Enter commits and deactivates a focused value input

Every keyboard-editable value input SHALL commit its current value on
Return/Enter and move focus out of the input. The value inputs are the shared
`NumericField`, `QAbstractSpinBox` controls, editable combo boxes (including
`QFontComboBox`), and text line edits inside dialogs. The input SHALL consume
the key so it does not reach the active tool, the active transformation, the
active path, or a dialog's default Apply/OK button. Invalid or empty input SHALL
revert to the previous valid value and SHALL still consume the key. A multi-line
text editor SHALL keep Enter for a new line, and a search, filter, or rename
field that is not a value input SHALL keep its own Enter handling.

#### Scenario: Enter commits a value field and leaves it [vie_commit]

- **WHEN** a numeric field, spin box, editable combo box, or dialog line edit has
  focus with a typed value and Enter is pressed
- **THEN** the value is committed once, the input loses focus, and the key does
  not reach the active tool or a dialog's default button

#### Scenario: Invalid input reverts and leaves [vie_invalid]

- **WHEN** an empty or invalid value is typed into a numeric field or spin box
  and Enter is pressed
- **THEN** the previous valid value is restored, no edit is committed, and the
  input loses focus

#### Scenario: Enter in a multi-line editor inserts a new line [vie_multiline]

- **WHEN** a multi-line text editor has focus in a dialog with a default button
  and Enter is pressed
- **THEN** a new line is inserted, the default button is not activated, and the
  editor keeps focus

### Requirement: Shift+Enter commits and keeps the value selected

Shift+Enter SHALL commit the current value of a focused value input, keep focus
in that input, and select its whole text so the value reads as highlighted.

#### Scenario: Shift+Enter commits and highlights [vie_shift_enter]

- **WHEN** a value is typed into a value input and Shift+Enter is pressed
- **THEN** the value is committed, the input keeps focus, and its text is
  selected

### Requirement: A dialog's default button waits for an unfocused field

A dialog SHALL activate its default Apply/OK button on Return/Enter only when no
value input has focus. While a value input has focus, Enter SHALL commit that
input and SHALL NOT press the default button.

#### Scenario: Enter in a dialog field does not press the default button [vie_dialog_default]

- **WHEN** a value field in a dialog has focus and Enter is pressed
- **THEN** the field commits, the dialog stays open, and the default button is
  not activated; a later Enter with no value input focused activates it

### Requirement: Tool commands ignore Enter while a value input is focused

The Crop and Perspective Crop commit, the Free Transform commit, and the path
finish SHALL run on Return/Enter only when no keyboard-editable value input has
focus. With a value input focused, Enter SHALL belong to the input.

#### Scenario: Enter in a field leaves the active tool or transform alone [vie_tool_guard]

- **WHEN** a crop box, Perspective Crop quad, Free Transform session, or path is
  active and a value input has focus, and Enter is pressed
- **THEN** the field commits and the tool or transformation stays active; a
  later Enter with no value input focused commits or finishes it as before
