# ui/transform-options-bar Specification Delta

## MODIFIED Requirements

### Requirement: Session-keyed transform options page

The options bar SHALL carry a transform page, laid out left to right as: the
tool icon, a show-reference-point toggle, a 3×3 reference point locator, an X
field, a relative-positioning toggle, a Y field, a W field, a link toggle, an H
field, an angle field, H and V skew fields, an interpolation combo, a warp
button, a Cancel button, and an Apply button. The page SHALL be shown whenever
a transform session is active, whichever tool is active, for both a Free
Transform session and a Transform Selection session, and the bar SHALL return
to the active tool's page when the session ends. The X/Y/W/H fields SHALL be
seeded from the session on show and re-read on every change; for a Transform
Selection session the box is the selection's coverage bounds. The relative
toggle and the W/H link are page state.

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

### Requirement: Transform page control wiring

The page's controls SHALL write through to the session: the locator and the X/Y
fields move the reference point, the show toggle sets `show_pivot`, W/H and the
link set the scale, the angle sets the rotation, H/V set the skew, and the
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
