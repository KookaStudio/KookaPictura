## MODIFIED Requirements

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
container. Each applied operation SHALL mark the document modified and be one
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

#### Scenario: Duplicating the Background makes an ordinary layer

- **WHEN** the Background is duplicated
- **THEN** `Background copy` sits above it as an unlocked pixel layer with no lock badge, and the original stays the Background

## ADDED Requirements

### Requirement: The Background's lock badge unlocks it

Clicking the lock badge on the Background row SHALL convert the Background into an ordinary layer at once, without a dialog: it SHALL be named with the next free `Layer N` name, unflagged, unlocked, and given an opaque alpha channel, as one undoable step. Converting a Background by any path SHALL give it an alpha channel so it can take transparency.

#### Scenario: Clicking the Background's lock unlocks it

- **WHEN** the lock badge on an opened photo's Background row is clicked
- **THEN** the row becomes an unlocked pixel layer and exactly one history state is recorded
