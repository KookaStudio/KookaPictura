## ADDED Requirements

### Requirement: Mask thumbnail activation routes edits to coverage

Clicking a raster mask thumbnail SHALL activate the clicked row, make that row's
mask the edit target, and keep the target after the selection model settles.
The Brush, Pencil, and Eraser SHALL then write mask coverage rather than the
layer's pixels.

Ctrl-clicking a raster mask thumbnail SHALL load the mask coverage as the pixel
selection and SHALL NOT toggle the row's membership in the layer selection.

An active mask target SHALL be a valid target for Image > Adjustments and filters
regardless of the owning layer kind, including fill, adjustment, and group rows.

#### Scenario: A non-active row's mask becomes the target

- **WHEN** another row is active and the user clicks a masked row's mask thumbnail
- **THEN** the clicked row becomes active, its mask becomes the edit target, and
  a brush stroke changes mask coverage instead of layer pixels

#### Scenario: Ctrl-click loads the mask selection

- **WHEN** the user Ctrl-clicks a raster mask thumbnail
- **THEN** the mask's white coverage becomes the pixel selection and the layer
  selection is unchanged

#### Scenario: Ctrl-I inverts a fill mask

- **WHEN** a fill's raster mask is the edit target and the user presses Ctrl-I
- **THEN** the mask coverage is inverted in one `Invert` state and the fill
  pixels are untouched
