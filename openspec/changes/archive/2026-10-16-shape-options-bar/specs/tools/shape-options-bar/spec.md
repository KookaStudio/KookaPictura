## ADDED Requirements

### Requirement: Shape options bar

Every shape tool's options bar SHALL offer Mode, Fill (None or a solid colour),
Stroke (None or a solid colour, width, and Align Inside / Center / Outside),
W and H with a proportion link, a geometry gear, Align Edges, and the tool's
own field (Radius, Sides, Weight, or the Shape picker); the path operation,
alignment, and arrangement menus, Gradient and Pattern, and Dashed and Dotted
lines SHALL be shown disabled. New shapes SHALL take the bar's fill, stroke,
geometry, and Align Edges. While a shape layer is active the bar SHALL show its
size, fill, and stroke, and editing them SHALL restyle or resize that layer,
one history state per change.

#### Scenario: Draw, mirror, restyle, resize, constrain

- **WHEN** the `tst_shape_tools` test draws a rectangle with no fill and a 2 px green inside stroke at fractional coordinates, then fills it red, removes the stroke, resizes it with the link on, and draws with Fixed Size from the centre and with Square
- **THEN** the shape shows only a green inner border on whole-pixel edges, the bar shows its 30 x 21 size, the fill and stroke changes each record one state and repaint it, the width of 60 makes it 60 x 42 as one "Resize Shape" state, the fixed shape is 20 x 10 centred on the pointer, and the square drag is 20 x 20

#### Scenario: The stroke and no-fill read elsewhere

- **WHEN** the `vector_mask_oracle` test saves an unfilled shape with a 3 px inside stroke and reads it with psd-tools
- **THEN** psd-tools reports an enabled inside Stroke effect of 3 px in the stroke colour and a fill opacity of 0
