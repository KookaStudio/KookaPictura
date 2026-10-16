## ADDED Requirements

### Requirement: Rectangle tool

Dragging with the Rectangle tool SHALL draw the axis-aligned rectangle the
drag spans; Shift held SHALL square it by the drag's longer side and Alt held
SHALL grow it from the press point. A drag enclosing no area SHALL draw
nothing and record nothing.

#### Scenario: Square and centred rectangles

- **WHEN** the `pictura_core::shape` unit tests drag a rectangle plainly, with Shift, and with Alt
- **THEN** the plain outline is the four corners clockwise from the top-left, the Shift outline is a square of the longer side, and the Alt outline is centred on the press point

### Requirement: Shape tool modes

The shape tools SHALL land a drag in the options bar's Mode, shared by the
Rectangle, Rounded Rectangle, Ellipse, and Polygon: Shape SHALL insert a
solid-color fill layer in the foreground color above the active layer, named
after the tool, and cut to the outline by a `vmsk` vector mask; Path SHALL add
the outline to the Work Path as a closed component and change no pixels; Pixels
SHALL paint the outline's anti-aliased interior in the foreground color onto
the active pixel layer, through the selection, adding no layer. Each SHALL
record exactly one `"<Tool> Tool"` state. The outline SHALL be previewed on the
canvas during the drag and cleared on release.

#### Scenario: Shape, Path, and Pixels

- **WHEN** the `tst_shape_tools` test draws a Polygon in Pixels mode, an Ellipse and a Rounded Rectangle in Path mode, and a Rectangle in Shape mode, then clicks and undoes
- **THEN** the Polygon paints red pixels with no new layer, the Path drags add closed components without changing pixels, the Rectangle adds a "Rectangle 1" layer showing red only inside the rectangle while the preview is drawn mid-drag and cleared after, each drag records one "<Tool> Tool" state, the click records nothing, and undo removes the layer

#### Scenario: An authored shape layer reads elsewhere

- **WHEN** the `vector_mask_oracle` test saves a document with a Rectangle shape layer and reads it with psd-tools
- **THEN** psd-tools reports one version-3 vector mask with one closed subpath on the rectangle's corners covering its pixels
