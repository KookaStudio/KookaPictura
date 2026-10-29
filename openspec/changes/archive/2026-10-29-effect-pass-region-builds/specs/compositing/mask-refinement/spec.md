# Spec Delta

## MODIFIED Requirements

### Requirement: Mask feather

A mask's feather radius SHALL blur its coverage with the selection feather's
Gaussian (`sigma = radius / 2`, clamped at 250 px) before density applies. A
raster mask's ramp MUST extend past its rect into the default colour, and a
vector mask's coverage MUST be rasterized before blurring. Feather 0 MUST leave
the mask unchanged. The blurred plane SHALL be cached and MUST be rebuilt after
the mask's data, rect, or feather changes. The plane SHALL be bounded by the
mask's bounds intersected with the sampled region, grown by the feather reach;
samples outside the plane MUST equal the mask's outside value. A region
composite SHALL build only the plane for its region, and every sample inside the
sampled region MUST equal the same sample of a plane built over the whole mask.
The kernel is inferred.

#### Scenario: Feather softens a hard edge

- **WHEN** a mask has a hard black/white edge and feather 8
- **THEN** pixels within the ramp take intermediate values that increase monotonically across the edge

#### Scenario: Feather reaches past the mask rect

- **WHEN** a white mask with default colour 0 is feathered by 8
- **THEN** pixels just outside its rect are above 0

#### Scenario: A region composite feathers only its region

- **WHEN** a feathered mask is sampled through a sub-region composite and then
  through a full composite
- **THEN** every pixel in the sub-region equals the full composite and the
  feathered plane is bounded by the mask's bounds intersected with the region,
  grown by the feather reach

#### Scenario: A mask dab rebuilds its region

- **WHEN** a dab is painted on a feathered mask and the dab's region is
  composited
- **THEN** only that region's feathered plane is rebuilt, it is bounded by the
  mask's bounds intersected with the region grown by the feather reach, and the
  pixels equal the same slice of a full composite
