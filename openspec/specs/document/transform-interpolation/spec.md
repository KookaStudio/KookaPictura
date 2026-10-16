# document/transform-interpolation Specification

## Purpose
The CS6 resample-method choice for layer transforms: six interpolation ids,
their kernels, and interpolation-parameterised variants of the similarity,
projective, and warp ops.

## Requirements

### Requirement: Interpolation method set and kernels

`pictura_render` SHALL expose `pub enum Interpolation` with the variants
`Nearest`, `Bilinear`, `Bicubic`, `BicubicSmoother`, `BicubicSharper`, and
`BicubicAutomatic`, a stable id for each (`nearest`, `bilinear`, `bicubic`,
`bicubic-smoother`, `bicubic-sharper`, `bicubic-automatic`), and a `from_id`
constructor over the full set. Sampling a plane SHALL interpret the
inverse-mapped source position in the pixel-centre convention already used by
the bilinear kernel (`u = lx − 0.5`, taps edge-clamped):

- `Nearest` SHALL replicate the source pixel nearest the sample point.
- `Bilinear` SHALL keep the existing 2×2 weighted average.
- `Bicubic`, `BicubicSmoother`, and `BicubicSharper` SHALL be separable
  Mitchell–Netravali cubic convolutions with `(B, C)` of `(0, 1/2)`, `(1, 0)`,
  and `(0, 3/4)` respectively.
- Every sample SHALL be rounded and clamped to the plane's range, and a
  destination whose source point lies outside the source plane SHALL remain
  `0` for every method.

The bicubic parameters are inferred approximations of Adobe's closed kernels.
Integer-aligned sampling SHALL be byte-exact for every interpolating method
(`Nearest`, `Bilinear`, and the three cubics at whole-pixel offsets).

#### Scenario: Nearest scaling replicates pixels

- **WHEN** a 400% scale of a two-colour checkerboard is sampled with `Nearest`
- **THEN** every output pixel equals a source pixel and no intermediate colour appears

#### Scenario: Bicubic differs from bilinear on a scale

- **WHEN** the same 200% scale is sampled with `Bilinear` and `Bicubic`
- **THEN** at least one output pixel differs

#### Scenario: Integer sampling is exact

- **WHEN** a plane is sampled at whole-pixel positions with `Nearest`, `Bilinear`, or `Bicubic`
- **THEN** every sample equals the corresponding source pixel

#### Scenario: Unknown ids are rejected

- **WHEN** `from_id` is called with a string outside the six ids
- **THEN** it returns `None`

### Requirement: Automatic method resolution

`BicubicAutomatic` SHALL resolve per op to `BicubicSharper` when the
destination pixel count is below the source pixel count, to `BicubicSmoother`
when it is above, and to `Bicubic` when they are equal.

#### Scenario: Downscale sharpens, upscale smooths

- **WHEN** a scale below 100% is committed with `BicubicAutomatic`
- **THEN** the result equals the same scale committed with `BicubicSharper`, and an above-100% scale equals `BicubicSmoother`

### Requirement: Interpolation-parameterized transform ops

`pictura_render` SHALL provide `transform_layer_with(doc, path, transform,
interpolation)`, `transform_layer_quad_with(doc, path, quad, interpolation)`,
and `transform_layer_warp_with(doc, path, mesh, params, interpolation)`. Each
SHALL follow the exact refusal, materialization, bounding-box, coverage, and
write-back contract of its unparameterised counterpart (`transform_layer`,
`transform_layer_quad`, `transform_layer_warp`) and SHALL resample every channel
plane, the layer mask, and the retained native store with the given method. A
refused call MUST leave `doc` bit-identical. The unparameterised ops SHALL
remain and SHALL equal their `_with` counterparts at `Interpolation::Bilinear`.

#### Scenario: A refused interpolated op mutates nothing

- **WHEN** `transform_layer_with` runs on a Background layer or with a degenerate transform
- **THEN** it returns false and every field of `doc` equals its pre-call value

#### Scenario: The legacy op is the bilinear variant

- **WHEN** the same transform is applied through `transform_layer` and `transform_layer_with(..., Bilinear)`
- **THEN** the resulting channel bytes are identical

#### Scenario: The method reaches every plane

- **WHEN** a masked layer with a retained native store is scaled with `Nearest`
- **THEN** the colour channels, the mask plane, and the native store are all resampled with the nearest kernel
