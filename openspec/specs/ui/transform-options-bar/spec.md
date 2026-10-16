# ui/transform-options-bar Specification

## Purpose
The CS6 Free Transform options bar: a session-keyed page with the reference
point locator, numeric transform fields, the interpolation choice, and the
warp / Cancel / Apply controls.

## Requirements

### Requirement: Session-keyed transform options page

The options bar SHALL carry a transform page, laid out left to right as: the
tool icon, a show-reference-point toggle, a 3×3 reference point locator, an X
field, a relative-positioning toggle, a Y field, a W field, a link toggle, an H
field, an angle field, H and V skew fields, an interpolation combo, a warp
button, a Cancel button, and an Apply button. The page SHALL be shown whenever
a Free Transform session is active, whichever tool is active, and the bar SHALL
return to the active tool's page when the session ends. The X/Y/W/H fields
SHALL be seeded from the session on show and re-read on every change; the
relative toggle and the W/H link are page state.

#### Scenario: Beginning a session shows the transform page

- **WHEN** `Edit > Free Transform` begins a session while the Brush tool is active
- **THEN** the options bar shows the transform page

#### Scenario: Ending a session restores the tool page

- **WHEN** the session commits or cancels
- **THEN** the options bar shows the active tool's page again

#### Scenario: The fields seed from the session

- **WHEN** the transform page is shown on a fresh session over a 100×50 layer
- **THEN** X/Y show the layer centre, W and H show 100%, and the angle shows 0°

### Requirement: Transform change signal

`ToolController` SHALL expose a `transformChanged` signal emitted when a
session begins, commits, or cancels, after each canvas gesture update, and
after a pivot press. The transform page SHALL refresh from the session on the
signal, so a canvas drag updates the numeric fields and a field edit updates
the canvas.

#### Scenario: A canvas drag updates the fields

- **WHEN** a corner-handle drag scales the layer
- **THEN** `transformChanged` fires and the W/H fields show the new percentages

#### Scenario: Begin and end fire the signal

- **WHEN** a session begins and later commits
- **THEN** `transformChanged` fires on both

### Requirement: Transform page control wiring

The page's controls SHALL write through to the session: the locator and the X/Y
fields move the reference point, the show toggle sets `show_pivot`, W/H and the
link set the scale, the angle sets the rotation, H/V set the skew, and the
interpolation combo sets the session's resample method. The interpolation combo
SHALL list the six CS6 methods (Nearest Neighbor, Bilinear, Bicubic, Bicubic
Smoother, Bicubic Sharper, Bicubic Automatic). The warp button SHALL open
`WarpPresetDialog` and set or clear the session warp. Cancel SHALL cancel the
session and Apply SHALL commit it, both ending the session.

#### Scenario: The locator moves the pivot

- **WHEN** the bottom-right locator square is clicked
- **THEN** the session's pivot equals the live quad's bottom-right corner

#### Scenario: Apply commits and records one state

- **WHEN** a field changed the transform and Apply is clicked
- **THEN** the session ends, exactly one `"Free Transform"` state is recorded, and the tool page returns

#### Scenario: Cancel restores the document

- **WHEN** a field changed the transform and Cancel is clicked
- **THEN** the session ends with the document bit-identical to its pre-session value

#### Scenario: The interpolation combo reaches the session

- **WHEN** Nearest Neighbor is chosen in the combo
- **THEN** the session reports `nearest` as its interpolation

### Requirement: Reference point canvas marker

The canvas SHALL draw the reference point as a target marker (a circle with a
centre dot or cross) at its document position while `show_pivot` is on and no
marker while it is off. The marker SHALL be drawn with the transform overlay
and SHALL track a pivot drag.

#### Scenario: The marker shows and hides

- **WHEN** `show_pivot` is toggled off and on during a session
- **THEN** the canvas draws no marker and then the marker again

#### Scenario: The marker follows a drag

- **WHEN** the pivot is dragged to a new document position
- **THEN** the canvas draws the marker at the new position
