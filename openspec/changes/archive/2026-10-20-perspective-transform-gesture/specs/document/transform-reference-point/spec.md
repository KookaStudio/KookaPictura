## MODIFIED Requirements

### Requirement: Pivot-anchored gestures and numeric setters

The rotate gesture SHALL rotate about the reference point's current image
position. The numeric setters SHALL be absolute and SHALL keep the reference
point's current image position fixed while changing the transform:

- X/Y SHALL move the reference point only: in absolute mode the value is the
  document coordinate, in relative mode it is the offset from the session
  origin; the rendered image MUST NOT move.
- W/H SHALL set the horizontal/vertical scale as a percentage of the source
  rect (100 = unchanged), scaling about the reference point; a link flag SHALL
  apply the changed axis's factor to both axes.
- The rotation angle SHALL set the clockwise rotation in degrees about the
  reference point.
- H/V SHALL set the horizontal/vertical skew in degrees, shearing about the
  reference point; the shear SHALL be committed through the projective
  (affine quad) op.

Handle drags in `Free` and `Skew` SHALL keep the opposite-handle anchor and
MUST NOT move the reference point. A `Distort` corner drag SHALL carry the
reference point with the quad, preserving the reference point's position within
the box. A `Perspective` drag, and the Free Transform Ctrl+Alt+Shift
perspective drag, SHALL carry the reference point by its bilinear position
within the box. In a projective mode the W/H setters
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
