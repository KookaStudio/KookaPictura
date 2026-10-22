# document/layer-target Specification

## Purpose
Gives every open document one authoritative layer target — the set of selected
layers and the active layer within it — so tools, commands, filters, and undo
agree on which layers an operation affects.

## Requirements

### Requirement: One authoritative layer target per document

The application SHALL maintain exactly one layer target per open document,
consisting of the selected layer paths and one active path. The active path SHALL
be one of the selected paths, or empty when nothing is selected. Every selected
path SHALL name a layer in that document; a path that no longer resolves SHALL be
dropped. When the active path is dropped but other selected paths survive, the
active path SHALL become the first surviving selected path in document order.
A document switch SHALL restore the target belonging to that document. Changing
the active path SHALL clear a mask edit target that named the previous active
path. Changing the layer target SHALL NOT mark the document modified and SHALL
NOT add a history state. A tool or command that selects a layer — including Move
Auto-Select — SHALL write the target regardless of whether the Layers panel
currently displays that row.

#### Scenario: Selecting one layer sets the target [lt_single]

- **WHEN** the user selects one layer in the Layers panel
- **THEN** that path is both the only selected path and the active path

#### Scenario: A multi-selection keeps every selected path [lt_multi]

- **WHEN** the user selects two layers with Ctrl/Cmd-click
- **THEN** both paths are selected, the last-clicked path is the active path, and
  a mask edit target on the previous active layer is cleared

#### Scenario: An explicit deselect clears the target [lt_deselect]

- **WHEN** the user runs Select > Deselect Layers or clicks empty panel space
- **THEN** the selected set and the active path are both empty

#### Scenario: A removed layer is dropped from the target [lt_stale]

- **WHEN** the only selected layer is removed from the document
- **THEN** the selected set and the active path are both empty

#### Scenario: A stale active path falls back to a survivor [lt_stale_anchor]

- **WHEN** the active layer is removed while other selected layers survive
- **THEN** the first surviving selected path becomes the active path

#### Scenario: A target change is not an edit [lt_no_history]

- **WHEN** the user selects, extends, or clears the layer selection
- **THEN** the document is not marked modified and the history count is unchanged

#### Scenario: Auto-Select reaches a filtered-out layer [lt_autoselect_hidden]

- **WHEN** Move Auto-Select hits a layer whose row is hidden by an active filter
- **THEN** that layer becomes the target and Move acts on it, not on the
  previously active layer

### Requirement: The tool edit target is derived from the selection

The application SHALL derive one tool-edit target only when exactly one layer is
selected. With zero or several layers selected, the tool-edit target SHALL be
empty. Structural and property commands SHALL use the whole selected set, not
only the active path. The active path alone SHALL NOT enable a tool edit while
other layers are selected.

#### Scenario: One selected layer is the edit target [lt_edit_single]

- **WHEN** exactly one layer is selected
- **THEN** it is the tool-edit target

#### Scenario: A multi-selection has no single edit target [lt_edit_multi]

- **WHEN** two layers are selected, with one of them active
- **THEN** no single tool-edit target exists, while the selected set still has
  both layers

### Requirement: Structural operations keep the target on the intended layers

A successful structural layer operation SHALL leave the target on the layers the
user now expects to work with:

- New Layer, New Group, Paste, Place, type creation, shape creation, and New
  Smart Object via Copy SHALL select and activate the created node.
- Duplicate SHALL select the created copies and activate one of them; an
  Alt-drag copy SHALL select and activate the copy. Duplicating several layers
  SHALL select every copy and SHALL NOT leave a layer that was not copied
  selected.
- An adjustment layer created from the Adjustments panel icon grid, the panel
  menu, or the `Layer` menu SHALL select and activate the created layer.
- Group Layers SHALL select and activate the new group.
- Ungroup Layers SHALL select the spliced-out children. A selected layer outside
  the ungrouped groups SHALL remain selected at its new path. The active path
  SHALL become the first spliced child of the topmost ungrouped group, or the
  first surviving selected path when no child is selected.
- Delete SHALL select and activate the nearest surviving layer to the topmost
  deleted layer: the layer immediately below the deleted range in its container,
  then the layer immediately above, then the topmost remaining layer in the
  document. With no layers left, the target SHALL be empty.
- Move Up/Down, Arrange, and Reverse SHALL keep the affected layers selected at
  their new paths.
- Background From Layer SHALL keep the converted layer selected at its new
  bottom path.
