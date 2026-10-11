# tools/perspective-crop — delta

## MODIFIED Requirements

### Requirement: Perspective Crop tool

The Perspective Crop tool SHALL stage its quad from four separate corner clicks
in any corner order, drawing each placed point as it is clicked and closing the
quad on the fourth. On the fourth click it SHALL order the four points into
top-left, top-right, bottom-right, bottom-left (clockwise about their centroid,
starting from the top-left-most point) before warping, so the result is neither
rotated nor mirrored whatever corner was clicked first. While the quad is
incomplete a click SHALL add the next corner; once complete, a corner dragged
within 8 screen pixels of it SHALL move that corner, and a drag inside the quad
SHALL move the whole quad. Enter or a double-click inside the quad SHALL commit
it as one "Perspective Crop" history state and drop the selection; a refused
commit SHALL keep the quad staged and report why; Escape SHALL discard it.
Starting a quad on a refused document SHALL report the reason. Each placed or
dragged corner SHALL snap to the canvas edges and corners (and the composite
bounding box) within a 6-screen-pixel threshold. The canvas SHALL draw the
placed points and, once the quad is complete, shade only the canvas area outside
the quad (not the workspace), draw its edges, and draw a grid at a fixed
physical spacing (one inch, from the document resolution) clipped to the quad,
so a larger quad shows more grid lines at the same spacing.

#### Scenario: Enter commits a staged quad

- **WHEN** the `crop_group` self-test clicks four corners over a 20×20 black square in a 40×40 image and presses Enter
- **THEN** one "Perspective Crop" state is added and the document is 20×20 and black

#### Scenario: Four clicks stage the quad

- **WHEN** four corners are clicked in any order
- **THEN** the quad closes on the fourth click and no state is added until a commit

#### Scenario: Any click order is upright

- **WHEN** the four corners are clicked starting from any corner, clockwise or counter-clockwise
- **THEN** the quad reads top-left, top-right, bottom-right, bottom-left, so the warp is neither rotated nor mirrored

#### Scenario: A degenerate quad stays staged

- **WHEN** a corner is dragged onto its neighbour and Enter is pressed
- **THEN** no state is added, the size is unchanged, and the quad is still shown

#### Scenario: A corner snaps to the canvas

- **WHEN** a corner is clicked within the threshold of a canvas corner
- **THEN** the corner lands exactly on the canvas corner
