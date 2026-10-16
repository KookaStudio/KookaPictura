## ADDED Requirements

### Requirement: Show Transform Controls starts checked

The Move tool's `Show Transform Controls` SHALL be checked by default, by
owner decision; CS6 ships it unchecked. The user's choice SHALL persist
across restarts like the other tool options.

#### Scenario: Checked on a fresh launch [mtc_default_on]

- **WHEN** `tst_move_feedback::transformControlsOnByDefault` starts with an empty session store
- **THEN** the Move options report `Show Transform Controls` checked

### Requirement: The controls box leaves type layers untrimmed

Showing the transform controls (or starting Free Transform) SHALL NOT trim a
type layer's rect to its pixels. A type layer's rect comes from its layout (a
paragraph layer's is its box), and its origin is read back against that rect,
so trimming it would shift the text on the next edit.

#### Scenario: A type layer keeps its rect [mtc_type_untrimmed]

- **WHEN** `pictura-render::trim_leaves_type_layers` trims a type layer whose pixels cover only its middle
- **THEN** nothing changes and the layer keeps its rect

### Requirement: Dismissing the box restores the layer's extent

When the controls box (or Free Transform) hugged a layer to its pixels and is
then dismissed without a transform (a tool switch, another layer, Esc, or an
identity commit), the layer SHALL be padded back out to its previous rect with
transparent pixels. The composite SHALL be unchanged, and later filters SHALL
keep their room to spread past the content.

#### Scenario: The extent comes back, the pixels stay [mtc_untrim]

- **WHEN** `pictura-render::untrim_restores_the_extent_without_changing_pixels` trims a layer to its content, then untrims it
- **THEN** the layer has its original rect and alpha, and its content keeps its place

## MODIFIED Requirements

### Requirement: Transform controls box on the active layer

While the Move tool is active, `Show Transform Controls` is checked, no
selection is active, and the active layer is transformable, the application
SHALL show the Free Transform bounding box and its eight handles on that layer,
without the reference-point marker. The box SHALL be rebuilt when the active
layer changes, when its content rect changes (including an undo), and when the
document changes; it SHALL be hidden when the checkbox is unchecked, the tool
changes, a selection becomes active, or the layer is not transformable (a
Background, group, adjustment, or position-locked layer). Showing the box MUST
NOT record a history state, and MUST NOT start a transform session or change the
layer: the box is drawn round the layer's content, and a session (which trims
the layer to that box) SHALL begin only when a press grabs one of its handles.

#### Scenario: Checking the box shows the controls [lmtc_box_shown]

- **WHEN** `Show Transform Controls` is checked with the Move tool on a raster layer
- **THEN** the canvas shows the box and handles round the layer's content, no reference-point marker is drawn, and no transform session runs

#### Scenario: Unchecking hides the box [lmtc_box_hidden]

- **WHEN** the checkbox is unchecked while the box is shown
- **THEN** the overlay ends and no transform session remains

#### Scenario: The box follows the active layer [lmtc_box_follows]

- **WHEN** another transformable layer is selected, or the layer's content changes, while the box is shown
- **THEN** the box is rebuilt on the new layer or the new content rect

#### Scenario: An untransformable target shows no box [lmtc_box_refused]

- **WHEN** the active layer is a Background, a group, an adjustment layer, or position-locked
- **THEN** no box is shown and no session begins

#### Scenario: A selection suppresses the box [lmtc_box_selection]

- **WHEN** a selection becomes active while the box is shown
- **THEN** the box is hidden until the selection is cleared

#### Scenario: Showing the box records nothing [lmtc_box_no_history]

- **WHEN** the box is shown and hidden without a gesture
- **THEN** the history count is unchanged

#### Scenario: The idle box leaves the layer alone [lmtc_box_idle]

- **WHEN** `tst_transform_controls::idleBoxLeavesTheLayerAlone` shows the box on a canvas-sized layer holding a small square
- **THEN** the box hugs the square while the layer keeps its full rect, so a filter run with the box up can still spread past the square
