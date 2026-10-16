## ADDED Requirements

### Requirement: Create shape dialogs

A click (a press that does not drag) with a shape tool SHALL open its Create
dialog: "Create Rectangle" and "Create Ellipse" with Width, Height, and From
Center; "Create Rounded Rectangle" adding a radius per corner; "Create Polygon"
adding Number of Sides, Smooth Corners, Star, Indent Sides By, and Smooth
Indents (the last two enabled only for a star). OK SHALL draw the shape in the
options bar's Mode with its top-left corner, or with From Center its centre, at
the click; Cancel SHALL draw nothing and record nothing.

#### Scenario: Each dialog places its shape

- **WHEN** the `tst_shape_tools` test clicks with each tool and fills its dialog (a 30 x 20 rectangle, a centred 40 x 20 ellipse, a 40 x 40 rounded rectangle with a square top-left corner, a 30 x 30 five-pointed star), and cancels one dialog
- **THEN** the rectangle layer covers exactly its box, the ellipse is centred on the click, the rounded rectangle has seven anchors, the star has ten anchors filling its box, and the cancelled dialog records nothing
