# Spec Delta

## MODIFIED Requirements

### Requirement: Layer drag and drop

A layer row SHALL be draggable. Starting a drag SHALL NOT extend the selection
or begin a rubber-band selection. The row model SHALL implement the drag-and-drop
capability virtuals — `mimeTypes()`, `canDropMimeData()`, and
`supportedDropActions()` — so the view's `canDrop()` is true and a real drop
indicator position is computed, and the view SHALL enter the dragging state on
drag enter so the indicator paints. **The model SHALL return
`Qt::ItemIsDropEnabled` for the invalid parent index**, so a top-level row is a
valid drop target surface and Qt's `AboveItem`/`BelowItem` indicator is
available for the top level as well as for nested rows; without it the drop
indicator is empty and reordering a top-level row shows nothing. **The view SHALL
set a closed-hand cursor for the duration of `startDrag`** and restore the
previous cursor when the drag ends. The system SHALL validate a candidate drop
during the drag and SHALL show a drop indicator only for a valid target, and it
SHALL reject an invalid drop without calling the bridge or changing the document.
Releasing a dragged row above or below another row SHALL reorder it at that
position, releasing it onto a group row SHALL reparent it as a child of that
group, and releasing it on the empty viewport below the last row SHALL move it to
the document root (out of any group), each as one undoable step. The system SHALL
refuse, leaving the document unchanged, a move of the Background layer, of a
fully-locked or nesting-locked layer, of a row onto itself, of a row into its
own descendant, or of a row to a position at or below a Background in the
destination container. Dropping a dragged row on a bottom-strip button SHALL apply that
button's action to the dragged row: Delete deletes it, New Layer duplicates it
unless the dragged row is the Background — in which case New Layer converts the
Background to a normal layer in place — and New Group groups it; buttons whose
operation is not yet implemented (mask, link, fx) SHALL be inert. A sibling
reorder SHALL resolve to an above/below drop and SHALL NOT be mistaken for a drop
into the row.

#### Scenario: Dragging reorders a row [lpr_drag]

- **WHEN** a row is dragged and released above a sibling
- **THEN** the row moves to that position in one undo step and the drag does not
  select any other row

#### Scenario: A top-level sibling shows the above/below indicator [lpr_drag_top_level]

- **WHEN** a top-level row is dragged over the gap above or below a top-level
  sibling
- **THEN** the model is drop-enabled for the invalid parent, the drop indicator
  is above/below, and the reorder is accepted

#### Scenario: The drag cursor is a closed hand [lpr_drag_cursor]

- **WHEN** a row drag starts
- **THEN** the viewport cursor is a closed hand until the drag ends, then the
  previous cursor is restored

#### Scenario: A sibling reorder resolves at the drop [lpr_drag_reorder_mode]

- **WHEN** a row is dragged over the gap above or below a sibling
- **THEN** the drop indicator is set to the above/below position and the reorder
  is accepted rather than being treated as a drop into the sibling

#### Scenario: The view enters the dragging state [lpr_drag_state]

- **WHEN** a drag enters the layer tree's viewport
- **THEN** the view sets its dragging state so the drop indicator is painted, and
  an invalid target shows no indicator

#### Scenario: Dragging onto a group reparents [lpr_drag]

- **WHEN** a row is dragged and released onto a group row
- **THEN** the row becomes a child of that group in one undo step

#### Scenario: Dragging out of a group to the viewport [lpr_drop_out]

- **WHEN** a row inside a group is dragged and released on the empty viewport
  below the last row
- **THEN** the row is reparented to the document root in one undo step

#### Scenario: An invalid target is rejected before commit [lpr_drop_rules]

- **WHEN** a row is dragged over its own descendant, onto itself, or over a
  refusal target such as the Background
- **THEN** no valid drop indicator is shown and releasing the drag changes
  nothing and adds no history state

#### Scenario: A drop below the Background is refused [lpr_drop_bg_below]

- **WHEN** a row is dragged over the gap below the Background in the same
  container
- **THEN** no drop indicator is shown, releasing the drag changes nothing, and
  no history state is added

