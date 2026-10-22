# ui/layers-shape-badge Specification

## Purpose
Shape layer rows in the Layers panel: badge and vector-mask affordances.

## Requirements

### Requirement: Shape layer rows

A shape layer's row in the Layers panel SHALL show a thumbnail of its filled
outline and the `layers.kindShape` badge on the thumbnail's corner, in place of
the adjustment badge.

#### Scenario: A drawn shape is marked

- **WHEN** the `tst_shape_tools` test draws a Rectangle in Shape mode
- **THEN** the Layers panel reports the new row as a shape row with a non-null thumbnail

### Requirement: A shape row has one thumbnail

A shape layer's row SHALL NOT show a vector-mask thumbnail beside its
thumbnail; the outline is drawn in the one shape thumbnail, as CS6 shows a
shape layer. The row SHALL still report that the layer has a vector mask.

#### Scenario: A new shape shows no mask slot [lsb_one_thumbnail]

- **WHEN** a rectangle shape layer is added and the Layers snapshot is taken with thumbnails
- **THEN** its row is a shape row with a thumbnail, no vector-mask thumbnail, and no layer-mask thumbnail
