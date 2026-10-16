# free-transform Specification

## MODIFIED Requirements

### Requirement: Transform bounding-box geometry and hit-testing

The system SHALL compute the session's bounding quad as the four corners of the
source rect under the current transform and SHALL draw the quad with eight scale
handles (four corners and four edge midpoints) and a rotate affordance. Hit
testing SHALL be in screen space with a tolerance of 6 screen pixels for handles;
a press inside the quad but away from a handle SHALL move, and a press within a
20 screen-pixel band outside a corner SHALL rotate about the reference point.
The canvas SHALL show a diagonal, horizontal, or vertical size cursor over a
handle, an open or closed hand while moving, and a cross cursor in the rotate
band.

#### Scenario: Dragging a corner handle scales

- **WHEN** the user presses on a corner handle and drags away from the layer centre
- **THEN** the layer scales about the opposite corner and the bounding quad follows the pointer

#### Scenario: Dragging inside the quad moves

- **WHEN** the user presses inside the quad away from every handle and drags
- **THEN** the layer translates by the pointer delta and does not scale or rotate

#### Scenario: Dragging the rotate band rotates

- **WHEN** the user presses in the rotate band outside a corner and drags
- **THEN** the layer rotates about the reference point and the quad rotates with the pointer

### Requirement: Scale and rotate modifiers and minimum size

Holding Shift while dragging a corner handle SHALL lock the aspect ratio by
applying one uniform scale factor to both axes; edge handles SHALL always scale
one axis. Holding Shift while rotating SHALL snap the angle to 15° steps. The
session SHALL clamp the transform so the transformed rect keeps at least 1 pixel
in each axis and SHALL refuse a gesture that would produce a zero-area result.
Negative scale factors SHALL be permitted and flip the content. The reference
point SHALL be the session's movable pivot, the source rect centre by default
(see `document/transform-reference-point`).

#### Scenario: Shift on a corner keeps the aspect ratio

- **WHEN** the user drags a corner handle with Shift held
- **THEN** both scale factors are equal and the layer aspect ratio is preserved

#### Scenario: Shift rotation snaps to 15° steps

- **WHEN** the user rotates with Shift held
- **THEN** the committed rotation angle is an exact multiple of 15°

#### Scenario: A gesture cannot collapse the layer

- **WHEN** the user drags a handle so the transformed rect would have zero width or height
- **THEN** the session clamps to at least 1 pixel per axis and does not commit a zero-area rect
