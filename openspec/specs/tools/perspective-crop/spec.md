# tools/perspective-crop Specification

## Purpose
The Perspective Crop tool: straighten and crop a user-marked quadrilateral through a homography warp of every layer.

## Requirements

### Requirement: Perspective crop warp

`pictura_render::perspective_crop(doc, quad)` SHALL map the quad (TL, TR, BR,
BL, document pixels) onto a `w×h` canvas, where `perspective_crop_size(quad)` is
the rounded longer length of each pair of opposite edges clamped to 1–30 000,
unless the caller supplies an explicit output size, in which case that size
(also clamped) SHALL be used. Every pixel layer's channels and layer mask SHALL
be resampled bilinearly through the inverse homography onto the new canvas, with
alpha 0 outside the source for a layer with alpha, white for a layer without
alpha, and the mask's default colour for a mask; document extra channels SHALL
warp the same way; channel-less layers SHALL be unchanged. It SHALL refuse
without change a degenerate quad and any document for which
`perspective_crop_refusal` gives a reason (live type, smart objects, vector
masks, retained 16/32-bit samples).

#### Scenario: The canvas quad is the identity

- **WHEN** a document is perspective-cropped to its own corners
- **THEN** its size and pixels are unchanged

#### Scenario: A skewed region is straightened

- **WHEN** a trapezoid's corners are the quad
- **THEN** the document takes `perspective_crop_size` and the trapezoid fills its middle row

#### Scenario: An explicit output size wins

- **WHEN** a perspective crop is applied with an explicit width and height
- **THEN** the document takes exactly that size

#### Scenario: Refusals change nothing

- **WHEN** the quad is degenerate or the document holds a type layer
- **THEN** `perspective_crop` returns false and the document is unchanged

### Requirement: Perspective Crop tool

The Perspective Crop tool SHALL stage its quad from four separate corner clicks
in top-left, top-right, bottom-right, bottom-left order, drawing each placed
point as it is clicked and closing the quad on the fourth. While the quad is
incomplete a click SHALL add the next corner; once complete, a corner dragged
within 8 screen pixels of it SHALL move that corner, and a drag inside the quad
SHALL move the whole quad. Enter or a double-click inside the quad SHALL commit
it as one "Perspective Crop" history state and drop the selection; a refused
commit SHALL keep the quad staged and report why; Escape SHALL discard it.
Starting a quad on a refused document SHALL report the reason. Each placed or
dragged corner SHALL snap to the canvas edges and corners (and the composite
bounding box) within a 6-screen-pixel threshold. The canvas SHALL draw the
placed points and, once the quad is complete, shade only the canvas area outside
the quad (not the workspace), draw its edges, and draw a grid at a fixed
physical spacing (one inch, from the document resolution) clipped to the quad,
so a larger quad shows more grid lines at the same spacing.

#### Scenario: Enter commits a staged quad

- **WHEN** the `crop_group` self-test clicks four corners over a 20×20 black square in a 40×40 image and presses Enter
- **THEN** one "Perspective Crop" state is added and the document is 20×20 and black

#### Scenario: Four clicks stage the quad

- **WHEN** four corners are clicked in order
- **THEN** the quad closes on the fourth click and no state is added until a commit

#### Scenario: A degenerate quad stays staged

- **WHEN** a corner is dragged onto its neighbour and Enter is pressed
- **THEN** no state is added, the size is unchanged, and the quad is still shown

#### Scenario: A corner snaps to the canvas

- **WHEN** a corner is clicked within the threshold of a canvas corner
- **THEN** the corner lands exactly on the canvas corner

### Requirement: Perspective Crop pointer

With the Perspective Crop tool active, the canvas pointer SHALL be a precise
crosshair (hot spot at its centre) over the canvas and inside the quad, SHALL
change to the move cursor while hovering within 8 screen pixels of a staged
quad's corner handle, and SHALL be the eight-way rotate cursor when outside the
completed quad beyond the resize margin, where dragging SHALL rotate the quad
about its centre.

#### Scenario: Crosshair and corner cursor

- **WHEN** a quad is staged and the pointer hovers inside it and then over a corner
- **THEN** the pointer is the crosshair inside and the move cursor over the corner

#### Scenario: Rotate outside the quad

- **WHEN** the pointer is outside a completed quad beyond the resize margin
- **THEN** the cursor is the rotate cursor and dragging rotates the quad about its centre

### Requirement: Perspective Crop options

The Perspective Crop tool SHALL show an options bar with `W` and `H` output-pixel
fields, a swap control, `Resolution` and its unit, `Front Image`, `Clear`, and a
`Show Grid` toggle, plus `Cancel` and `Apply` icon buttons visible only while a
quad is complete. The `W`/`H` fields SHALL hold the final output pixels: blank
means the output is computed from the quad automatically, a value forces that
dimension, and each field SHALL be clearable individually or all at once with
`Clear` (which clears only the input values). `Front Image` SHALL set the
resolution from the front document. `Show Grid` SHALL toggle the grid.

#### Scenario: Blank output size is automatic

- **WHEN** a quad is committed with the W/H fields blank
- **THEN** the output size is computed from the quad

#### Scenario: A typed size is used

- **WHEN** a quad is committed with a value typed into W and H
- **THEN** the output takes those pixel dimensions

#### Scenario: Clear empties the fields

- **WHEN** `Clear` is clicked
- **THEN** the W/H fields are empty and the resolution is unchanged
