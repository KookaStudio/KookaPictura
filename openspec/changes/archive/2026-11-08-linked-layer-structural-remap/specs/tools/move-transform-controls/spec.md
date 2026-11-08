## MODIFIED Requirements

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

### Requirement: Handle gestures apply per drag

A press on a scale handle or in the rotate band while the controls box is shown
SHALL start the matching transform gesture; releasing SHALL apply it
immediately and record exactly one `"Free Transform"` history state when
anything changed, then show the box again on the resulting layer. On a linked
layer the gesture SHALL apply to every transformable member of the set. A drag
that changes nothing SHALL record no state. The options bar SHALL stay on the
Move page throughout, and `Ctrl+T` SHALL replace the box with the modal Free
Transform session (switching the bar to the transform page).

#### Scenario: A handle drag scales and commits [lmtc_handle_drag]

- **WHEN** a corner handle is dragged while the controls box is shown
- **THEN** the layer is scaled on release, exactly one `"Free Transform"` state is recorded, and the box remains on the layer

#### Scenario: A linked handle drag scales the set [lmtc_handle_drag_linked]

- **WHEN** a corner handle is dragged while the controls box spans a link set
- **THEN** every transformable member scales about the same anchor, one `"Free Transform"` state is recorded, and the links survive

#### Scenario: An identity drag records nothing [lmtc_identity]

- **WHEN** a handle is pressed and released without moving
- **THEN** no history state is recorded and the box remains

#### Scenario: Ctrl+T takes the box over [lmtc_ctrl_t]

- **WHEN** `Ctrl+T` is pressed while the controls box is shown
- **THEN** the modal Free Transform session begins on the layer and the options bar shows the transform page
