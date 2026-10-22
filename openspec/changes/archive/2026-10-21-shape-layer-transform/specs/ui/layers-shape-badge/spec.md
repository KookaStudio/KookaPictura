## ADDED Requirements

### Requirement: A shape row has one thumbnail

A shape layer's row SHALL NOT show a vector-mask thumbnail beside its
thumbnail; the outline is drawn in the one shape thumbnail, as CS6 shows a
shape layer. The row SHALL still report that the layer has a vector mask.

#### Scenario: A new shape shows no mask slot [lsb_one_thumbnail]

- **WHEN** a rectangle shape layer is added and the Layers snapshot is taken with thumbnails
- **THEN** its row is a shape row with a thumbnail, no vector-mask thumbnail, and no layer-mask thumbnail
