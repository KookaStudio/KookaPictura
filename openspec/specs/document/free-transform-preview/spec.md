# document/free-transform-preview Specification

## Purpose
The live preview of an open transform session: the layer is drawn under the
session's map while the box is dragged, before the commit.

## Requirements

### Requirement: The preview follows a similarity session

While a Free Transform, Scale, or Rotate session is open, the canvas SHALL draw
the moving layer under the session's current similarity map (rotation, scale,
and translation about the reference point), so the layer turns and scales with
the box. A raster layer and a shape layer SHALL preview alike.

#### Scenario: A quarter turn previews the turned layer [ftp_similarity]

- **WHEN** Free Transform opens on a layer over (10, 10)-(30, 20) and its angle is set to 90°
- **THEN** the canvas draws the layer's top-left corner at (25, 5), for a shape layer and for a raster layer
