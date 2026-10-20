## MODIFIED Requirements

### Requirement: Warp resampling op

`pictura_render::transform_layer_warp(doc, path, warp: &Warp, quad:
Option<[(f64, f64); 4]>, interpolation) -> bool` SHALL resample the layer at
`path` through `warp.map`, which is document-space, followed by the homography
sending the warp box's corners onto `quad` (none when `quad` is `None`), ported
from photocraft's mesh warp. The source rect SHALL be cut
into a grid of cells a few pixels across in both source and destination. Each
cell's two triangles SHALL be rasterized at pixel centres, with the source
position interpolated barycentrically and the planes sampled premultiplied by
alpha (taps outside the source read zero). Where the warp shrinks the source, a
pixel SHALL average bilinear subsamples over its source footprint. Where the
warp folds, later cells SHALL draw over earlier ones. The layer mask SHALL be
warped with full coverage and flattened onto its default colour; a mask point
outside the warp box SHALL move with the nearest box point. A retained
16/32-bit store SHALL be resampled through the same triangles. The result rect
SHALL be the integer box round the mapped vertices, and SHALL be held to the
pixel-area ceiling of the other transform ops. The op SHALL refuse, leaving
`doc` bit-identical, for every case `transform_layer` refuses, and also for an
identity warp with no box change, a malformed warp, a singular `quad`, an
out-of-range or oversized warped box, and a layer mask with area that cannot be
warped. It SHALL NOT
recomposite.

#### Scenario: An identity warp changes nothing [twe_op_identity]

- **WHEN** the op runs with `Warp::none`, an identity custom mesh, or a preset at bend 0
- **THEN** it returns false and the document is unchanged

#### Scenario: A translating mesh moves pixels exactly [twe_op_affine]

- **WHEN** the op runs with a custom mesh that shifts the box by whole pixels, under Nearest, Bilinear, or Bicubic
- **THEN** the layer rect is shifted and every channel is byte-identical to the source

#### Scenario: Arc keeps its bottom corners and spreads its top [twe_op_arc]

- **WHEN** Arc at 50% warps a raster layer
- **THEN** the bottom corners map to themselves and the new rect extends above, left of, and right of the box

#### Scenario: A thin layer bent far keeps the curve [twe_op_thin]

- **WHEN** a 3 px column is stretched 100× and bent into an arch
- **THEN** the middle is lifted and the space under the arch is empty

#### Scenario: A shrinking warp is prefiltered [twe_op_shrink]

- **WHEN** a grid of 1 px lines is warped to about one fifth of its size
- **THEN** opaque output pixels are within 0.25 of the area average

#### Scenario: A fold draws the later cell on top [twe_op_fold]

- **WHEN** the left half of a source is mirrored onto the strip its right half covers
- **THEN** the sampled pixel shows the right (later) half

#### Scenario: The layer mask moves and keeps its default [twe_op_mask]

- **WHEN** a layer whose mask is black inside and white by default is shifted by a warp
- **THEN** the mask rect shifts, its default stays white, and its pixels stay black

#### Scenario: A high-depth store is resampled [twe_op_native]

- **WHEN** a 16-bit layer with a retained store is warped
- **THEN** the store's rect equals the new layer rect, every plane has its area, and a save writes 16-bit

#### Scenario: A projective box is kept exactly [twe_op_projective]

- **WHEN** an identity mesh is warped with a perspective `quad`
- **THEN** interior pixels sample where the inverse homography says, within 3 levels of a gradient

#### Scenario: A mask wider than the layer moves with it [twe_op_mask_beyond]

- **WHEN** a mask reaching 6 px past the layer is shifted by a translating warp
- **THEN** the whole mask rect shifts and its pixels are kept

#### Scenario: A mask that cannot follow refuses the warp [twe_op_mask_refuse]

- **WHEN** the pixels can move but the mask's warped box is out of range
- **THEN** the op returns false and the document is unchanged

#### Scenario: An oversized result is refused [twe_op_area]

- **WHEN** a mesh maps a 4 px box onto 65536 px square
- **THEN** triangulation and the op refuse and the document is unchanged
