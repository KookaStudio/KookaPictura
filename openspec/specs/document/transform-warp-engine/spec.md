# document/transform-warp-engine Specification

## Purpose
The Edit ▸ Transform ▸ Warp engine ported from photocraft: warp styles and the
bicubic Bezier mesh (`pictura_core::warp`), and the adaptive mesh resampler
(`pictura_render::transform_layer_warp`).

## Requirements

### Requirement: Warp geometry

`pictura_core::warp` SHALL provide `WarpStyle` (`None`, `Custom`, and the
fifteen presets in Photoshop's menu order, each with a PSD `warpStyle` name, a
stable id, and a label), `BezierMesh` (a grid of bicubic Bezier patches with
control points in document space), and `Warp` (style, bend, H/V distortion,
orientation, source bounds, and an optional custom mesh) with a forward `map`.
Every preset SHALL be the identity at bend 0. A preset SHALL be one bicubic
patch whose control grid follows the bend, ported from photocraft's fits to
Photoshop renders: an approximation, not verified parity. Subdividing a mesh
SHALL NOT change its shape. Dragging an anchor or handle, or pulling the
surface, SHALL keep the surface smooth across the patch lines, and a pull SHALL
put the grabbed surface point exactly at the pointer.

#### Scenario: Presets at bend zero are the identity [twe_identity]

- **WHEN** any preset is evaluated at bend 0
- **THEN** every point maps to itself

#### Scenario: Presets follow the measured Photoshop points [twe_presets]

- **WHEN** each preset at its sampled bend maps the measured source points
- **THEN** each lands within the test's tolerance of where Photoshop put it

#### Scenario: Subdivision is exact [twe_split]

- **WHEN** a bent mesh is split at an interior parameter on each axis
- **THEN** the surface is unchanged within `1e-9` and `param_at` inverts `eval`

#### Scenario: A pull lands the grabbed point and stays smooth [twe_pull]

- **WHEN** the surface is pulled at an interior point by an offset
- **THEN** that surface point moves by exactly the offset and the grid lines keep their tangents across anchors

### Requirement: Warp resampling op

`pictura_render::transform_layer_warp(doc, path, warp: &Warp, interpolation) ->
bool` SHALL resample the layer at `path` through `warp.map`, which is
document-space, ported from photocraft's mesh warp. The source rect SHALL be cut
into a grid of cells a few pixels across in both source and destination. Each
cell's two triangles SHALL be rasterized at pixel centres, with the source
position interpolated barycentrically and the planes sampled premultiplied by
alpha (taps outside the source read zero). Where the warp shrinks the source, a
pixel SHALL average bilinear subsamples over its source footprint. Where the
warp folds, later cells SHALL draw over earlier ones. The layer mask SHALL be
warped with full coverage and flattened onto its default colour. A retained
16/32-bit store SHALL be resampled through the same triangles. The result rect
SHALL be the integer box round the mapped vertices. The op SHALL refuse, leaving
`doc` bit-identical, for every case `transform_layer` refuses, and also for an
identity or malformed warp or an out-of-range warped box. It SHALL NOT
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

- **WHEN** every source column maps to the same destination column
- **THEN** the right-most source column shows

#### Scenario: The layer mask moves and keeps its default [twe_op_mask]

- **WHEN** a layer whose mask is black inside and white by default is shifted by a warp
- **THEN** the mask rect shifts, its default stays white, and its pixels stay black

#### Scenario: A high-depth store is resampled [twe_op_native]

- **WHEN** a 16-bit layer with a retained store is warped
- **THEN** the store's rect equals the new layer rect, every plane has its area, and a save writes 16-bit
