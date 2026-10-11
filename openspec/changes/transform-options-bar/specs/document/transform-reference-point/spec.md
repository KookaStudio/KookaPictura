# transform-reference-point Specification

## Purpose

The Free Transform reference point (pivot): a movable document-space anchor for
the session's gestures and numeric setters, with 3×3 presets, canvas dragging
with anchor snapping, and X/Y semantics.

## ADDED Requirements

### Requirement: Session reference point and presets

The Free Transform session SHALL hold a document-space reference point
(`pivot`), initialised to the centre of the source rect when the session
begins, and a `show_pivot` flag, on by default. The session SHALL expose the
pivot position, the session origin (the initial centre), and the show flag
through the bridge, and SHALL accept a new pivot position and a preset anchor
index. A preset index `0..=8` in row-major order (top-left, top-centre,
top-right, middle-left, centre, middle-right, bottom-left, bottom-centre,
bottom-right) SHALL place the pivot at the matching anchor of the live quad:
its four corners, its four edge midpoints, or its centre. An out-of-range index
SHALL be refused. Setting the pivot SHALL NOT change the rendered quad: the
session's translation SHALL be compensated so the image stays where it is.

#### Scenario: The pivot starts at the source centre

- **WHEN** a session begins on a layer
- **THEN** the pivot equals the source rect centre and `show_pivot` is on

#### Scenario: A preset anchors on the live quad

- **WHEN** a session has scaled and rotated its layer and the top-left preset is applied
- **THEN** the pivot equals the live quad's top-left corner

#### Scenario: Moving the pivot leaves the image in place

- **WHEN** the pivot is set to a new position mid-session
- **THEN** the quad and the committed result are identical to the same session with the old pivot

### Requirement: Reference point drag and snapping

The canvas SHALL show the reference point while `show_pivot` is on, and a press
within 6 screen pixels of it SHALL begin a pivot drag (a dedicated hit result,
tested before the box handles) even in a projective mode. Dragging SHALL move
the pivot to the pointer; while within 6 screen pixels of any of the live quad's
nine anchors the pivot SHALL snap to that anchor. Releasing SHALL end the drag.
A pivot drag MUST NOT change the quad, scale, rotation, or skew.

#### Scenario: A pivot press is distinct from the box handles

- **WHEN** a press lands on the reference point
- **THEN** the session reports a pivot hit and not a scale, rotate, or move handle

#### Scenario: Dragging near an anchor snaps

- **WHEN** a pivot drag moves within the snap threshold of the live quad's centre
- **THEN** the pivot equals that anchor exactly

#### Scenario: Dragging away keeps the free position

- **WHEN** a pivot drag moves past the snap threshold of every anchor
- **THEN** the pivot equals the pointer's document position

#### Scenario: A hidden pivot is not draggable

- **WHEN** `show_pivot` is off and a press lands where the pivot is
- **THEN** no pivot drag begins

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

Handle drags SHALL keep the opposite-handle anchor and MUST NOT move the
reference point. In a projective mode the W/H setters SHALL be unavailable and
the angle and H/V setters SHALL rotate/shear the live quad about the reference
point.

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

### Requirement: Skew readouts

The session SHALL report H skew as the deviation of the quad's left edge from
vertical and V skew as the deviation of the quad's top edge from horizontal,
both in degrees and relative to the layer's rotation. In `Free` mode the
readouts SHALL equal the stored skew scalars; in a projective mode they SHALL
be derived from the live quad. The derived readouts are inferred (no Adobe
oracle).

#### Scenario: Readouts match the setters in Free mode

- **WHEN** H is set to 20° and V to −10° in a Free session
- **THEN** the H and V readouts are 20° and −10°

#### Scenario: A Skew-mode drag changes the readouts

- **WHEN** a Skew session slides its top edge horizontally
- **THEN** the derived H skew readout is non-zero
