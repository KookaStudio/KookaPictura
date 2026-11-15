# Spec Delta

## ADDED Requirements

### Requirement: Save a selection into an existing raster layer mask

The Save Selection dialog SHALL offer every existing raster layer mask in the
chosen document as a destination. Vector masks, colour channels, and documents
with no raster layer mask MUST NOT offer a mask target. The operation SHALL be
Replace, Add, Subtract, or Intersect, with the destination mask's current
coverage as the first operand. Selected coverage SHALL be written as white and
unselected coverage as black; Add, Subtract, and Intersect SHALL use the
selection-model algebra. The write SHALL preserve the mask's density, feather,
flags, and enabled state, SHALL normalize its stored rectangle to the document
canvas, and SHALL drop a retained native mask plane. It SHALL record exactly one
history state; a missing target, a different document size, or no active
selection MUST be refused without recording.

#### Scenario: Replace reveals the selected area

- **WHEN** a selection is saved into an existing raster layer mask with Replace
- **THEN** selected pixels become white, unselected pixels become black, and the layer's visibility follows the new mask

#### Scenario: Boolean operations combine with the mask

- **WHEN** a selection is saved into an existing raster layer mask with Add, Subtract, or Intersect
- **THEN** the mask coverage becomes the maximum, the saturating difference, or the rounded product with the incoming coverage

#### Scenario: Mask metadata is preserved

- **WHEN** a selection is saved into a mask with non-default density, feather, flags, or disabled state
- **THEN** those settings are unchanged after the save

#### Scenario: A disabled mask stays disabled

- **WHEN** the destination mask is disabled
- **THEN** its coverage changes but it remains disabled and does not alter the composite

#### Scenario: Saving into a mask is one state

- **WHEN** a selection is saved into an existing raster layer mask
- **THEN** the history gains exactly one state and undo restores the previous mask bytes and metadata

#### Scenario: A missing mask target is refused

- **WHEN** the chosen mask no longer exists or belongs to a different-size document
- **THEN** the save is refused and no history state is recorded

## MODIFIED Requirements

### Requirement: Mask thumbnail activation routes edits to coverage

Clicking a raster mask thumbnail SHALL activate the clicked row, make that row's
mask the edit target, and keep the target after the selection model settles.
The Brush, Pencil, and Eraser SHALL then write mask coverage rather than the
layer's pixels.

Ctrl-clicking a raster mask thumbnail SHALL load the mask coverage as the pixel
selection with New and SHALL NOT toggle the row's membership in the layer
selection. Ctrl+Shift-clicking SHALL add the mask coverage to the selection;
Ctrl+Alt-clicking SHALL subtract it; Ctrl+Shift+Alt-clicking SHALL intersect it.
Each load SHALL record exactly one "Load Selection" state and SHALL NOT toggle
the row's membership.

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

#### Scenario: Ctrl+Shift adds the mask coverage

- **WHEN** a selection exists and the user Ctrl+Shift-clicks a raster mask thumbnail
- **THEN** the mask coverage is added to the selection and one "Load Selection" state is recorded

#### Scenario: Ctrl+Alt subtracts the mask coverage

- **WHEN** a selection exists and the user Ctrl+Alt-clicks a raster mask thumbnail
- **THEN** the mask coverage is subtracted from the selection and one "Load Selection" state is recorded

#### Scenario: Ctrl+Shift+Alt intersects the mask coverage

- **WHEN** a selection exists and the user Ctrl+Shift+Alt-clicks a raster mask thumbnail
- **THEN** the selection becomes its intersection with the mask coverage and one "Load Selection" state is recorded

#### Scenario: Ctrl-I inverts a fill mask

- **WHEN** a fill's raster mask is the edit target and the user presses Ctrl-I
- **THEN** the mask coverage is inverted in one `Invert` state and the fill
  pixels are untouched
