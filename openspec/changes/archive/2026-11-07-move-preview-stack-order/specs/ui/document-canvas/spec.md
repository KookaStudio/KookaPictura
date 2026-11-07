## ADDED Requirements

### Requirement: A Move drag repaint covers the previous frame

While a Move drag is active, a canvas repaint SHALL cover the previous frame's
moving-layer rectangle as well as the current one, so a repaint limited to the
marching-ants rectangle cannot leave the previous frame's pixels on the canvas.

#### Scenario: An ants-only repaint clears the previous frame [cvda_move_previous_frame]

- **WHEN** a Move drag has drawn a frame, then moved, and a repaint covers only
  the marching-ants rectangle
- **THEN** the pixels at the previous frame's position are the backdrop, not the
  moving layer, and the current frame shows the moving layer at its new place