- Merge Down, Merge Layers, and Merge Visible SHALL select and activate the
  merged result. Flatten Image SHALL select and activate the sole result.
- A layer copied from another open document SHALL select and activate the copy
  in the destination document.
- A refused operation SHALL leave the target unchanged.

#### Scenario: Created nodes become the target [lt_op_create]

- **WHEN** the user creates a layer or group, pastes, places, creates type or a
  shape, or makes a smart object via copy
- **THEN** the created node is selected and active

#### Scenario: A duplicate becomes the target [lt_op_duplicate]

- **WHEN** the user duplicates a layer or an Alt-drag creates a copy
- **THEN** the copy is selected and active

#### Scenario: Every copy of a multi-selection is selected [lt_op_duplicate_many]

- **WHEN** the user duplicates two or more selected layers
- **THEN** every copy is selected, one of them is active, and no layer that was
  not copied is selected

#### Scenario: An adjustment layer becomes the target [lt_op_adjustment]

- **WHEN** an adjustment layer is created from the Adjustments panel, the panel
  menu, or the `Layer` menu
- **THEN** the created layer is the only selected layer and is active

#### Scenario: Delete selects a deterministic survivor [lt_op_delete]

- **WHEN** the user deletes one or more selected layers and a layer remains
- **THEN** the nearest surviving layer to the topmost deleted layer is selected
  and active

#### Scenario: Group and Ungroup target their results [lt_op_group]

- **WHEN** Group Layers or Ungroup Layers succeeds
- **THEN** the new group or the spliced-out children are selected, with one of
  them active, and selected non-groups outside an ungrouped group remain selected

#### Scenario: Merge and Flatten select the result [lt_op_merge]

- **WHEN** Merge Down, Merge Layers, Merge Visible, or Flatten Image succeeds
- **THEN** the resulting layer is the only selected layer and is active

#### Scenario: A reorder keeps the moved layer selected [lt_op_reorder]

- **WHEN** the selected layer is moved up, down, arranged, or reversed
- **THEN** the same layer remains the target at its new path

#### Scenario: Background From Layer keeps the layer selected [lt_op_background]

- **WHEN** Layer from Background converts a layer and moves it to the bottom
- **THEN** the converted layer remains selected and active at its new path

#### Scenario: A cross-document copy becomes the target [lt_op_cross_document]

- **WHEN** a layer dragged from another open document is copied into the
  destination document
- **THEN** the copy is selected and active in the destination document

#### Scenario: A refused operation leaves the target alone [lt_op_refused]

- **WHEN** a structural operation is refused
- **THEN** the selected set and the active path are unchanged

### Requirement: Undo and redo preserve the layer target

Undo and redo SHALL restore the layer target captured with the restored document
state when the live target still equals the target of the state being left. A
structural undo under that rule SHALL leave the target on the restored layers,
not on whatever layer now occupies the old path. When the user has changed the
layer selection after the captured state, undo and redo SHALL keep that later
selection and SHALL drop paths that no longer resolve. Restoring a named history
snapshot SHALL restore the target captured with that snapshot.

The opening history state SHALL carry the target selected on open, so undoing the
first edit after opening SHALL keep that target.

#### Scenario: Undo of a structural edit keeps the layer [lt_undo_structural]

- **WHEN** the user undoes a delete, duplicate, group, or merge without changing
  the layer selection first
- **THEN** the target names the restored layer that the operation affected, not
  whatever layer now occupies the old path

#### Scenario: Undo of a pixel edit does not change the selection [lt_undo_pixel]

- **WHEN** the user paints on a layer, selects another layer, and then undoes the
  paint
- **THEN** the layer selected after the paint remains selected

#### Scenario: Undo after open keeps the opening target [lt_undo_first_edit]

- **WHEN** the user opens a document, paints once, and undoes that paint
- **THEN** the layer selected on open is still selected and active

#### Scenario: Redo restores the post-operation target [lt_redo]

- **WHEN** the user redoes an undone structural operation
- **THEN** the target follows the operation's result again

#### Scenario: A named snapshot restores its target [lt_undo_named_snapshot]

- **WHEN** the user restores a named history snapshot
- **THEN** the layer target captured with that snapshot is restored

### Requirement: Open, new, and document switch initialize the target

Opening a document SHALL select and activate its topmost pixel layer. When the
document has no pixel layer, it SHALL select and activate the topmost top-level
layer; when it has no layers, the target SHALL be empty. Creating a document
SHALL do the same for its initial layer. Switching documents SHALL restore the
target of the document being activated rather than carrying the previous
document's paths across.

