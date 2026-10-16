# free-transform Specification

## ADDED Requirements

### Requirement: Transform overlay is not clipped to the document edge

The Free Transform overlay — the bounding quad, its eight handles, the rotate
knob, and the reference point — SHALL be painted without clipping to the
document rectangle, so a layer transformed past the canvas edge keeps its full
box and handles. Only the rasterised preview content SHALL be clipped to the
canvas. The rule SHALL apply to the modal session and to the Move tool's
controls box, which share the overlay path.

#### Scenario: An off-canvas corner keeps its handle [ft_overlay_unclipped]

- **WHEN** a layer is scaled so one corner lies outside the document and the canvas is painted
- **THEN** the off-canvas corner's handle is drawn exactly as the on-canvas corner's handle

#### Scenario: The preview content stays clipped [ft_preview_clipped]

- **WHEN** the transformed layer extends past the document edge
- **THEN** its rasterised preview is not painted outside the canvas rectangle
