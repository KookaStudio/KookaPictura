# Spec Delta — tools/brush-tools

## ADDED Requirements

### Requirement: The brush outline is hidden during a transform session

While a Free Transform, Scale, Rotate, Skew, Distort, Perspective, or Warp
session is open on the canvas, the system SHALL NOT draw the brush-size ring,
even when Brush, Pencil, or the Eraser is the active tool. Entering the session
SHALL clear an outline that is already drawn before the first transform frame
is painted, and a zoom or brush-size change during the session MUST NOT draw it
again. The outline SHALL be available again on the next hover after the session
ends.

#### Scenario: Entering Free Transform clears the ring [bt_transform_clears]

- **WHEN** the Brush tool is active with its ring drawn and Free Transform opens
- **THEN** the ring is gone in the first painted transform frame

#### Scenario: A zoom during the session keeps the ring hidden [bt_transform_zoom]

- **WHEN** the canvas is zoomed while a transform session is open
- **THEN** no brush ring is drawn

#### Scenario: The ring returns after the session [bt_transform_restores]

- **WHEN** the session is committed or cancelled and the pointer next hovers the canvas
- **THEN** the brush ring is drawn again at the pointer
