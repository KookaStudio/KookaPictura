# ui/delete-key Specification

## Purpose
Delete and Backspace act on the active layer or pixel selection from anywhere
in the workspace, while a type session, an editable field, an open dialog, or a
tool that consumes the key keeps it.

## Requirements

### Requirement: Delete key deletes the active layer

The system SHALL route Delete and Backspace through application-level shortcuts
on the main window, so the key acts regardless of which widget holds focus.
Unless a type-editing session is active, an editable text field has focus, or
an active tool consumes the key, Delete/Backspace SHALL act in this order: when a
raster layer mask is the current mask edit target, SHALL act on the mask — with a
pixel selection present, SHALL fill the selection's area of the mask with the
background colour (one `Fill` state, the mask's coverage set to the background's
luma, as `Edit > Fill` with the background colour does); with no pixel selection,
SHALL delete that mask without applying it (as `Delete Layer Mask`, with no
confirmation dialog). Otherwise, when a pixel selection exists, SHALL clear it (as
`Edit > Clear`); otherwise SHALL delete the selected layer(s) exactly as
`Layer > Delete Layer` does. The key SHALL
not auto-repeat into repeated deletions, and an open dialog SHALL keep the key, as
it does for the window's other shortcuts.

#### Scenario: Deletion works away from the canvas [dk_delete_any_focus]

- **WHEN** a pixel layer is selected, the Layers panel holds focus, and Delete
  is pressed
- **THEN** the layer is deleted and one undo state is recorded

#### Scenario: The active mask is deleted instead [dk_mask_target]

- **WHEN** a raster layer mask is the mask edit target, no pixel selection
  exists, and Delete or Backspace is pressed
- **THEN** the mask is removed (the layer's pixels are unchanged) and one undo
  state is recorded

#### Scenario: A selection on a mask is filled, not the mask deleted [dk_mask_fill]

- **WHEN** a raster layer mask is the mask edit target, a pixel selection exists,
  and Delete or Backspace is pressed
- **THEN** the selection's area of the mask is filled with the background
  colour's luma as one `Fill` state, the mask survives, and the layer's pixels are
  unchanged

#### Scenario: A pixel selection is cleared instead [dk_selection_clear]

- **WHEN** a pixel selection exists, no mask is the edit target, and Delete or
  Backspace is pressed
- **THEN** the selection's pixels are cleared with one `Clear` state and the
  layer itself survives

#### Scenario: A tool that consumes the key keeps it [dk_tool_consumes]

- **WHEN** the Magnetic Lasso has an active trace and Delete is pressed
- **THEN** the last fastening point is removed and no layer is deleted

#### Scenario: Type editing keeps the key [dk_type_session]

- **WHEN** a type session is open and Backspace is pressed
- **THEN** the text is edited and no layer is deleted

#### Scenario: An editable field keeps the key [dk_editable_focus]

- **WHEN** a numeric field in the options bar holds focus and Delete is pressed
- **THEN** no layer is deleted

#### Scenario: The Background is refused [dk_background_refused]

- **WHEN** the only selected layer is the Background and Delete is pressed
- **THEN** the document is unchanged and no history state is recorded

### Requirement: The field gate follows the newly focused widget

The Delete and Backspace shortcuts SHALL be disabled exactly while an editable
text field holds focus, judged by the widget that receives focus: moving focus
into a field SHALL disable them, and moving focus out of a field to a widget
that is not a field SHALL enable them again.

#### Scenario: Leaving a field gives Delete back to the workspace [dk_after_field]

- **WHEN** a text field has focus, focus then moves to a widget that is not a
  field, a pixel layer is selected, and Delete is pressed
- **THEN** the shortcuts were disabled while the field had focus and enabled
  after it lost focus, and the layer is deleted with one undo state
