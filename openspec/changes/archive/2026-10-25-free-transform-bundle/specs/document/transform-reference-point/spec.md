# Spec Delta

## MODIFIED Requirements

### Requirement: Session reference point and presets

The Free Transform session SHALL hold a reference point (`pivot`), initialised
to the centre of the source rect when the session begins, and a `show_pivot`
flag, on by default. In `Free`, `Scale`, and `Rotate` the pivot SHALL be held in
the item's source frame, so its document position is the source point through
the session's similarity map; in a projective mode (`Skew`, `Distort`,
`Perspective`) it SHALL be a document point. The session SHALL expose the
pivot's document position, the session origin (the initial centre), and the
show flag through the bridge, and SHALL accept a new document position and a
preset anchor index. A preset index
`0..=8` in row-major order (top-left, top-centre, top-right, middle-left,
centre, middle-right, bottom-left, bottom-centre, bottom-right) SHALL place the
pivot at the matching anchor of the live quad: its four corners, its four edge
midpoints, or its centre. An out-of-range index SHALL be refused, as SHALL a
document position whose inverse through the session's map does not exist.
Setting the pivot SHALL NOT change the rendered quad: the request SHALL be
inverted through the session's map to the source point whose image is the
requested document position, and the translation SHALL be compensated so the
image stays where it is and the marker lands on the requested position. In a
projective mode the request SHALL set the document point directly.

#### Scenario: The pivot starts at the source centre

- **WHEN** a session begins on a layer
- **THEN** the pivot equals the source rect centre and `show_pivot` is on

#### Scenario: A preset anchors on the live quad

- **WHEN** a session has scaled and rotated its layer and the top-left preset is applied
- **THEN** the pivot equals the live quad's top-left corner

#### Scenario: Moving the pivot leaves the image in place

- **WHEN** the pivot is set to a new position mid-session
- **THEN** the quad and the committed result are identical to the same session with the old pivot

#### Scenario: The setter uses the document position [pivot_setter_document_position]

- **WHEN** a translated and scaled session sets the pivot to a document position
- **THEN** the marker is at that position, the stored source point is its inverse through the session map, and the quad is unchanged

### Requirement: Pivot-anchored gestures and numeric setters

The rotate gesture SHALL rotate about the reference point's current document
position. The numeric setters SHALL be absolute. Except for X/Y, they SHALL
keep the reference point's current document position fixed while changing the
transform:

- X/Y SHALL move the reference point only: in absolute mode the value is the
  document position, in relative mode it is the offset from the session origin;
  the rendered image MUST NOT move and the marker MUST land on the requested
  position.
- W/H SHALL set the horizontal/vertical scale as a percentage of the source
  rect (100 = unchanged), scaling about the reference point; a link flag SHALL
  apply the changed axis's factor to both axes.
- The rotation angle SHALL set the clockwise rotation in degrees about the
  reference point.
- H/V SHALL set the horizontal/vertical skew in degrees, shearing about the
  reference point; the shear SHALL be committed through the projective
  (affine quad) op.

Handle drags in `Free` SHALL keep the opposite-handle anchor. A translation or
a scale SHALL carry the reference point with the box so it keeps its place on
the item; a rotation SHALL keep it at the rotation centre. A `Free` scale that
edits a projective base SHALL carry the reference point by the box's projective
map. Handle drags in `Skew` SHALL keep the opposite-handle anchor and the
reference point's document position fixed; the Free Transform Ctrl+Shift skew
shortcut SHALL also keep the reference point's document position fixed. A
`Distort` corner drag SHALL carry the reference point with the quad, preserving
the reference point's position within the box. A `Perspective` drag, and the
Free Transform Ctrl+Alt+Shift perspective drag, SHALL carry the reference point
by its bilinear position within the box. In a projective mode the W/H setters
SHALL be unavailable and the angle and H/V setters SHALL rotate/shear the live
quad about the reference point.

#### Scenario: Rotating about a moved pivot keeps it fixed

- **WHEN** the pivot is moved, the layer is moved, and then the angle is set to 30°
- **THEN** the pixel at the pivot's image position is unchanged by the rotation

#### Scenario: Relative X/Y is an offset from the origin

- **WHEN** relative mode is on and X is set to 10
- **THEN** the pivot's x equals the session origin's x plus 10

#### Scenario: Link applies one factor to both axes

- **WHEN** W is set to 150 with the link on
- **THEN** both the horizontal and vertical scale are 1.5

#### Scenario: H skew shears the layer

- **WHEN** H is set to 30° and the session commits
- **THEN** the layer's vertical edges deviate 30° from vertical and one `"Free Transform"` state is recorded

#### Scenario: A Distort corner drag carries the reference point [distort_pivot_carry]

- **WHEN** a `Distort` corner drag moves the box
- **THEN** the reference point moves with the box and keeps its position within it

#### Scenario: A Perspective drag carries the reference point [perspective_pivot_carry]

- **WHEN** a `Perspective` drag spreads one edge of a box whose reference point is at the centre
- **THEN** the reference point stays at the average of the quad's corners

#### Scenario: The pivot follows a move and an off-anchor scale [pivot_follows_move_and_scale]

- **WHEN** a `Free` session moves the box and then scales it with a handle whose opposite anchor is away from the pivot
- **THEN** the pivot keeps its source-frame place and its document position follows the box

#### Scenario: A Free scale of a projective base carries the pivot [pivot_follows_base_scale]

- **WHEN** a `Free` session with a projective base scales the box by dragging a handle
- **THEN** the reference point moves with the box's projective map and keeps its relative place in it

#### Scenario: Rotation keeps the pivot at the rotation centre [pivot_stays_during_rotation]

- **WHEN** a `Free` session rotates the box by dragging the rotate band
- **THEN** the pivot's document position is unchanged and remains the rotation centre
