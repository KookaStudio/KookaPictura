# compositing/mask-refinement Specification

## Purpose
Mask density and feather: scales a layer mask's coverage and softens its edge.

## Requirements

### Requirement: Mask density

A layer mask's or vector mask's density `d` (0-255) SHALL lift its coverage
toward white before the raster and vector parts multiply:
`m' = 255 - d * (255 - m) / 255`, rounded. Density 255 MUST leave the mask
unchanged and density 0 MUST reveal the whole layer. The formula is inferred
(Adobe's is closed). A disabled mask MUST still contribute 255.

#### Scenario: Full density is the painted mask

- **WHEN** a mask pixel is 0 and density is 255
- **THEN** the composited mask value there is 0

#### Scenario: Half density reveals half

- **WHEN** a mask pixel is 0 and density is 128
- **THEN** the composited mask value there is 127

#### Scenario: Zero density reveals everything

- **WHEN** density is 0
- **THEN** every composited mask value is 255

### Requirement: Mask feather

A mask's feather radius SHALL blur its coverage with the selection feather's
Gaussian (`sigma = radius / 2`, clamped at 250 px) before density applies. A
raster mask's ramp MUST extend past its rect into the default colour, and a
vector mask's coverage MUST be rasterized before blurring. Feather 0 MUST leave
the mask unchanged. The blurred plane SHALL be cached and MUST be rebuilt after
the mask's data, rect, or feather changes. The kernel is inferred.

#### Scenario: Feather softens a hard edge

- **WHEN** a mask has a hard black/white edge and feather 8
- **THEN** pixels within the ramp take intermediate values that increase monotonically across the edge

#### Scenario: Feather reaches past the mask rect

- **WHEN** a white mask with default colour 0 is feathered by 8
- **THEN** pixels just outside its rect are above 0

### Requirement: Mask refinement edits

The document SHALL provide `set_layer_mask_density`, `set_layer_mask_feather`,
`set_vector_mask_density`, and `set_vector_mask_feather`, each recorded as one
undoable state and each returning false when the mask is absent or the value
is unchanged, and `invert_layer_mask`, which replaces every mask sample `m`
and the default colour with `255 - m` as one undoable "Invert" state. Feather
MUST reject negative and non-finite radii.

#### Scenario: Density change undoes

- **WHEN** a mask's density is set to 128 and then undone
- **THEN** the density is 255 again

#### Scenario: Invert flips the mask permanently

- **WHEN** `invert_layer_mask` runs on a mask whose pixel is 40 with default colour 0
- **THEN** the pixel is 215 and the default colour is 255, and undo restores both
