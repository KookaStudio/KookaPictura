# move-transform-controls Specification

## Purpose

The Move tool's `Show Transform Controls`: the Free Transform box shown on the
active layer with direct scale/rotate gestures, without the modal command.

## ADDED Requirements

### Requirement: Transform controls box on the active layer

While the Move tool is active, `Show Transform Controls` is checked, no
selection is active, and the active layer is transformable, the application
SHALL show the Free Transform bounding box and its eight handles on that layer,
without the reference-point marker. The box SHALL be rebuilt when the active
layer changes, when its content rect changes (including an undo), and when the
document changes; it SHALL be hidden when the checkbox is unchecked, the tool
changes, a selection becomes active, or the layer is not transformable (a
Background, group, adjustment, or position-locked layer). Showing the box MUST
NOT record a history state.

#### Scenario: Checking the box shows the controls [lmtc_box_shown]

- **WHEN** `Show Transform Controls` is checked with the Move tool on a raster layer
- **THEN** the transform overlay is active on that layer, the canvas shows the box and handles, and no reference-point marker is drawn

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

### Requirement: Handle gestures apply per drag

A press on a scale handle or in the rotate band while the controls box is shown
SHALL start the matching transform gesture; releasing SHALL apply it
immediately and record exactly one `"Free Transform"` history state when
anything changed, then show the box again on the resulting layer. A drag that
changes nothing SHALL record no state. The options bar SHALL stay on the Move
page throughout, and `Ctrl+T` SHALL replace the box with the modal Free
Transform session (switching the bar to the transform page).

#### Scenario: A handle drag scales and commits [lmtc_handle_drag]

- **WHEN** a corner handle is dragged while the controls box is shown
- **THEN** the layer is scaled on release, exactly one `"Free Transform"` state is recorded, and the box remains on the layer

#### Scenario: An identity drag records nothing [lmtc_identity]

- **WHEN** a handle is pressed and released without moving
- **THEN** no history state is recorded and the box remains

#### Scenario: Ctrl+T takes the box over [lmtc_ctrl_t]

- **WHEN** `Ctrl+T` is pressed while the controls box is shown
- **THEN** the modal Free Transform session begins on the layer and the options bar shows the transform page

### Requirement: Inside and outside presses are Move drags

A press inside the controls box away from a handle, or outside it, SHALL
suspend the box and run the ordinary Move tool drag: the layer translates, Alt
duplicates, and a cross-document drag keeps working, recording the Move tool's
own `"Move Layer"` state. The box SHALL return when the drag ends, positioned
on the moved layer.

#### Scenario: An inside drag moves the layer [lmtc_inside_move]

- **WHEN** the pointer is pressed inside the box and dragged
- **THEN** the layer translates by the drag delta, one `"Move Layer"` state is recorded, and the box returns on the moved layer

#### Scenario: A click with no movement leaves the document alone [lmtc_click]

- **WHEN** the pointer is pressed and released inside the box without moving
- **THEN** no history state is recorded and the box remains
