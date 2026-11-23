# ui/view-menu Specification

## MODIFIED Requirements

### Requirement: Layer Edges overlay

`View > Show > Layer Edges` SHALL be a checkable command, on by default. When on,
the canvas SHALL outline the active layer's content bounds with a thin
non-printing border that follows pan, zoom and view rotation. A target change, a
layer selection, or a document edit SHALL update the outline; while the Move tool
previews a layer move, the outline SHALL follow the preview offset, and a
selection-content move SHALL leave it in place. No active layer or an empty
content rect SHALL draw nothing. Extras off SHALL hide the outline without
changing the Show toggle.

#### Scenario: The active layer is outlined [vm_layer_edges]

- **WHEN** a small pixel layer is the active layer on a larger canvas and Layer Edges is on
- **THEN** the outline matches the layer's content rect and not the canvas rect

#### Scenario: Changing the target moves the outline [vm_layer_edges_target]

- **WHEN** the active layer switches from one pixel layer to another at a different position
- **THEN** the outline follows the new active layer's content bounds

#### Scenario: A document edit moves the outline [vm_layer_edges_edit]

- **WHEN** the active layer's pixels change so its content bounds grow
- **THEN** the outline updates to the new content rect

#### Scenario: Dragging a layer moves its outline [vm_layer_edges_drag]

- **WHEN** the Move tool drags a shape layer with Layer Edges on
- **THEN** the dashed outline follows the preview offset during the drag and lands at the committed content bounds

#### Scenario: Selecting another layer moves the outline [vm_layer_edges_active]

- **WHEN** the Layers panel selects a different shape layer with Layer Edges on
- **THEN** the dashed outline shows the newly selected layer's content bounds before any document edit