#### Scenario: Open selects the topmost pixel layer [lt_open]

- **WHEN** a document is opened
- **THEN** its topmost pixel layer is the only selected layer and is active

#### Scenario: Open falls back to the topmost layer without a pixel layer [lt_open_no_pixel]

- **WHEN** a document containing only groups or adjustment layers is opened
- **THEN** its topmost top-level layer is the only selected layer and is active

#### Scenario: Switching documents restores each target [lt_switch]

- **WHEN** the user switches from one open document to another and back
- **THEN** each document shows the target it had before the switch

#### Scenario: A new document selects its first layer [lt_new]

- **WHEN** a new document is created
- **THEN** its initial layer is the only selected layer and is active

### Requirement: The engine owns the layer target and the user interface projects it

Every structural operation SHALL decide the layer target it leaves in the
engine. A command that changes the selection — Select All Layers, Select
Similar, Select Linked Layers, Deselect Layers — SHALL leave the target in the
engine as well. Select Similar SHALL select the active layer together with
every matching layer, and Select Linked Layers SHALL select every member of the
active layer's link set; in both, the layer the command ran on SHALL remain the
active path. Select Linked Layers on a layer with no link set SHALL leave the
target unchanged. After a structural operation the command handler SHALL refresh
the view and SHALL NOT rewrite the target the operation left. The panel's own
row gesture SHALL write the target, because the user is picking the rows there.
A control whose enablement depends on a selection SHALL read the target rather
than the rows the panel happens to display. The projection SHALL be the panel's
displayed selected rows and its current row: the current row SHALL be the
target's active path whenever the target moves and that row is displayed, and a
current row the user moved outside the selection SHALL survive a refresh while
the target stays the same and the row is still displayed.

#### Scenario: An operation leaves the target to the engine [lt_ui_projection]

- **WHEN** the user runs Duplicate on two selected layers
- **THEN** both copies are selected, one of them is active, and the panel
  shows that selection

#### Scenario: A refresh does not rewrite the target [lt_ui_no_rewrite]

- **WHEN** a structural command runs and the panel refreshes afterwards
- **THEN** the target is the one the engine set, unchanged by the refresh

#### Scenario: The current row follows the target [lt_ui_current_row]

- **WHEN** an operation moves the target's active path to another layer and
  that layer's row is displayed
- **THEN** the panel's current row is the new active path

#### Scenario: A current row the user moved survives a refresh [lt_ui_stray_row]

- **WHEN** the user moves the panel's current row outside a multi-selection
  whose active path is not the first selected path, and the panel refreshes
  while the target stays the same and the row is still displayed
- **THEN** the current row stays on the row the user put it on and the target's
  active path is unchanged

#### Scenario: Select Similar keeps the active layer [lt_op_select_similar]

- **WHEN** Select Similar runs on a layer
- **THEN** the active layer and every matching layer are selected, and the layer
  the command ran on is active

#### Scenario: Select Linked selects the link set [lt_op_select_linked]

- **WHEN** Select Linked Layers runs on a linked layer
- **THEN** every member of its link set is selected, and the layer the command
  ran on is active

#### Scenario: Select Linked leaves an unlinked target alone [lt_op_select_linked_unlinked]

- **WHEN** Select Linked Layers runs on a layer with no link set
- **THEN** the selected set and the active path are unchanged

### Requirement: The engine names the layer target of every structural operation

Each structural operation SHALL compute the target it leaves as part of the
operation in the engine, not in the caller. The target rules for duplicate,
merge, merge visible, group and ungroup SHALL be reachable without a user
interface so that the engine's own tests can check them. A duplicate SHALL
report every copy it made at the copy's final path, including a copy whose
ancestor container moved, and the target it leaves SHALL name those copies and
not the layers they were copied from.

#### Scenario: The duplicate target names the copies [lt_op_target_copies]

- **WHEN** two layers are duplicated
- **THEN** the target names both copies at their final paths, and the active
  path is one of them

#### Scenario: The engine can be asked for the target of an operation [lt_op_target_engine]

- **WHEN** the engine runs a duplicate, a merge, a merge of the visible layers,
  a group or an ungroup
- **THEN** the target it leaves is the one that operation's rule names, without
  a user interface being involved

#### Scenario: A duplicate in different containers reports every copy [lt_op_duplicate_nested]

- **WHEN** layers in different containers are duplicated together, including a
  source whose ancestor container a later copy shifts
- **THEN** every copy is reported at its final path and the target names every
  copy
