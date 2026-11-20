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
a transform session is active, whichever tool is active, for both a Free
Transform session and a Transform Selection session, and the bar SHALL return
to the active tool's page when the session ends. A Move tool `Show Transform
Controls` gesture in progress is the exception: the bar SHALL stay on the Move
page while the idle box is shown and while the gesture drags, and SHALL show the
transform page once the gesture ends with a pending transform. The X/Y/W/H
fields SHALL be seeded from the session on show and re-read on every change;
for a Transform Selection session the box is the selection's coverage bounds.
The relative toggle and the W/H link are page state.

#### Scenario: Beginning a session shows the transform page

- **WHEN** `Edit > Free Transform` begins a session while the Brush tool is active
- **THEN** the options bar shows the transform page

#### Scenario: Ending a session restores the tool page

- **WHEN** the session commits or cancels
- **THEN** the options bar shows the active tool's page again

#### Scenario: The fields seed from the session

- **WHEN** the transform page is shown on a fresh session over a 100×50 layer
- **THEN** X/Y show the layer centre, W and H show 100%, and the angle shows 0°

#### Scenario: A border session seeds from the selection bounds [tob_seed_border]

- **WHEN** `Select > Transform Selection` begins on a 100×50 selection
- **THEN** X/Y show the selection's centre, W and H show 100%, and the angle shows 0°

#### Scenario: A Move controls gesture shows the transform page when it becomes pending [tob_controls_pending]

- **WHEN** a Move tool `Show Transform Controls` handle drag changes the layer and is released
- **THEN** the options bar shows the transform page with its Apply and Cancel controls until the transform is confirmed or rejected, when the Move page returns

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

The page's controls SHALL write through to the session. The 3×3 locator SHALL
move the reference point within the target and SHALL highlight the cell that
holds the reference point; when the reference point is not on one of the nine
anchors, no cell SHALL be highlighted. The X/Y fields SHALL set the reference
point's document position and translate the target so the reference point lands
there, refreshing the canvas preview on every committed edit; in absolute mode
the values are document coordinates, and with relative positioning on they are
offsets from a fixed session origin. The show toggle sets `show_pivot`, W/H and
the link set the scale, the angle sets the rotation, H/V set the skew, and the
interpolation combo sets the session's resample method. The interpolation combo
SHALL list the six CS6 methods (Nearest Neighbor, Bilinear, Bicubic, Bicubic
Smoother, Bicubic Sharper, Bicubic Automatic). The warp button SHALL switch
between Free Transform and Warp modes and show as checked in Warp mode. Cancel
SHALL cancel the session and Apply SHALL commit it, both ending the session; the
commit SHALL record the session's own history label (`"Free Transform"` for a
layer, `"Transform Selection"` for a selection border).

#### Scenario: The locator moves the pivot

- **WHEN** the bottom-right locator square is clicked
- **THEN** the session's pivot equals the live quad's bottom-right corner

#### Scenario: The locator highlights the active cell [tob_pivot_highlight]

- **WHEN** a locator cell is clicked, and then the reference point is dragged between anchors
- **THEN** the clicked cell is highlighted and the others are not, and a pivot between anchors leaves every cell unhighlighted

#### Scenario: X and Y translate the target [tob_xy_moves_target]

- **WHEN** a document X or Y value is typed into the field
- **THEN** the target translates so the reference point lands at that position, the canvas preview shows the move before Apply, and Apply commits the moved result

#### Scenario: Apply commits and records one state

- **WHEN** a field changed the transform and Apply is clicked
- **THEN** the session ends, exactly one state labelled for the session's target is recorded, and the tool page returns

#### Scenario: Apply on a border session records Transform Selection [tob_apply_border]

- **WHEN** a field changes a selection-border transform and Apply is clicked
- **THEN** exactly one `"Transform Selection"` state is recorded and the tool page returns

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

### Requirement: Warp mode fields

In Warp mode the transform page SHALL hide the reference-point, X/Y, W/H, angle
and skew fields and show the Warp fields, ported from photocraft's Warp bar:
- a style menu with None, Custom, and the fifteen presets in CS6 order;
- an orientation toggle;
- Bend, H, and V percentages in `[-100, 100]`.

Choosing Custom SHALL keep the current shape as a mesh. Choosing None SHALL
reset to the untouched one-patch grid, which reads as Custom. Choosing a preset
SHALL start it at the current bend, or at 50% from a non-preset, with no
distortion. The orientation toggle and the Bend, H, and V fields SHALL be
enabled only for a preset and SHALL write through to it.

#### Scenario: Warp mode swaps the fields [tob_warp_fields]

- **WHEN** a session enters Warp mode
- **THEN** the warp toggle is checked, the Warp fields are shown, and the angle field is hidden

#### Scenario: A preset style takes its values from the bar [tob_warp_values]

- **WHEN** Arc is chosen, Bend is typed as -25, H as 10, and the orientation toggle is clicked
- **THEN** the session's warp is Arc with bend -25, H 10, and vertical orientation, and choosing None disables the Bend field again

### Requirement: Split scale slider

The W and H fields' slider popups SHALL map their first half to the scale range
from the field minimum to 100% and their second half to 100%-1000%, so the
midpoint is exactly 100% and the maximum is 1000%. The typed range SHALL also
end at 1000%.

#### Scenario: The scale slider midpoint is 100% [tob_scale_slider_split]

- **WHEN** the W slider is set to its midpoint, to a quarter, and to its maximum
- **THEN** W is 100% at the midpoint, about 50% at a quarter, and 1000% at the maximum, and the H slider has the same range

### Requirement: X/Y slider span

The X and Y fields' slider popups SHALL use a coarse span of about 20,000 px
centred on the typed range's midpoint, so one slider step changes the position
by about 100 px, while the typed fields SHALL still accept the full document
range. A value outside the slider span SHALL park the slider at the nearest end
without changing the typed value.

#### Scenario: A slider step moves about 100 px [tob_xy_slider_span]

- **WHEN** the X slider is moved by one step and then the X field is typed a value outside the slider span
- **THEN** X changes by about 100 px for the step, and the typed value still moves the target beyond the slider span
