# Spec Delta

## MODIFIED Requirements

### Requirement: Active layer row reflects the document

The system SHALL keep the document's layer target reflected in the panel. The
panel SHALL write the user's selection into the document target when the user
changes it, and SHALL use the current row as the active path when the current
row is part of that selection. When the current row is outside the selection,
the panel SHALL leave the target's active path unchanged. On open, on a
document switch, and on any refresh, the panel SHALL select the rows named by
the document target and SHALL NOT capture or overwrite that target; it SHALL
NOT fall back to the first displayed row. When the target's active path moves to another layer and that
layer's row is displayed, the panel's current row SHALL follow it, so that the
current row is the active layer. A current row the user moved outside the
selection SHALL survive a refresh that leaves the target unchanged while the row
is still displayed. An explicit deselect (Select > Deselect Layers, or a click
on empty space) SHALL clear the document target and SHALL NOT be undone by the
next refresh. A filter SHALL hide rows without changing the target; when the
filter is cleared, the target's visible rows SHALL be selected again. A
selection-based command SHALL use the target even while a selected row is
hidden by the filter. With several layers selected, every selected row SHALL
remain visually selected. A document whose only layer is the locked
`Background` SHALL report that row as active.

#### Scenario: The Background is active when it is the only layer [lpr_bg_active]

- **WHEN** a document whose only layer is the locked Background is opened or refreshed
- **THEN** the Background row is the selected and active row, and the document's
  active-layer path is its path

#### Scenario: The active layer is restored when it is not the first row [lpr_active_restore]

- **WHEN** a refresh rebuilds the model and the document's target is not the
  first displayed row
- **THEN** the panel selects the target's path rather than the first displayed
  row

#### Scenario: The current row follows the target's active path [lpr_active_follows]

- **WHEN** an operation moves the document target's active path to another
  layer and that layer's row is displayed
- **THEN** the panel's current row is the new active path

#### Scenario: A current row outside the selection survives a refresh [lpr_active_stray]

- **WHEN** the user moves the panel's current row outside a multi-selection
  whose active path is not the first selected path, and the panel refreshes
  while the document target stays the same and the row is still displayed
- **THEN** the current row stays on the row the user put it on and the
  document's active path stays on the previously active selected layer

#### Scenario: A document switch shows the new document's active row [lpr_switch_current]

- **WHEN** the user switches to another open document
- **THEN** the panel's selected rows are that document's target and its current
  row is that document's active path

#### Scenario: An explicit deselect is preserved [lpr_active_deselect]

- **WHEN** the user clears the layer selection, leaving the document with no
  active layer
- **THEN** the next refresh leaves the panel with no selected row

#### Scenario: A filter hiding the target does not clear it [lpr_filter_target]

- **WHEN** an active filter hides the row named by the document target
- **THEN** the document still reports that layer as selected and active, and
  clearing the filter selects its row again

#### Scenario: A command uses a filtered-out target [lpr_filter_command]

- **WHEN** the user runs a selection-based layer command while a selected row is
  hidden by the filter
- **THEN** the command acts on that hidden selected layer as part of the target

#### Scenario: A multi-selection is shown in full [lpr_multi_visual]

- **WHEN** the document target holds several selected layers
- **THEN** every visible selected row is shown as selected and the active path's
  row is the current row

### Requirement: Layer menu duplication follows the panel selection

`Layer ▸ Duplicate Layer…` SHALL duplicate every layer the document target
holds, addressing each layer by its path so a layer nested inside a group is
duplicated in place. The command SHALL be enabled only when the active document
has a non-empty layer target; with an empty target it SHALL be disabled and
SHALL change nothing. Each copy SHALL sit directly above its source in the same
container, and the whole operation SHALL be one undoable `"Duplicate Layer"`
step. Every copy SHALL become part of the document target, with one of them
active, and the panel SHALL show that selection.

#### Scenario: Duplicate a nested layer from the Layer menu [lpd_nested]

- **WHEN** a layer inside a group is selected and `Layer ▸ Duplicate Layer…` is
  run
