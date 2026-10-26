# Spec Delta — tools/type-color-live-preview

## ADDED Requirements

### Requirement: The Text Color picker previews live

The system SHALL show each color the user picks in the Text Color picker on the
selected type layer or the text being typed while the picker is still open, SHALL
record no history state for the preview, SHALL record one state when the picker is
accepted, and SHALL restore the original color with no state when it is cancelled.
This SHALL hold for both the options-bar swatch and the Character panel swatch.

#### Scenario: The picker reports each change [tclp_picker_live]

- **WHEN** the user changes the color in an open Color Picker dialog opened with a live callback
- **THEN** the callback receives that color before the dialog closes

#### Scenario: The bar previews a selected layer [tclp_bar_live]

- **WHEN** a type layer is selected and the user changes the color in the options-bar Text Color picker
- **THEN** the layer's color is the picked color while the picker is open
- **AND** the document history is unchanged

#### Scenario: OK commits one state [tclp_bar_ok]

- **WHEN** the user accepts the options-bar Text Color picker after previewing
- **THEN** the layer keeps the picked color
- **AND** exactly one "Edit Type Layer" state is recorded
- **AND** undoing it restores the color the layer had before the picker opened

#### Scenario: Cancel restores the color [tclp_bar_cancel]

- **WHEN** the user cancels the options-bar Text Color picker after previewing
- **THEN** the layer has its original color
- **AND** the document history is unchanged

#### Scenario: Text being typed previews and restores [tclp_session]

- **WHEN** text is being typed and the user changes the color in the options-bar picker, then cancels
- **THEN** the session's text color followed each picked color while it was open
- **AND** it is the original color after Cancel

#### Scenario: The Character panel previews and commits [tclp_panel]

- **WHEN** a type layer is selected and the user changes the color in the Character panel's Text Color picker
- **THEN** the layer shows the picked color with no history state while the picker is open
- **AND** accepting records one state, and cancelling restores the original color with none
