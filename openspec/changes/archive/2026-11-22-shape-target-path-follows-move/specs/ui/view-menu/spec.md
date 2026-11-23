# ui/view-menu Specification

## MODIFIED Requirements

### Requirement: Target Path overlay

`View > Show > Target Path` SHALL be a checkable command, on by default. When on
and a non-path tool is active, the canvas SHALL draw the Paths panel's active
path (Work Path, saved path, or shape layer path) as a curve following pan, zoom
and view rotation. The path tools SHALL keep their editing overlay with anchors
and handles, and the base curve SHALL NOT draw while a path-editing tool is
active. A target change, a path edit, or a move or transform of the layer whose
outline is the target SHALL update the curve; no active path or an empty path
SHALL draw nothing. Extras off SHALL hide the curve without changing the Show
toggle.

#### Scenario: The target path shows with another tool [vm_target_path]

- **WHEN** a Work Path exists and the Move tool is active with Target Path on
- **THEN** the path curve is drawn without the Pen tool's anchors and handles

#### Scenario: The toggle and Extras hide the target path [vm_target_path_off]

- **WHEN** Target Path is turned off and then on, and Extras is then turned off
- **THEN** the curve disappears, returns, and disappears again with the Show checkmark unchanged

#### Scenario: A path edit updates the curve [vm_target_path_edit]

- **WHEN** the active path is edited or the Paths panel selects another path
- **THEN** the persistent curve shows the new target and geometry

#### Scenario: A path tool owns its overlay [vm_target_path_path_tool]

- **WHEN** the Pen or Path Selection tool is active with Target Path on
- **THEN** only the tool's editing overlay is drawn, with no duplicate base curve

#### Scenario: Moving a shape layer moves its target path [vm_target_path_layer_move]

- **WHEN** a shape layer's outline is the target path and the Move tool moves the layer
- **THEN** the persistent curve follows the moved outline
