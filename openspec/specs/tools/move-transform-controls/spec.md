# tools/move-transform-controls Specification

## Purpose
The Move tool's `Show Transform Controls`: the Free Transform box shown on the
active layer with direct scale/rotate gestures, without the modal command.

## Requirements

### Requirement: Transform controls box on the active layer

While the Move tool is active, `Show Transform Controls` is checked, no
selection is active, and the active layer is transformable, the application
SHALL show the Free Transform bounding box and its eight handles on that layer,
without the reference-point marker. When the active layer belongs to a link set
the box SHALL span the union of every transformable member's content rect, and a
handle gesture SHALL transform the whole set exactly as the linked Free
Transform session does. The box SHALL be rebuilt when the active layer changes,
when its content rect changes (including an undo), when the link set changes,
and when the document changes; it SHALL be hidden when the checkbox is
unchecked, the tool changes, a selection becomes active, or the layer is not
transformable (a Background, group, adjustment, or position-locked layer).
Showing the box MUST NOT record a history state, and MUST NOT start a transform
session or change the layer: the box is drawn round the layer's content, and a
session (which trims the layer to that box) SHALL begin only when a press grabs
one of its handles.

#### Scenario: Checking the box shows the controls [lmtc_box_shown]

- **WHEN** `Show Transform Controls` is checked with the Move tool on a raster layer
- **THEN** the canvas shows the box and handles round the layer's content, no reference-point marker is drawn, and no transform session runs

#### Scenario: Unchecking hides the box [lmtc_box_hidden]

- **WHEN** the checkbox is unchecked while the box is shown
- **THEN** the overlay ends and no transform session remains

#### Scenario: The box follows the active layer [lmtc_box_follows]

- **WHEN** another transformable layer is selected, or the layer's content changes, while the box is shown
- **THEN** the box is rebuilt on the new layer or the new content rect

#### Scenario: The box spans a link set [lmtc_box_linked]

- **WHEN** the active layer belongs to a link set
- **THEN** the box spans the union of the set's content rects and follows the set when it is linked or unlinked

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

### Requirement: Hidden layers are not movable or transformable

A hidden layer SHALL NOT be moved or transformed. While the active layer is
hidden, the Move tool's `Show Transform Controls` box SHALL be hidden, an
in-canvas Move drag SHALL leave the layer's rect and the history unchanged, and
`Free Transform`, `Skew`, `Distort`, `Perspective`, and `Warp` SHALL be disabled
and refused. A selection SHALL NOT lift a hidden layer's pixels, and an Alt-drag
SHALL NOT duplicate it. Toggling the layer's visibility SHALL remain available.

#### Scenario: A hidden layer shows no controls box [lhe_box]

- **WHEN** the active layer is hidden while the Move tool's controls box is shown
- **THEN** the box disappears and no transform session runs

#### Scenario: The Move tool does not move a hidden layer [lhe_move]

- **WHEN** a Move drag starts on a hidden layer
- **THEN** the layer's rect is unchanged, no move preview starts, and no history
  state is added

#### Scenario: Transform commands refuse a hidden layer [lhe_transform]

- **WHEN** `Free Transform`, a transform mode, or `Warp` is invoked on a hidden
  layer, with or without a selection
- **THEN** the command is disabled or refused, no transform session starts, and
  the document is unchanged

### Requirement: Handle gestures become a pending transform

A press on a scale handle or in the rotate band while the controls box is shown
SHALL start the matching transform gesture. Releasing a gesture that changed the
target SHALL keep the transform pending: the idle box SHALL be replaced by the
session overlay with its reference point, the options bar SHALL show the Free
Transform page with Confirm and Reject controls, and no history state SHALL be
recorded yet. Confirm (the Apply button or Enter) SHALL commit it as exactly one
`"Free Transform"` history state and restore the idle controls box on the
resulting layer. Reject (the Cancel button or Esc) SHALL cancel it with no
history state and restore the idle controls box on the unchanged layer. A
gesture that changed nothing SHALL record no state, leave no session, and return
to the idle controls box. On a linked layer the pending transform SHALL cover
every transformable member of the set, and Confirm SHALL apply it to all of
them. The pending transform SHALL lock document edits and ask before a tool
switch or a layer change exactly as a modal Free Transform does. `Ctrl+T` SHALL
replace the idle box with the modal Free Transform session.

#### Scenario: A handle drag stays pending and then commits [lmtc_handle_drag]

- **WHEN** a corner handle is dragged while the controls box is shown and released
- **THEN** no history state is recorded, the transform is pending, the options bar shows the transform page, and Apply records exactly one `"Free Transform"` state and restores the idle box on the scaled layer

#### Scenario: A linked handle drag stays pending and then commits [lmtc_handle_drag_linked]

- **WHEN** a corner handle is dragged while the controls box spans a link set and released
- **THEN** the set stays pending, and Apply scales every transformable member about the same anchor, records one `"Free Transform"` state, and the links survive

#### Scenario: An identity drag records nothing [lmtc_identity]

- **WHEN** a handle is pressed and released without moving
- **THEN** no history state is recorded, no transform session remains, and the idle box remains

#### Scenario: Ctrl+T takes the box over [lmtc_ctrl_t]

- **WHEN** `Ctrl+T` is pressed while the controls box is shown
- **THEN** the modal Free Transform session begins on the layer and the options bar shows the transform page

#### Scenario: Reject cancels the pending transform [lmtc_handle_reject]

- **WHEN** a corner handle is dragged, released, and the transform page's Cancel button is clicked
- **THEN** no history state is recorded, the document is unchanged, and the idle box returns

#### Scenario: The bar shows the transform page while pending [lmtc_pending_bar]

- **WHEN** a handle drag that changed the layer is released
- **THEN** the options bar shows the transform page, and Apply or Cancel returns it to the Move page

#### Scenario: The pending session shows the reference point [lmtc_pending_pivot]

- **WHEN** a handle drag that changed the layer is released
- **THEN** the session overlay shows the reference point, while the idle box showed none

### Requirement: Idle-box presses are Move drags

A press inside the idle controls box away from a handle, or outside it, SHALL
suspend the box and run the ordinary Move tool drag: the layer translates, Alt
duplicates, and a cross-document drag keeps working, recording the Move tool's
own `"Move Layer"` state. The box SHALL return when the drag ends, positioned on
the moved layer.

#### Scenario: An inside drag moves the layer [lmtc_inside_move]

- **WHEN** the pointer is pressed inside the idle box and dragged
- **THEN** the layer translates by the drag delta, one `"Move Layer"` state is recorded, and the box returns on the moved layer

#### Scenario: A click with no movement leaves the document alone [lmtc_click]

- **WHEN** the pointer is pressed and released inside the box without moving
- **THEN** no history state is recorded and the box remains

### Requirement: Pending transform gestures refine the session

While a controls transform is pending, another handle gesture SHALL refine the
same session, and a press inside the box SHALL move the target within the
transform (Free Transform semantics) instead of recording a separate
`"Move Layer"` state. Confirm SHALL apply every refinement as one
`"Free Transform"` history state.

#### Scenario: A second handle drag refines the pending transform [lmtc_handle_refine]

- **WHEN** a second handle drag is made while the transform is pending
- **THEN** the session stays pending, no history state is recorded, and Apply commits both gestures as one `"Free Transform"` state

#### Scenario: An inside drag moves within the pending transform [lmtc_inside_pending]

- **WHEN** the pointer is pressed inside the pending box and dragged
- **THEN** the target moves within the same session, no history state is recorded, and Apply commits the move with the transform
