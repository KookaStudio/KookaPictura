## ADDED Requirements

### Requirement: Polygon tool

Dragging with the Polygon tool SHALL draw a regular polygon of the options
bar's Sides (3-100, default 5) centred on the press point, its corners on the
circle through the pointer and its first corner under the pointer; Shift held
SHALL snap that rotation to 15° steps.

#### Scenario: Centred and snapped polygons

- **WHEN** the `pictura_core::shape` unit tests drag a six-sided polygon from (50, 50) to (50, 20), Shift-drag one at 50°, and ask for one side
- **THEN** the first has six corners on the radius-30 circle starting at (50, 20), the second's first corner lies at 45°, and the third has three sides

#### Scenario: Polygon in the app

- **WHEN** the `tst_shape_tools` test sets Sides to 6 and drags a Polygon in Pixels mode from (50, 50) to (50, 20)
- **THEN** the foreground fills the hexagon's interior on the active layer, no layer is added, and one "Polygon Tool" state is recorded