- **THEN** a copy named `"<name> copy"` appears directly above the source inside
  the same group and exactly one history state is added

#### Scenario: Duplicate a multi-selection from the Layer menu [lpd_multi]

- **WHEN** two layers are selected and `Layer ▸ Duplicate Layer…` is run
- **THEN** both layers gain a copy directly above themselves, exactly one
  history state is added, and both copies are selected with one of them active

#### Scenario: Duplicate is disabled with no selection [lpd_empty]

- **WHEN** the document has no layer in its target
- **THEN** `Layer ▸ Duplicate Layer…` is disabled and running it leaves the rows
  and history unchanged

### Requirement: Solo visibility

The system SHALL implement `Alt`-click solo visibility on a row's visibility
toggle: the first `Alt`-click SHALL snapshot the visibility of every row and
then show only the clicked layer or group (with its ancestors, and for a group
its descendants that were visible in the snapshot), and a second `Alt`-click
SHALL restore the snapshot exactly, including a prior partial-hidden state. The
snapshot SHALL belong to the document it was taken in and SHALL NOT be
serialized with the document. Each direction SHALL be exactly one undoable
step, and the snapshot SHALL be discarded when a structural operation runs or
the document is replaced or closed. Switching to another document and back
SHALL leave the solo in place, with its snapshot still restorable.

#### Scenario: Alt-click solos a layer [m39_solo]

- **WHEN** the user `Alt`-clicks a layer's eye in a document with several visible layers
- **THEN** only that layer (and its ancestor groups) is visible, one undo state
  is added, and the prior visibility is remembered

#### Scenario: A second Alt-click restores exactly [m39_solo]

- **WHEN** the user `Alt`-clicks again while solo is active
- **THEN** every row's prior visibility is restored exactly, including layers
  that were hidden before the first `Alt`-click

#### Scenario: Solo survives undo via its history state [m39_solo]

- **WHEN** the user undoes a solo operation
- **THEN** the prior visibility of every layer is restored

#### Scenario: A solo belongs to its document [m39_solo_document]

- **WHEN** the user solos a layer, switches to another document, and switches
  back
- **THEN** the first document is still soloed and a second `Alt`-click restores
  its prior visibility

### Requirement: Group expansion survives a structural change

A group's expanded/collapsed state SHALL survive a structural change that shifts
the group's index-based path — adding, removing, or moving a layer, or dropping
a layer into the group. The panel SHALL re-find the group by its ancestor name
chain, disambiguating same-named siblings by their order among them, and keep it
expanded or collapsed as before. A rename, which leaves the index path intact,
SHALL keep the state through the unchanged path. A refused operation SHALL change
neither the document nor the expansion. The expanded groups SHALL belong to the
document they were expanded in: switching to another document SHALL NOT carry
the expansion across, and switching back SHALL restore that document's own
expanded groups.

#### Scenario: Adding a layer outside an expanded group keeps it open [lpr_add_keeps_expanded]

- **WHEN** a group is expanded and a layer is added elsewhere in the document so
  the group's index-based path shifts
- **THEN** the group stays expanded with its children visible

#### Scenario: A rename keeps the group expanded [lpr_rename_keeps_expanded]

- **WHEN** an expanded group is renamed in place
- **THEN** it stays expanded, because the rename leaves its index path intact

#### Scenario: Same-named sibling groups stay distinct [lpr_dup_group_names]

- **WHEN** two sibling groups share a name, one is expanded, and a structural
  change shifts their paths
- **THEN** the group that was expanded is the one that stays expanded

#### Scenario: A drop into an open group keeps it open [lpr_drop_into_expanded]

- **WHEN** an open group is a valid drop-into target and a layer is dropped onto it
- **THEN** the move succeeds as one undo step and the group stays expanded with
  the dropped layer visible as its child

#### Scenario: Switching documents does not carry the expansion [lpr_expansion_document]

- **WHEN** a group is expanded in one document and the user switches to another
  document
- **THEN** the other document's groups are not expanded, and switching back
  restores the first document's expanded group
