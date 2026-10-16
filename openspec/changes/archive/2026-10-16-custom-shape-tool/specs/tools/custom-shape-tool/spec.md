## ADDED Requirements

### Requirement: Custom Shape tool

The Custom Shape tool SHALL offer a picker of the built-in shapes (Star, Heart,
Arrow, Cross, Lightning, Check) with silhouette previews. Dragging SHALL draw
the chosen shape stretched to the dragged box; Shift held SHALL keep its
designed proportions and Alt held SHALL grow it from the press point. In Shape
mode the layer SHALL be named "Shape N". A click SHALL open "Create Custom
Shape" with Width, Height, and From Center.

#### Scenario: Shapes fill their boxes

- **WHEN** the `pictura_core::shape::custom` unit tests draw every shape into a 100 x 50 box and Shift-drag the heart
- **THEN** every outline touches all four sides of its box and the heart keeps its designed proportions, reaching the pointer

#### Scenario: Custom Shape in the app

- **WHEN** the `tst_shape_tools` test picks the Heart, drags it in Shape mode, and clicks with a 20 x 20 Create Custom Shape dialog
- **THEN** the picker lists six shapes with icons, the drag adds a non-live "Shape 1" layer filled inside the heart as one "Shape Tool" state, and the click adds "Shape 2"
