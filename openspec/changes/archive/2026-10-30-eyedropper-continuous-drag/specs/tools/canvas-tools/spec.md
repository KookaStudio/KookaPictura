## ADDED Requirements

### Requirement: Eyedropper samples continuously while dragging

While the Eyedropper tool's press gesture is live, the system SHALL sample the
composited colour again at each pointer position over the document and update
the foreground colour, so the colour follows a held drag until the button is
released. A pointer move with no button held SHALL NOT sample.

#### Scenario: A held drag samples continuously [ct_eyedropper_drag]

- **WHEN** the Eyedropper is pressed over one colour and dragged with the left
  button held across a different colour
- **THEN** the foreground colour becomes the colour under the pointer after
  each move, and a move after the release leaves it unchanged