#### Scenario: A locked or Background row is refused [lpr_drag]

- **WHEN** the Background, a fully-locked, or a nesting-locked row is dragged
  onto another row
- **THEN** the move is refused and the document is unchanged

#### Scenario: A press-move starts a drag rather than extending a selection [lpr_drag_enabled]

- **WHEN** the user presses a draggable row and moves the pointer
- **THEN** the drag pipeline starts, the selection is not extended, and no
  rubber-band selection appears

#### Scenario: Dropping on Delete deletes the dragged row [lpr_drop_button]

- **WHEN** a row is dragged onto the Delete strip button and released
- **THEN** that row is deleted in one undo step

#### Scenario: Dropping on New Layer duplicates the dragged row [lpr_drop_button]

- **WHEN** a non-Background row is dragged onto the New Layer strip button and
  released
- **THEN** the row is duplicated in one undo step

#### Scenario: Dropping the Background on New Layer converts it [lpr_drop_background_convert]

- **WHEN** the Background is dragged onto the New Layer strip button and released
- **THEN** the Background is converted to a normal layer in place in one undo
  step, rather than cloned as a locked `Background copy`

#### Scenario: Dropping on New Group groups the dragged row [lpr_drop_button]

- **WHEN** a row is dragged onto the New Group strip button and released
- **THEN** the row is wrapped in a new group in one undo step

### Requirement: Layer operations

The system SHALL offer New Layer, New Group, Duplicate Layer(s), Delete
Layer(s), Group Layers, Ungroup Layers, and Move Layer Up/Down from the panel
and its menus, applying them to the active document through the bridge, and the
document's rows SHALL update afterwards. A new layer SHALL be an empty, fully
transparent raster layer at the document size and SHALL NOT change the
composite. A new group SHALL be an empty group with a Normal blend. New
Layer/New Group SHALL insert directly above the selected layer, or, when the
selected row is a group, as that group's topmost child; with no selection they
SHALL insert at the top of the stack, and the new row SHALL become selected.
Duplicating SHALL deep-copy each selected layer — children, mask, adjustment
data, and all attributes included — directly above itself, naming the copy
`"<name> copy"`, except that the copy of the Background SHALL be an ordinary
layer: not the Background, unlocked, and with an opaque alpha channel. Deleting SHALL remove every selected, eligible layer. Move
Layer Up/Down SHALL swap the selected layer with its neighbour within its own
container and MUST refuse a move that would place the layer below the
Background. Each applied operation SHALL mark the document modified and be one
undoable step.

#### Scenario: Add a layer

- **WHEN** the user clicks New Layer with a layer selected
- **THEN** an empty transparent layer is inserted directly above it, becomes selected, the row count grows by one, and the composite is unchanged

#### Scenario: Add a layer inside a selected group [m39_tree]

- **WHEN** the user clicks New Layer with a group selected
- **THEN** an empty transparent layer is inserted as that group's topmost child
  and becomes the selected row

#### Scenario: Add a group

- **WHEN** the user clicks New Group with no layer selected
- **THEN** an empty group is inserted at the top of the stack, becomes selected, and a new row appears

#### Scenario: Duplicate a layer

- **WHEN** the user duplicates the selected layer
- **THEN** a deep copy named `"<name> copy"` is inserted directly above it, the row count grows by one, and the copy becomes selected

#### Scenario: Delete a layer

- **WHEN** the user deletes the selected layer
- **THEN** the layer is removed and the panel updates

#### Scenario: Move a layer

- **WHEN** the user moves the selected layer up or down from the panel or row menu
- **THEN** its order within its container changes, the rows reorder, and the composite updates

#### Scenario: Move Down at the Background floor is refused

- **WHEN** the layer directly above the Background is moved down
- **THEN** the order is unchanged, no history state is added, and the Background
  stays bottom-most

#### Scenario: Duplicating the Background makes an ordinary layer

- **WHEN** the Background is duplicated
- **THEN** `Background copy` sits above it as an unlocked pixel layer with no lock badge, and the original stays the Background
