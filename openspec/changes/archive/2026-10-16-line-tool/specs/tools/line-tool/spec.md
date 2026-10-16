## ADDED Requirements

### Requirement: Line tool

Dragging with the Line tool SHALL draw a filled line of the options bar's
Weight from the press point to the pointer; Shift held SHALL snap its angle to
45°. Arrowheads switched on at the Start or End SHALL replace that end with a
tip on the end point, Width and Length percent of the weight, its base pulled
toward the tip by Concavity percent of its length. A drag with no length or a
click SHALL draw nothing.

#### Scenario: Weight, snapping, and arrowheads

- **WHEN** the `pictura_core::shape::line` unit tests draw a 6 px line, a Shift-dragged line 4.6° off horizontal, a line with both default arrowheads, and one with a 50 % concave end
- **THEN** the first is the 6 px-wide quad, the second is exactly horizontal, the third has ten anchors with 10 px-wide, 20 px-long heads tipped on the ends, and the fourth's base meets the line's edge 8 px nearer the tip

#### Scenario: Line in the app

- **WHEN** the `tst_shape_tools` test sets Weight to 4, drags a Line in Pixels mode, clicks, switches the End arrowhead on, and drags one in Path mode
- **THEN** a 4 px band is painted as one "Line Tool" state, the click opens no dialog and records nothing, and the path component has seven anchors whose head is 20 px wide
