# free-transform Specification

## MODIFIED Requirements

### Requirement: Transform bounding-box geometry and hit-testing

The system SHALL compute the session's bounding quad as the four corners of the
source rect under the current transform and SHALL draw the quad with eight scale
handles (four corners and four edge midpoints) and a rotate affordance. Hit
testing SHALL be in screen space with a tolerance of 6 screen pixels for handles;
a press inside the quad but away from a handle SHALL move, and a press within a
20 screen-pixel band outside the quad's perimeter — any side or corner — SHALL
rotate about the reference point. A press beyond the band SHALL be inert: it MUST
NOT move, rotate, commit, or cancel the session. The canvas SHALL show a
diagonal, horizontal, or vertical size cursor over a handle, an open or closed
hand while moving, and a rotate cursor oriented to the grabbed side or corner
throughout the rotate band.

#### Scenario: Dragging a corner handle scales

- **WHEN** the user presses on a corner handle and drags away from the layer centre
- **THEN** the layer scales about the opposite corner and the bounding quad follows the pointer

#### Scenario: Dragging inside the quad moves

- **WHEN** the user presses inside the quad away from every handle and drags
- **THEN** the layer translates by the pointer delta and does not scale or rotate

#### Scenario: Dragging the rotate band rotates

- **WHEN** the user presses in the rotate band outside a corner and drags
- **THEN** the layer rotates about the reference point and the quad rotates with the pointer

#### Scenario: Rotating from an edge midpoint [ft_edge_rotate]

- **WHEN** the user presses in the rotate band just outside an edge midpoint and drags
- **THEN** the layer rotates about the reference point and the quad rotates with the pointer

#### Scenario: A press beyond the band is inert [ft_far_press_inert]

- **WHEN** the user presses and releases beyond the rotate band while a session is active
- **THEN** the session stays active, the document is unchanged, and no history state is added

#### Scenario: Projective modes keep handle-only hit testing [ft_projective_band]

- **WHEN** a Distort, Perspective, or Skew session is pressed outside the quad near an edge
- **THEN** nothing is hit and the quad does not change
