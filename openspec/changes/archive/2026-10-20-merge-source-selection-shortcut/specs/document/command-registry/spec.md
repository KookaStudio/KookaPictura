# Spec Delta

## ADDED Requirements

### Requirement: Live enablement for selection-dependent commands

The system SHALL refresh a selection-dependent command's label and enabled state
when the Layers panel selection changes, without requiring the command's menu to
be opened first. This SHALL keep the command's `QAction` shortcut consistent with
the current selection.

#### Scenario: Ctrl+E follows an extended selection

- **WHEN** the user selects the Background layer alone and then extends the
  selection to a layer that can merge, without opening the Layer menu
- **THEN** the merge command becomes enabled and `Ctrl+E` performs the merge for
  the current selection
