# delete-key Specification

## Purpose

Delete and Backspace act on the active layer or pixel selection from anywhere
in the workspace, while a type session, an editable field, an open dialog, or a
tool that consumes the key keeps it.

## ADDED Requirements

### Requirement: Delete key deletes the active layer

The system SHALL route Delete and Backspace through application-level shortcuts
on the main window, so the key acts regardless of which widget holds focus.
Unless a type-editing session is active, an editable text field has focus, or
an active tool consumes the key, Delete/Backspace SHALL clear the pixel
selection when one exists (as `Edit > Clear`) and otherwise delete the selected
layer(s) exactly as `Layer > Delete Layer` does. The key SHALL not auto-repeat
into repeated deletions, and an open dialog SHALL keep the key, as it does for
the window's other shortcuts.

#### Scenario: Deletion works away from the canvas [dk_delete_any_focus]

- **WHEN** a pixel layer is selected, the Layers panel holds focus, and Delete
  is pressed
- **THEN** the layer is deleted and one undo state is recorded

#### Scenario: A pixel selection is cleared instead [dk_selection_clear]

- **WHEN** a pixel selection exists and Delete or Backspace is pressed
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
