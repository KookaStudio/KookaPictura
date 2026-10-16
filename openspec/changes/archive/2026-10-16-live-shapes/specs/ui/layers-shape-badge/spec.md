## ADDED Requirements

### Requirement: Shape layer rows

A shape layer's row in the Layers panel SHALL show a thumbnail of its filled
outline and the `layers.kindShape` badge on the thumbnail's corner, in place of
the adjustment badge.

#### Scenario: A drawn shape is marked

- **WHEN** the `tst_shape_tools` test draws a Rectangle in Shape mode
- **THEN** the Layers panel reports the new row as a shape row with a non-null thumbnail
