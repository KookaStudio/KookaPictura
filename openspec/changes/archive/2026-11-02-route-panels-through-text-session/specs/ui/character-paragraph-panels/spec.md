## ADDED Requirements

### Requirement: The panels edit the open type session

While any Type or Type Mask session is open, whether setting new text or
reopening a type layer, the Character and Paragraph panels SHALL show and edit
that session's attributes instead of any layer's. This SHALL include the panel
menus' Reset Character, Reset Paragraph and Justification actions, which SHALL
be enabled during a session with no type layer selected. Each change SHALL
re-render the on-canvas preview at once and record no history state. The commit
SHALL carry the changes and cancelling SHALL discard them. No layer SHALL change
before the commit, and a layer selected but not being edited SHALL NOT change at
all. After an options bar change during a session, a panel edit SHALL keep the
options bar's value.

#### Scenario: Character edits land on a reopened layer [tsp_character]

- **WHEN** `tst_type_session_panels::reopenedCharacterEditsSession` reopens a
  type layer, changes size, anti-aliasing, family and Faux Bold in the Character
  panel, and commits
- **THEN** each change alters the preview, history does not move until the
  commit, and one state carries every change

#### Scenario: A colour pick edits a reopened layer [tsp_color]

- **WHEN** `tst_type_color_preview::characterPanelColorEditsReopenedSession`
  cancels one Text Color pick in a reopened session, accepts another, and commits
- **THEN** cancelling restores the session's colour, and one state carries the
  accepted colour

#### Scenario: Cancelling discards panel edits [tsp_cancel]

- **WHEN** `tst_type_session_panels::reopenedCharacterCancelKeepsLayer` edits
  both panels in a reopened session and cancels
- **THEN** the layer is unchanged and no state is recorded

#### Scenario: Paragraph edits reach new text [tsp_para_new]

- **WHEN** `tst_type_session_panels::paragraphEditsNewText` selects a type layer,
  opens new text elsewhere, sets a justification and a first-line indent, and
  commits
- **THEN** the justification changes the preview, the new layer has both, and the
  selected layer has neither

#### Scenario: Paragraph edits land on a reopened layer [tsp_para_reopen]

- **WHEN** `tst_type_session_panels::paragraphEditsReopenedLayer` sets an indent
  and a justification in a reopened session and commits
- **THEN** the preview changes, history does not move until the commit, and one
  state carries both

#### Scenario: The panels edit a Type Mask session [tsp_mask]

- **WHEN** `tst_type_session_panels::panelsEditMaskSession` selects a type layer,
  raises the size and sets a justification in a Type Mask session, and commits
- **THEN** the selection is larger than the same text's at its old size, and the
  selected layer is unchanged

#### Scenario: Menu actions edit the session [tsp_menu]

- **WHEN** `tst_type_session_panels::menuActionsEditSession` opens a session with
  no type layer, then uses Justification, Reset Character and Reset Paragraph
- **THEN** the items are enabled, the session's attributes change, and no state
  is recorded until the commit

#### Scenario: The options bar's value survives a panel edit [tsp_bar]

- **WHEN** `tst_type_session_panels::optionsBarEditSurvivesPanelEdit` changes size
  and justification in the options bar, edits both panels, and commits
- **THEN** the layer has the bar's size and justification and the panel edits
