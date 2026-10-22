## ADDED Requirements

### Requirement: Character panel edits new type while it is set

While a Horizontal or Vertical Type session is setting new text (no layer
exists yet), the Character panel SHALL show and edit that text's character
attributes. Each change SHALL re-render the on-canvas preview at once, record
no history state, and be carried by the layer the commit creates. The layer
selected before the session SHALL NOT change.

#### Scenario: Toggles preview on new text and land on its layer [cpn_new_text]

- **WHEN** `tst_character_paragraph_panels::characterPanelEditsNewText` selects
  a type layer, opens new type elsewhere, clicks Faux Bold and Underline in the
  Character panel, and commits
- **THEN** each click changes the preview, the new layer has Faux Bold and
  Underline, and the previously selected layer has neither
