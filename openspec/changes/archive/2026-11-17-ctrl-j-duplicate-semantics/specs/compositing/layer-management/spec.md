# Spec Delta

## MODIFIED Requirements

### Requirement: Layer via Copy and Layer via Cut

The system SHALL provide `Layer via Copy` (`Ctrl+J`) and `Layer via Cut`
(`Shift+Ctrl+J`). The active layer target is the Layers panel's current row; a
multi-selection follows that row's kind. A selection that covers no pixels SHALL
count as no selection. `Layer via Cut` SHALL require an active selection and a
non-group target; with no selection or with a group target the command SHALL be
unavailable and SHALL leave the document unchanged.

`Layer via Copy` SHALL dispatch on the active layer target:

- when the target is a group, it SHALL duplicate the whole selection: each
  selected group SHALL be copied whole, including every descendant, and every
  other selected layer SHALL also be copied, whether or not a selection is
  active;
- when the target is not a group and no selection is active, it SHALL duplicate
  the selected layer(s);
- when the target is not a group and a selection is active, it SHALL create a
  new pixel layer directly above the target containing only the pixels inside
  the active selection, and SHALL preserve every pixel outside the selection.

`Layer via Cut` SHALL copy the selection pixels into a new pixel layer directly
above the target and SHALL additionally clear those pixels from the source
layer when the source carries an alpha channel, preserving every pixel outside
the selection. Every `Layer via Copy` variant and `Layer via Cut` SHALL
recomposite and record exactly one undo state; each duplication variant SHALL
leave the copies selected, with one of them active.

#### Scenario: Layer via Copy extracts the selection

- **WHEN** a non-group layer has an active selection and Layer via Copy runs
- **THEN** a new layer above it contains only the selected pixels, every pixel
  outside the selection is preserved on both layers, the source layer is
  otherwise unchanged, and exactly one undo state is recorded

#### Scenario: Layer via Cut clears the source

- **WHEN** a non-group layer that carries an alpha channel has an active
  selection and Layer via Cut runs
- **THEN** a new layer above it contains the selected pixels, those pixels are
  cleared to transparency in the source layer, every pixel outside the
  selection is preserved, and exactly one undo state is recorded

#### Scenario: Ctrl+J duplicates a selected group [lmb_via_copy_group]

- **WHEN** a group is the active layer target, alone or in a multi-selection,
  and Layer via Copy runs, with or without an active selection
- **THEN** a copy of the whole group appears directly above it with copies of
  every descendant, any other selected layer is copied too, the copies are
  selected with one of them active, the source group is unchanged, and exactly
  one undo state is recorded

#### Scenario: Ctrl+J duplicates with no selection [lmb_via_copy_no_selection]

- **WHEN** a non-group layer is the active layer target, no selection is
  active, and Layer via Copy runs
- **THEN** a copy of the selected layer(s) appears directly above each source,
  the copies are selected with one of them active, and exactly one undo state is
  recorded

#### Scenario: Shift+Ctrl+J is unavailable for a group [lmb_via_cut_group]

- **WHEN** a group is the active layer target with an active selection and
  Layer via Cut is invoked
- **THEN** the command is unavailable, no layer is added or changed, and no
  history state is recorded

#### Scenario: No selection is refused

- **WHEN** Layer via Cut is invoked with no active selection
- **THEN** the command is unavailable, no layer is added, and the document is
  unchanged
