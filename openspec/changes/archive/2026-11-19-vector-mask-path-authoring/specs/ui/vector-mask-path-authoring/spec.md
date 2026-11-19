## ADDED Requirements

### Requirement: Current Path authors the selected path

`Layer ▸ Vector Mask ▸ Current Path` SHALL author the target layer's vector mask
from the path selected in the Paths panel, whether that is a saved path or the
Work Path. The command SHALL be unavailable when the layer already has a vector
mask, when no path row is selected, when the selected path is empty, or when the
selected row is a shape path or vector-mask row.

#### Scenario: A selected saved path becomes the mask [vmask_current_saved]

- **WHEN** a saved path is selected in the Paths panel, the Work Path is empty, and Current Path runs on a pixel layer
- **THEN** the layer gains a vector mask whose outline is the saved path, and pixels outside the path are hidden

#### Scenario: No selected path disables Current Path [vmask_current_none]

- **WHEN** a non-empty Work Path exists but no Paths panel row is selected
- **THEN** Current Path is disabled and running it adds no vector mask

### Requirement: The Paths panel lists and edits a layer's vector mask

While exactly one non-shape layer with a vector mask is selected, the Paths
panel SHALL list a row named `<layer name> Vector Mask`. Selecting another layer
SHALL remove the row. While the row is active, the path tools (Pen, Path
Selection, Direct Selection, and the anchor tools) SHALL edit the layer's
vector mask, not the Work Path or a saved path. Each edit SHALL update the mask's
coverage on the canvas and record one undoable history state. While a saved path
or the Work Path row is active, the path tools SHALL NOT edit the vector mask.
Delete Path on the vector-mask row SHALL delete the vector mask. Rewriting the
mask outline SHALL keep the mask's invert, disabled, and unlinked flags.

#### Scenario: The row follows the selected layer [vmask_row_listed]

- **WHEN** a pixel layer with a vector mask is selected, and then a layer without one is selected
- **THEN** the panel first lists `<layer name> Vector Mask` and then no longer lists it

#### Scenario: Moving an anchor edits the mask [vmask_row_edit]

- **WHEN** the vector-mask row is active and an anchor of the mask is moved and committed
- **THEN** the mask outline and the composite follow the anchor, one history state is recorded, and undo restores the original outline

#### Scenario: The Pen draws into the mask [vmask_pen_adds]

- **WHEN** the vector-mask row is active and the Pen places and closes three anchors
- **THEN** the mask gains one closed subpath and the Work Path stays empty

#### Scenario: A selected saved path keeps priority [vmask_saved_unaffected]

- **WHEN** a pixel layer with a vector mask is selected, a saved path row is active, and a Path Selection edit moves a subpath
- **THEN** the saved path changes and the vector mask outline is unchanged

#### Scenario: Delete Path deletes the vector mask [vmask_delete_row]

- **WHEN** the vector-mask row is active and Delete Path runs
- **THEN** the layer no longer has a vector mask and the row is gone

#### Scenario: An outline edit keeps the mask flags [vmask_flags_kept]

- **WHEN** an inverted, disabled, and unlinked vector mask has its outline rewritten
- **THEN** the mask is still inverted, disabled, and unlinked

### Requirement: The vector-mask thumbnail selects the mask path

Clicking a non-shape layer's vector-mask thumbnail to make it the active
thumbnail SHALL also make that layer's vector-mask row the active Paths panel
row. Clicking it again to make it inactive SHALL leave no active path row.

#### Scenario: Thumbnail click activates the row [vmask_thumb_selects_row]

- **WHEN** a pixel layer's vector-mask thumbnail is clicked, then clicked again
- **THEN** the vector-mask row is first the active path row, and then no row is active

### Requirement: The vector-mask thumbnail strokes the outline

The Layers panel vector-mask thumbnail SHALL show the area the mask reveals in
white and the area it hides in grey, and SHALL draw the mask's path outline over
both as a dark 1 px line.

#### Scenario: Revealed, hidden, and outline pixels differ [vmask_thumb_outline]

- **WHEN** the thumbnail of a vector mask revealing a centred square is rendered
- **THEN** a pixel inside the square is white, a pixel outside it is grey, and a pixel on its edge is dark
