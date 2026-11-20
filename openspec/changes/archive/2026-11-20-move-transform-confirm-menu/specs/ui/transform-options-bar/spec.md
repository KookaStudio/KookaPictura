# Spec Delta

## MODIFIED Requirements

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
