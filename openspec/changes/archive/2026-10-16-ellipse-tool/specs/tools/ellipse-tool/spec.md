## ADDED Requirements

### Requirement: Ellipse tool

Dragging with the Ellipse tool SHALL draw the ellipse inscribed in the dragged
rectangle as four smooth anchors; Shift held SHALL draw a circle and Alt held
SHALL grow it from the press point.

#### Scenario: Ellipse and circle

- **WHEN** the `pictura_core::shape` unit tests drag an 80 x 40 ellipse and a Shift-dragged one
- **THEN** the first is four smooth anchors at the box's edge midpoints whose curve fills the box, and the second is a circle of the drag's longer side

#### Scenario: Ellipse in the app

- **WHEN** the `tst_shape_tools` test drags an Ellipse from (60, 60) to (90, 80) in Path mode
- **THEN** the Work Path gains one closed four-anchor component bounded by that box and records one "Ellipse Tool" state
