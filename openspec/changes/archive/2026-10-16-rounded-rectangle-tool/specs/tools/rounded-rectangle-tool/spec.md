## ADDED Requirements

### Requirement: Rounded Rectangle tool

Dragging with the Rounded Rectangle tool SHALL draw the dragged rectangle with
quarter-circle corners of the options bar's Radius (default 10 px), clamped to
half the shorter side, the outline staying inside the dragged box. Shift and
Alt SHALL behave as for the Rectangle tool.

#### Scenario: Rounded and clamped corners

- **WHEN** the `pictura_core::shape` unit tests draw a 100 x 60 box with Radius 10, and 100 x 40 and 40 x 40 boxes with Radius 500
- **THEN** the first is eight anchors whose corner arcs lie on radius-10 circles within the box, the second a six-anchor stadium, and the third a four-anchor circle

#### Scenario: Radius in the app

- **WHEN** the `tst_shape_tools` test draws a Rounded Rectangle in Path mode at the default Radius
- **THEN** the Radius field reads 10 px and the new component has eight anchors
