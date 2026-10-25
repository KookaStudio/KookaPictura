# Spec Delta

## ADDED Requirements

### Requirement: Committed outline visible at the canvas border

The committed marching-ants outline SHALL remain continuously visible when a
selection edge coincides with the canvas border, at every pan and zoom. The
traced contour MUST remain the exact pixel boundary, including the canvas-border
edges; only the drawing clip may allow the cosmetic stroke to straddle the
border. A fractional pan or zoom SHALL NOT make a border segment flicker or
disappear.

#### Scenario: The border segment survives fractional pan and zoom [committed_outline_border]

- **WHEN** a selection touches the canvas border and the canvas is panned and
  zoomed to fractional offsets
- **THEN** the committed outline's border segment is drawn at every offset, with
  no frame where it disappears

#### Scenario: The visible segment is still the traced boundary [committed_outline_trace]

- **WHEN** the committed outline of an edge-touching selection is compared with
  its traced contour
- **THEN** the drawn loops and vertices are the contour's, with no segment added
  or moved by the drawing clip
