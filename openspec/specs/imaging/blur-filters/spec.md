# blur-filters Specification

## Purpose
Gaussian, Box, Motion, Radial, Average, Surface, Blur, and Blur More with alpha preservation, edge handling, and an oracle.

## Requirements

### Requirement: Blur filter application and error contract

The system SHALL provide `pictura_filters::apply(filter: &Filter, buf: &mut PixelBuffer) -> Result<(), FilterError>`. It SHALL operate in place on a planar 8-bit buffer whose `channels` is 3 (RGB) or 4 (RGBA), treating channels 1 through 3 as the color planes and channel 4, when present, as alpha. It SHALL transform every color sample or return an error; it MUST NOT partially apply a filter and then fail. Malformed buffers MUST return `FilterError` instead of panicking.

#### Scenario: Apply a blur to a 3-channel planar buffer

- **WHEN** `apply` receives a blur `Filter` variant and a 3-channel planar buffer
- **THEN** the R, G, and B planes are rewritten in place and `Ok(())` is returned

#### Scenario: Reject an unsupported channel count

- **WHEN** the buffer has a channel count other than 3 or 4
- **THEN** `apply` returns `FilterError::Unsupported` and leaves the buffer unchanged

#### Scenario: Malformed buffers error instead of panicking

- **WHEN** `apply` receives an empty buffer, or a buffer whose `data.len()` does not equal `width * height * channels`, or a buffer with zero width or height
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Gaussian blur

The system SHALL implement `Filter::GaussianBlur { radius: f64 }` as a separable Gaussian FIR convolution. The UI radius SHALL be the 3σ support (`σ = sigma_from_radius(radius)`, with σ floored at 0.1); the 1-D kernel SHALL be normalized and have `⌈3σ⌉` samples of support each side; the two passes SHALL run horizontally then vertically. `radius == 0.0` SHALL be a no-op (or near-identity within 1 LSB). A negative or non-finite radius SHALL be rejected with `FilterError::InvalidParams`. Oracle expectation: differential against ImageMagick `-gaussian-blur 0xσ` with the matching σ within an absolute tolerance of 2 per sample for `radius <= 50`.

#### Scenario: Solid color is unchanged

- **WHEN** Gaussian blur is applied to a uniform-color buffer
- **THEN** every output sample equals the input within 1 LSB

#### Scenario: An edge softens as radius grows

- **WHEN** Gaussian blur is applied to a step edge at increasing radii
- **THEN** the transition width grows monotonically with the radius

#### Scenario: Radius zero is a no-op

- **WHEN** Gaussian blur is applied with radius 0.0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Matches ImageMagick

- **WHEN** Gaussian blur with a given radius is applied to the oracle image and ImageMagick applies `-gaussian-blur 0xσ` with the matching σ
- **THEN** no sample differs by more than 2

### Requirement: Box blur

The system SHALL implement `Filter::BoxBlur { radius: u32 }` as a separable moving average over the `(2r+1)²` neighborhood, applied per color channel. `radius == 0` SHALL be a no-op. Oracle expectation: differential against ImageMagick `-statistic mean NxN` with `N = 2*radius + 1` within an absolute tolerance of 1 per sample.

#### Scenario: A uniform region keeps its exact mean

- **WHEN** Box blur is applied to a uniform-color buffer
- **THEN** every output sample equals the region mean rounded, within 1 LSB

#### Scenario: Radius zero is a no-op

- **WHEN** Box blur is applied with radius 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Matches ImageMagick

- **WHEN** Box blur with a given radius is applied to the oracle image and ImageMagick applies `-statistic mean NxN` with `N = 2*radius + 1`
- **THEN** no sample differs by more than 1

### Requirement: Motion blur

The system SHALL implement `Filter::MotionBlur { angle: f64, distance: u32 }` as a 1-D line convolution through each pixel along `angle` with length `distance`. `angle` SHALL span `-360.0..=360.0`; `distance` SHALL span `1..=999`; values outside those ranges SHALL be rejected with `FilterError::InvalidParams`. `angle == 0` SHALL streak horizontally and `angle == 90` vertically. `distance <= 1` SHALL be near-identity within 1 LSB. The kernel SHALL be supersampled to reduce aliasing. Oracle expectation: differential against ImageMagick `-motion-blur 0xN+angle` with the divergence documented if its kernel differs.

#### Scenario: Angle zero streaks horizontally

- **WHEN** Motion blur with angle 0 and a distance greater than 1 is applied to an impulse
- **THEN** the energy spreads horizontally and not vertically

#### Scenario: Angle ninety streaks vertically

- **WHEN** Motion blur with angle 90 and a distance greater than 1 is applied to an impulse
- **THEN** the energy spreads vertically and not horizontally

#### Scenario: Distance one is near-identity

- **WHEN** Motion blur is applied with distance 1
- **THEN** every output sample equals the input within 1 LSB

#### Scenario: Out-of-range distance is rejected

- **WHEN** Motion blur is applied with distance 0 or 1000
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

### Requirement: Radial blur

The system SHALL implement `Filter::RadialBlur { method: RadialMethod, amount: f64, quality: Quality }`. For `RadialMethod::Spin` it SHALL take a moving average along the polar angle over the rotation amount; for `RadialMethod::Zoom` it SHALL take a moving average along the radial direction. `amount` SHALL be finite and non-negative; for `RadialMethod::Zoom` it SHALL span `0..=100`. `Quality` (`Draft`, `Good`, `Best`) SHALL select the sampling density, with `Draft` visibly grainier than `Best` at equal amount. `amount == 0` SHALL be a no-op. Negative, non-finite, or (`Zoom`) out-of-range amounts SHALL be rejected with `FilterError::InvalidParams`. Oracle expectation: no faithful ImageMagick equivalent exists (ImageMagick `-radial-blur` semantics differ), so property and known-value tests cover the behavior and the divergence is documented.

#### Scenario: Spin smears rotationally

- **WHEN** Radial blur is applied with `RadialMethod::Spin` and a non-zero amount to a radial impulse
- **THEN** the energy spreads along concentric arcs about the center rather than radially

#### Scenario: Zoom smears radially

- **WHEN** Radial blur is applied with `RadialMethod::Zoom` and a non-zero amount to an off-center impulse
- **THEN** the energy spreads along the radial direction from the center

#### Scenario: Amount zero is identity

- **WHEN** Radial blur is applied with amount 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Radial blur has no ImageMagick operator, tolerance 0, and a non-empty no-equivalent note

### Requirement: Average blur

The system SHALL implement `Filter::Average` with no parameters. It SHALL compute the mean color over the whole buffer, per color channel, and write that mean to every pixel. Oracle expectation: no faithful ImageMagick operator exists (it is a trivial region mean), so a known-value test asserts the output is the rounded region mean.

#### Scenario: Uniform color is the exact mean

- **WHEN** Average is applied to a buffer whose color samples sum to a known per-channel mean
- **THEN** every output pixel carries that rounded mean per channel

#### Scenario: A varying region is flattened

- **WHEN** Average is applied to a buffer with more than one distinct color
- **THEN** every output pixel equals every other output pixel

### Requirement: Blur and Blur More

The system SHALL implement `Filter::Blur` and `Filter::BlurMore` with no parameters, as Gaussian Blur at fixed radii: `Blur` at `blur::BLUR_RADIUS` (2.1, σ 0.7) and `BlurMore` at `blur::BLUR_MORE_RADIUS` (6.0, σ 2.0), through the same separable kernel as `Filter::GaussianBlur`. `BlurMore`'s deviation from the input SHALL be at least as large as `Blur`'s. A uniform-color buffer SHALL be unchanged. Oracle expectation: differential against ImageMagick `-gaussian-blur 0x0.7` for `Blur` within tolerance 0 and `-gaussian-blur 0x2` for `BlurMore` within tolerance 1.

#### Scenario: Blur More is stronger than Blur

- **WHEN** `Blur` and `BlurMore` are each applied to the same edge image
- **THEN** the total deviation from the input is larger for `BlurMore` than for `Blur`

#### Scenario: Uniform color is unchanged

- **WHEN** `Blur` or `BlurMore` is applied to a uniform-color buffer
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Blur is a fixed Gaussian

- **WHEN** `Blur` and `GaussianBlur { radius: BLUR_RADIUS }` are applied to the same buffer, and likewise `BlurMore` and `GaussianBlur { radius: BLUR_MORE_RADIUS }`
- **THEN** each pair of outputs is byte-identical

#### Scenario: Matches ImageMagick

- **WHEN** `Blur` is applied to the oracle image and ImageMagick applies `-gaussian-blur 0x0.7`, and `BlurMore` against `-gaussian-blur 0x2`
- **THEN** no sample differs by more than 0 for `Blur` and 1 for `BlurMore`

### Requirement: Surface blur

The system SHALL implement `Filter::SurfaceBlur { radius: u32, threshold: u8 }` as a per-channel thresholded mean: each output sample SHALL be the truncated integer mean of the samples in the same channel, within the `(2·radius+1)²` window clipped to the image, that differ from the centre sample by at most `threshold`. Each channel SHALL decide independently which neighbours count, so averaging stops at an edge that steps by more than the threshold. `radius` SHALL span `1..=100` and `threshold` SHALL span `1..=255`; values outside those ranges SHALL be rejected with `FilterError::InvalidParams`. Oracle expectation: no faithful ImageMagick equivalent exists (ImageMagick has no thresholded-mean operator), so property tests cover edge preservation and threshold-controlled smoothing, and the divergence is documented.

#### Scenario: A small threshold preserves a step edge

- **WHEN** Surface blur with a small threshold is applied to an image containing a step edge
- **THEN** the edge contrast is preserved while the flat regions are smoothed

#### Scenario: A larger threshold smooths across the edge

- **WHEN** the threshold is increased on the same image
- **THEN** more averaging occurs across the edge than at the small threshold

#### Scenario: Invalid parameters are rejected

- **WHEN** Surface blur is applied with radius 0 or threshold 0
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

### Requirement: Clamp-to-edge borders and tiny images

Every blur kernel except Surface Blur SHALL sample with clamp-to-edge at the image borders, so an out-of-range index maps to the nearest edge sample, and SHALL clamp its kernel support to the available source. Surface Blur SHALL clip its window to the image, averaging only the neighbours inside it. A 1×1 image and a 1-pixel-wide or 1-pixel-tall image MUST NOT panic and MUST return either a result or a typed error.

#### Scenario: A 1x1 image does not panic

- **WHEN** every blur variant is applied to a 1×1 buffer
- **THEN** no variant panics and each returns `Ok(())` or a `FilterError`

#### Scenario: Border pixels clamp to the edge

- **WHEN** a blur other than Surface Blur is applied to a small buffer and a pixel at the border is inspected
- **THEN** the result is consistent with repeating the edge sample rather than reading out of bounds

#### Scenario: Surface Blur averages inside the image

- **WHEN** Surface Blur is applied and a pixel at the border is inspected
- **THEN** the result is the mean of the in-threshold samples inside the image, with no edge sample counted more than once

### Requirement: Deterministic blur output

Blur filters SHALL be deterministic. The same filter applied to equal input buffers SHALL produce bit-identical output every run, with no random, time, or thread-order dependence.

#### Scenario: Repeated runs match

- **WHEN** each blur variant is applied to two clones of one buffer
- **THEN** the two output buffers are bit-identical

### Requirement: Blur ImageMagick oracle and no-equivalent classification

The system SHALL ship `scripts/filter_oracle.py`, which applies an ImageMagick operator to a raw 8-bit image, and `crates/pictura-filters/tests/oracle.rs`, which diffs `apply` against it. The mapping table SHALL have exactly one row per `Filter` variant. Gaussian, Box, Blur, and Blur More SHALL be diffed (`-gaussian-blur 0xσ`, `-statistic mean NxN`, `-gaussian-blur 0x0.7`, `-gaussian-blur 0x2`) within their stated tolerances. Motion, Average, Radial, and Surface SHALL be classified as having no faithful ImageMagick operator and SHALL use tolerance 0 with property or known-value tests — for Motion because ImageMagick's `-motion-blur` is a one-sided Gaussian line kernel, not a symmetric uniform streak. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table covers every blur variant

- **WHEN** the oracle tests run
- **THEN** every blur variant has a table row, each no-equivalent row has tolerance 0, and each row carries a non-empty note

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes

### Requirement: Shape blur

The system SHALL implement `Filter::ShapeBlur { shape: usize, radius: u32 }` as a convolution of each colour plane with the Custom Shape `shape` (an index into `CUSTOM_SHAPE_NAMES`) rasterized as antialiased coverage weights into a `(2r+1)²` square at its designed proportions, its longer side spanning the square. The kernel SHALL be rotated 180° before it is applied, so a single bright pixel spreads into the shape as drawn. Each output sample SHALL be `Σ w·v / Σ w`, rounded, with clamp-to-edge borders; alpha SHALL NOT be modified. `radius == 0` SHALL be a no-op. A shape index outside the catalogue or a radius above 1000 SHALL be refused with `FilterError::InvalidParams`. Oracle expectation: differential against ImageMagick `-define convolve:scale=! -morphology Convolve SxS:{weights}` with the same coverage weights and `S = 2*radius + 1`, exact.

#### Scenario: A flat region is unchanged

- **WHEN** Shape blur is applied to a uniform-color buffer with any shape and radius
- **THEN** every sample keeps its value

#### Scenario: A bright pixel takes the shape unflipped

- **WHEN** Shape blur with the right-pointing Arrow is applied to a black buffer holding one white pixel
- **THEN** the lit area is wide to the right of the pixel (the arrowhead) and a thin shaft to its left

#### Scenario: Radius zero is a no-op

- **WHEN** Shape blur is applied with radius 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Bad parameters are refused

- **WHEN** Shape blur is applied with a shape index past the catalogue or a radius of 1001
- **THEN** the call returns `FilterError::InvalidParams`

#### Scenario: Matches ImageMagick

- **WHEN** Shape blur with a given shape and radius is applied to the oracle image and ImageMagick applies `-morphology Convolve` with the same coverage weights, normalized
- **THEN** no sample differs

### Requirement: Smart blur

The system SHALL implement `Filter::SmartBlur { radius: f64, threshold: f64, quality: Quality, mode: SmartBlurMode }` on the colour planes; alpha SHALL NOT be modified. In `Normal` mode each output pixel SHALL be the Gaussian-weighted mean (σ = radius / 3) of the taps within the disc of `radius` whose every colour channel differs from the centre by at most `threshold` levels, with clamp-to-edge borders. `quality` Draft, Good and Best (CS6's Low, Medium and High) SHALL sample the disc with a step chosen so each side carries at most 3, 6 or 12 taps respectively. A pixel SHALL be an edge when any 4-neighbour differs from it by more than `threshold` levels in some channel. `EdgeOnly` SHALL write 255 to every colour plane on edges and 0 elsewhere; `OverlayEdge` SHALL write 255 on edges and the `Normal` result elsewhere. A `radius` or `threshold` outside `0.1..=100` SHALL be refused with `FilterError::InvalidParams`. Oracle expectation: none (ImageMagick's `-selective-blur` gates on luma and draws no edges); unit tests and kernel digests guard it.

#### Scenario: Noise below the threshold smooths and the step survives

- **WHEN** Smart blur in Normal mode is applied to a two-tone step whose halves carry jitter smaller than the threshold
- **THEN** each half's spread shrinks and no sample crosses to the other half's range

#### Scenario: The threshold gates a step

- **WHEN** Normal mode is applied to a 50-level step with thresholds 49.9 and 50
- **THEN** the step is unchanged at 49.9 and blurred at 50

#### Scenario: Edge Only marks the step

- **WHEN** Edge Only is applied to a noisy step whose jitter is below the threshold
- **THEN** the two columns either side of the step are 255 and every other sample is 0

#### Scenario: Overlay Edge is the blur with white edges

- **WHEN** Normal, Edge Only and Overlay Edge are applied with the same parameters
- **THEN** Overlay Edge equals 255 wherever Edge Only is 255 and the Normal result elsewhere

#### Scenario: Quality only matters past its tap budget

- **WHEN** Smart blur is applied at radius 3 with Draft and with Best
- **THEN** the outputs are identical, while at radius 40 Draft samples fewer taps than Good and Good fewer than Best

#### Scenario: Bad parameters are refused

- **WHEN** Smart blur is applied with a radius or threshold of 0, above 100, or NaN
- **THEN** the call returns `FilterError::InvalidParams`

### Requirement: Field blur (pin-interpolated variable blur)

The system SHALL implement `Filter::FieldBlur { pins: Vec<BlurPin> }` with
`BlurPin { x: f64, y: f64, blur: f64 }` at the crate root. `x` and `y` SHALL
be fractions of the buffer (0..1 inside the image; a pin outside SHALL be
allowed so it can anchor the border), and `blur` SHALL be a radius in buffer
pixels spanning `0..=MAX_FIELD_RADIUS` (500). At most `MAX_FIELD_PINS` (16)
pins SHALL be accepted. Each pixel's blur radius SHALL be the
inverse-square-distance weighted average of the pin amounts
(`r(p) = Σ wᵢ·blurᵢ / Σ wᵢ`, `wᵢ = 1/(dᵢ² + ε)`), so the field is that pin's
amount at a pin and a smooth gradient between pins. The kernel SHALL apply a
variable-radius blur whose strength follows `r(p)` and SHALL leave a pixel
with `r(p) = 0` unchanged; an empty pin list SHALL be a no-op. The kernel
SHALL blur every plane it is given, alpha included, so a layer's edge
feathers. Non-finite coordinates or amounts, an amount above `MAX_FIELD_RADIUS`,
and more than `MAX_FIELD_PINS` pins SHALL be rejected with
`FilterError::InvalidParams` before any pixel is written. The kernel SHALL be
deterministic: equal inputs SHALL produce bit-identical output. Oracle
expectation: no faithful ImageMagick operator exists (ImageMagick has no
pin-interpolated variable blur), so property and known-value tests cover the
behavior and the mapping row is no-equivalent with tolerance 0 and a
non-empty note.

#### Scenario: A pin gradient blurs more toward the larger amount

- **WHEN** Field Blur is applied with one pin at the left with blur 0 and one
  pin at the right with blur 40 to an edge or impulse buffer
- **THEN** the left region stays sharp, the right region's transition is
  wider, and the transition widens monotonically from left to right

#### Scenario: A pin outside the image anchors the border

- **WHEN** Field Blur is applied with a sharp pin outside one edge and a
  blurred pin inside the opposite half
- **THEN** the border near the outside pin stays sharper than the interior
  near the blurred pin

#### Scenario: Zero pins is a no-op

- **WHEN** Field Blur is applied with no pins
- **THEN** every byte of the buffer is unchanged

#### Scenario: Invalid parameters are rejected without mutation

- **WHEN** Field Blur is applied with a non-finite coordinate, a negative or
  over-500 blur amount, or more than 16 pins
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is
  bit-exactly unchanged

#### Scenario: Alpha blurs with the colour

- **WHEN** Field Blur is applied to a four-channel buffer with a hard alpha
  edge under a blurring pin, and again with a zero amount
- **THEN** the edge feathers, and the zero amount leaves alpha bit-identical

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Field Blur has no ImageMagick operator, tolerance 0, and a
  non-empty no-equivalent note

### Requirement: Field Blur Blur Effects and Noise passes

When `bokeh.enabled`, the kernel SHALL apply the Blur Gallery's Blur Effects
to the blurred pixels only (field weight > 0): `light_bokeh` (`0..=100`)
SHALL multiply a pixel by `1 + 2 × light_bokeh/100 × luma/255 × weight`, so a
bright, well-blurred pixel blooms brilliantly toward white; `bokeh_color`
(`0..=100`) SHALL push the lightened, non-clipped pixels away from their
luminance, and `light_range` (two `0..=255` values) SHALL gate both to that
tonal window. When `noise.enabled`, the kernel SHALL re-inject grain scaled
by `amount` (`0..=100`) and the field weight, damped in bright areas by
`highlights` (`0..=100`), drawn with the `Gaussian` model (a near-Gaussian
per-pixel draw), the `Uniform` model (a uniform per-pixel draw), or the
`Grain` model (interpolated lattice cells sized by `size`, blended toward
per-pixel jitter by `roughness`), monochromatic at `color` 0 and per-channel
at 100, seeded by `seed` so equal inputs and seeds produce bit-identical
output. `light_bokeh`, `bokeh_color`, `amount`, `size`, `roughness`, `color`,
and `highlights` outside `0..=100` SHALL be rejected with
`FilterError::InvalidParams` before any pixel is written. The passes SHALL
write the colour planes only.

#### Scenario: Light Bokeh brightens only the blurred band

- **WHEN** Field Blur with a sharp pin and a blurred pin is applied to a flat
  mid-grey buffer with Light Bokeh 100 over the whole tonal range
- **THEN** the blurred end brightens while a pixel at the sharp pin is
  unchanged, and Light Bokeh 0 with Bokeh Color 0 is a no-op

#### Scenario: Bokeh Color adds chroma to the lightened areas

- **WHEN** Field Blur with Bokeh Color 100 is applied to a flat tinted buffer
- **THEN** the blurred end's channel spread grows while a pixel at the sharp
  pin is unchanged

#### Scenario: Light Range gates the bokeh

- **WHEN** the range is 100..255 and the buffer's tone is 50
- **THEN** the blurred pixels are unchanged, while a tone of 200 in range
  brightens

#### Scenario: Noise is seeded and follows its controls

- **WHEN** the Noise pass with a non-zero amount is applied twice, and again
  with a different seed, colour 100, highlights 100, or another model
- **THEN** equal seeds are bit-identical, a different seed differs, colour 100
  separates the channels where colour 0 keeps them equal, highlights 100
  damps the grain in a bright buffer, Gaussian/Uniform/Grain differ from one
  another, and amount 0 is a no-op

#### Scenario: Invalid effect parameters are rejected without mutation

- **WHEN** the bokeh or noise values fall outside their ranges or are
  non-finite
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is
  bit-exactly unchanged

### Requirement: Iris blur (elliptical focus variable blur)

The system SHALL implement `Filter::IrisBlur { pins: Vec<IrisPin>, bokeh,
noise }` with `IrisPin { x, y, radius_x, radius_y, angle, squareness,
feather, blur }` at the crate root. `x` and `y` SHALL be fractions of the
buffer (outside allowed); `radius_x` and `radius_y` SHALL be the outline's
semi-axes as fractions of the buffer width in
`0.0001..=MAX_IRIS_RADIUS` (10); `angle` SHALL rotate the `radius_x` axis
clockwise in degrees; `squareness` (`0..=100`) SHALL select the
superellipse exponent `p = 2 / (1 − 0.009·squareness)`; `feather`
(`0..=1`) SHALL be the sharp core as a fraction of the outline; and `blur`
SHALL be the exterior radius in buffer pixels in `0..=MAX_FIELD_RADIUS`. At
most `MAX_FIELD_PINS` (16) pins SHALL be accepted. A pin's falloff SHALL be
0 where the normalized superellipse distance `ρ ≤ feather`, 1 where
`ρ ≥ 1`, and a smoothstep between; the per-pixel radius SHALL be the product
of the pins' falloffs times the inverse-square-distance blend of their
amounts. The kernel SHALL run the shared Blur Gallery variable blur and its
Blur Effects / Noise passes weighted by the normalized radius, SHALL leave a
zero-radius pixel unchanged, SHALL blur the alpha plane with the colour (the
effects passes stay colour-only), SHALL treat
an empty pin list as a no-op, and SHALL reject non-finite or out-of-range
values with `FilterError::InvalidParams` before writing any pixel. Equal
inputs SHALL produce bit-identical output. Oracle expectation: no faithful
ImageMagick operator exists; the mapping row is no-equivalent with tolerance
0 and a non-empty note.

#### Scenario: The sharp core is untouched and the exterior blurs

- **WHEN** Iris Blur is applied with one circular pin to a noisy picture
- **THEN** every pixel inside the feather core is bit-identical and pixels
  outside the outline change

#### Scenario: The fade band ramps

- **WHEN** an impulse is blurred inside the core, in the fade band, and
  outside the outline
- **THEN** the core impulse does not spread and the spread grows from the
  fade band to the exterior

#### Scenario: Rotation, squareness, and feather shape the falloff

- **WHEN** a wide pin is rotated 90°, a pin's squareness is raised to 100,
  or its feather is widened
- **THEN** the sharp and blurred axes swap, a diagonal point outside the
  ellipse falls inside the near-square, and more of the radius stays sharp

#### Scenario: Two pins keep two sharp islands

- **WHEN** Iris Blur is applied with two separate pins
- **THEN** both centres stay sharp and the region between them blurs

#### Scenario: Invalid parameters are rejected without mutation

- **WHEN** a pin has a non-finite value, a zero or over-10 radius, a
  squareness above 100, a feather outside 0..1, a blur outside 0..500, or
  more than 16 pins are given
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is
  unchanged

### Requirement: Lens blur (depth-map aperture convolution)

The system SHALL implement `Filter::LensBlur { depth: Option<Arc<DepthMap>>, focal: u8, invert: bool, shape: IrisShape, radius: u32, curvature: u8, rotation: f64, specular_brightness: u8, specular_threshold: u8, noise_amount: u8, noise_distribution: NoiseDistribution, monochromatic: bool, faster: bool, seed: u64 }`. `DepthMap` SHALL be a one-plane 8-bit map (0 near/front, 255 far) and `IrisShape` SHALL be one of `Triangle`, `Square`, `Pentagon`, `Hexagon`, `Heptagon`, `Octagon` (3 through 8 blades). Each pixel's circle-of-confusion radius SHALL be `radius × |d − focal| / 255`, where `d` is the depth sample after `invert`; a `None` map SHALL act as a uniform depth of 255. The kernel SHALL convolve the color planes with an antialiased aperture mask of the chosen iris shape, rounded toward a circle by `curvature` (0 = polygon, 100 = circle) and rotated by `rotation` degrees, so an out-of-focus point light spreads into the aperture shape. Source pixels at or above `specular_threshold` SHALL contribute an additive highlight weighted by `specular_brightness`, so an out-of-focus bright spot blooms rather than dims. The kernel SHALL re-inject noise of `noise_amount` in `noise_distribution` (`Uniform`/`Gaussian`), shared across channels when `monochromatic`, seeded by `seed` so equal inputs and seeds produce bit-identical output. A depth map whose dimensions differ from the buffer SHALL be sampled with a clamped bilinear stretch. `radius`, `curvature`, `specular_brightness`, and `noise_amount` SHALL span `0..=100`; `rotation` SHALL be finite; a value outside those ranges SHALL be rejected with `FilterError::InvalidParams` before any pixel is written. Alpha SHALL be left to the render layer's alpha pass. Oracle expectation: no faithful ImageMagick operator exists (ImageMagick has no depth-aware aperture blur), so property and known-value tests cover the behavior and the mapping row is no-equivalent with tolerance 0 and a non-empty note.

#### Scenario: In-focus depth stays sharp

- **WHEN** Lens Blur is applied with a depth map equal to the focal distance everywhere, a non-zero radius, and no noise
- **THEN** every output sample equals the input within 1 LSB

#### Scenario: A depth ramp blurs with distance

- **WHEN** Lens Blur with focal 0 is applied to a buffer whose depth map is black at the bottom and white at the top
- **THEN** the bottom edge stays sharp and the top edge's transition is wider than the input's

#### Scenario: A point light repeats the iris shape

- **WHEN** an out-of-focus impulse is blurred with a triangle iris and low curvature, and again with the iris rotated
- **THEN** the spread support follows the polygon rather than a disc, and rotating the iris rotates the support

#### Scenario: Specular highlights bloom

- **WHEN** an out-of-focus bright spot at or above `specular_threshold` is blurred with `specular_brightness` at 100 and again at 0
- **THEN** the spot is brighter with the boost than without it

#### Scenario: Noise is seeded and deterministic

- **WHEN** the same Lens Blur with a non-zero noise amount is applied twice, and again with a different seed
- **THEN** the equal seeds are bit-identical, the different seed differs, monochromatic noise leaves the three channels equal, and alpha is bit-identical throughout

#### Scenario: A differently sized depth map is stretched

- **WHEN** a 1×2 black/white depth map is stretched over a taller buffer
- **THEN** the blur increases smoothly along the buffer instead of in two hard steps

#### Scenario: Invalid parameters are rejected without mutation

- **WHEN** Lens Blur is applied with radius 101 or a non-finite rotation
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is bit-exactly unchanged

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Lens Blur has no ImageMagick operator, tolerance 0, and a non-empty no-equivalent note

### Requirement: Path blur (path-following motion blur)

The system SHALL implement `Filter::PathBlur { paths: Vec<BlurPath>, motion:
PathMotion, noise }` with `BlurPath { points, start_speed, end_speed }` and
`PathMotion { speed, taper, centered, rear_sync, strobe_strength,
strobe_flashes }` at the crate root, labelled non-parity (a post-CS6
effect, approximated). A path SHALL have 2..=`MAX_PATH_POINTS` (32) points
as fractions of the buffer (outside allowed), describing the Catmull-Rom
curve through them, and end-point speeds in buffer pixels in
`0..=MAX_END_POINT_SPEED` (500). `speed` SHALL be in `0..=MAX_PATH_SPEED`
(500 %), `taper` and `strobe_strength` in `0..=100`, and `strobe_flashes` in
`1..=100`; at most `MAX_FIELD_PINS` (16) paths SHALL be accepted. Each
pixel's motion vector SHALL be the inverse-square-distance blend of each
path's nearest-point unit tangent times the end speeds interpolated by arc
length, scaled by `speed / 100`; the pixel SHALL average taps along the
field's streamline over that length, centred on the pixel when `centered`
and behind it otherwise, with taper, strobe, and Rear Sync Flash weights.
A trail shorter than one pixel SHALL leave the pixel unchanged. The gather
SHALL run over the alpha plane too; Noise SHALL follow, weighted by the
trail length; the kernel SHALL treat an empty path list as a no-op and
SHALL reject non-finite or out-of-range values with
`FilterError::InvalidParams` before writing any pixel. Equal inputs SHALL
produce bit-identical output. Oracle expectation: no faithful ImageMagick
operator exists; the mapping row is no-equivalent with tolerance 0 and a
non-empty note.

#### Scenario: A horizontal path blurs along rows only

- **WHEN** one horizontal path is applied to a picture whose rows are each
  one colour, and to a picture of vertical stripes
- **THEN** the row picture is bit-identical and the stripes change

#### Scenario: Centered Blur off trails behind the motion

- **WHEN** an impulse is blurred by a rightward path with Centered Blur off
- **THEN** pixels to the impulse's right change and pixels to its left do
  not; with Centered Blur on both sides change

#### Scenario: Strobe, taper, and Rear Sync Flash shape the trail

- **WHEN** an impulse is blurred with Strobe Strength 100 and four flashes,
  with Taper 100, or with Rear Sync Flash
- **THEN** the trail shows separated ghosts, fades toward its far end, or
  keeps the impulse itself brightest, respectively

#### Scenario: A vertical path turns the blur

- **WHEN** a vertical path is applied to the row picture
- **THEN** the picture changes

#### Scenario: Invalid paths are rejected

- **WHEN** a path has one point, a non-finite coordinate, or an end speed
  above the maximum, or the motion values are out of range
- **THEN** the kernel returns `InvalidParams` and the buffer is unchanged

### Requirement: Combined kernel with paths

`Filter::BlurGallery` SHALL gain `path: Vec<BlurPath>` and `motion:
PathMotion`. With paths, the kernel SHALL run the radius-field blur and the
Tilt-Shift smear first (when those effects are on), then the path gather
over their result, then the effects: Bokeh weighted by the radius field
only, Noise by the per-pixel larger of the radius and path weights. With no
paths the output SHALL be bit-identical to the kernel without them, and
paths alone SHALL match `Filter::PathBlur` with the same noise.

#### Scenario: Paths alone match their own kernel

- **WHEN** the combined kernel runs with only paths
- **THEN** the output equals `Filter::PathBlur`'s

#### Scenario: Paths add to a radius effect

- **WHEN** a Field pin and a path are combined
- **THEN** the output differs from each effect alone

### Requirement: Tilt-shift blur (band focus variable blur)

The system SHALL implement `Filter::TiltShift { pins: Vec<TiltPin>, bokeh,
noise }` with `TiltPin { x, y, angle, focus_above, focus_below, fade_above,
fade_below, distortion, symmetric, blur }` at the crate root. `x` and `y`
SHALL be fractions of the buffer (outside allowed); `angle` SHALL turn the
band clockwise in degrees; the four extents SHALL be fractions of the buffer
height in `0..=MAX_TILT_EXTENT` (10), measured across the band, `focus_*`
from the centre line to each solid line and `fade_*` from each solid line to
its dashed line; `distortion` SHALL be in `-100..=100`; and `blur` SHALL be
the full radius in buffer pixels in `0..=MAX_FIELD_RADIUS`. At most
`MAX_FIELD_PINS` (16) pins SHALL be accepted. A pin's falloff SHALL be 0
within its focus extent on that side, 1 beyond focus plus fade, and a
smoothstep between; the per-pixel radius SHALL be the product of the pins'
falloffs times the inverse-square-distance blend of their amounts. A
non-zero distortion SHALL smear the blurred zone below the band (both zones
when `symmetric`) along the radius from the pin centre when positive and
around it when negative, over a length proportional to the local radius,
before the Blur Effects / Noise passes; a pixel whose radius is 0 SHALL stay
unchanged. The blur and smear SHALL run over the alpha plane too, so a
layer's edge feathers beyond the band (the effects passes stay
colour-only); the kernel SHALL treat an
empty pin list as a no-op, and SHALL reject non-finite or out-of-range
values with `FilterError::InvalidParams` before writing any pixel. Equal
inputs SHALL produce bit-identical output. Oracle expectation: no faithful
ImageMagick operator exists; the mapping row is no-equivalent with tolerance
0 and a non-empty note.

#### Scenario: The focus band is untouched and the far rows blur

- **WHEN** Tilt-Shift is applied with one horizontal band to a noisy picture
- **THEN** every pixel within the focus extents is bit-identical and the
  rows beyond the fade change

#### Scenario: The fade ramps and each side keeps its extents

- **WHEN** an impulse is blurred inside the band, in the fade, and beyond
  it, or the band's two sides are given different focus extents
- **THEN** the spread grows from the band through the fade, and each side
  stays sharp exactly as far as its own extent

#### Scenario: Rotation and two bands

- **WHEN** a band is rotated 90°, or two separate bands are applied
- **THEN** the sharp area turns vertical, or both bands stay sharp while the
  gap between them blurs

#### Scenario: Distortion acts below the band unless symmetric

- **WHEN** a positive distortion is applied, then the same with
  `symmetric`, then a negative one
- **THEN** only the zone below changes from the undistorted result, then
  both zones do, and the negative result differs from the positive while
  the band stays untouched

#### Scenario: A layer's edge feathers beyond the band only

- **WHEN** Tilt-Shift is applied to a four-channel buffer with hard alpha
  stripes
- **THEN** the band rows keep their alpha bit-identical and the far rows'
  alpha softens

#### Scenario: Invalid parameters are rejected without mutation

- **WHEN** a pin has a non-finite value, a negative or over-10 extent, a
  distortion outside -100..100, a blur outside 0..500, or more than 16 pins
  are given
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is
  unchanged

### Requirement: Combined Blur Gallery kernel

The system SHALL implement `Filter::BlurGallery { field: Vec<BlurPin>, iris:
Vec<IrisPin>, tilt: Vec<TiltPin>, bokeh, noise }`. Each non-empty list SHALL
build its radius field exactly as its own kernel does, and each pixel SHALL
take the largest of those radii (inferred; Adobe's combine is closed) in one
shared variable-blur pass; Tilt-Shift's distortion smear SHALL follow when
its pins distort, then the Blur Effects / Noise passes. An effect alone SHALL
produce bytes identical to its own kernel. All three lists empty SHALL be a
no-op; each list SHALL be validated as its own kernel validates it, before
any pixel is written. Equal inputs SHALL produce bit-identical output.
Oracle expectation: no-equivalent with tolerance 0 and a non-empty note.

#### Scenario: One effect alone matches its own kernel

- **WHEN** the combined kernel runs with only Field, only Iris, or only
  Tilt-Shift pins, with the effects on
- **THEN** each result is byte-identical to that effect's own kernel

#### Scenario: Each effect blurs the other's sharp area

- **WHEN** an iris and a band are combined
- **THEN** a band-row pixel the iris blurs changes, an iris-core pixel the
  band blurs changes, and a pixel sharp in both is bit-identical

### Requirement: Blur alpha follows the kernel

For 4-channel buffers, every blur filter SHALL run its kernel over channel 4 as it does over the colour planes, so the layer edge spreads with the colour, and a channel 4 that is 255 everywhere SHALL stay 255. Smart Blur SHALL do so in Normal mode only, averaging channel 4 over the same taps its colour threshold admits; its Edge Only and Overlay Edge modes SHALL leave channel 4 bit-identical. Lens Blur and the Blur Gallery's Blur Effects and Noise passes SHALL apply their highlight and noise to channels 1 through 3 only; channel 4 takes the blur alone.

#### Scenario: A blur moves a soft edge

- **WHEN** a blur variant is applied to an RGBA buffer whose alpha is a soft-edged disc
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: An opaque buffer stays opaque

- **WHEN** a blur variant is applied to an RGBA buffer whose alpha is 255 everywhere
- **THEN** every alpha sample is still 255

#### Scenario: Smart Blur edge modes keep alpha

- **WHEN** Smart Blur in Edge Only or Overlay Edge mode is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

#### Scenario: Lens Blur alpha has no bloom or grain

- **WHEN** Lens Blur with specular highlights and noise is applied to an RGBA buffer
- **THEN** the alpha plane equals the same Lens Blur, highlights and noise off, applied to that plane as colour

### Requirement: Spin blur (rotational motion blur)

The system SHALL implement `Filter::SpinBlur { spins: Vec<SpinPin>, strobe:
SpinStrobe, noise }` with `SpinPin { x, y, radius_x, radius_y, angle,
feather, blur_angle, pivot_x, pivot_y }` and `SpinStrobe { strength,
flashes, duration }` at the crate root, labelled non-parity (a post-CS6
effect, approximated). A spin's centre and rotation-centre offset SHALL be
fractions of the buffer (outside allowed), its semi-axes fractions of the
buffer width in `0.0001..=10`, its `feather` in `0..=1`, and its
`blur_angle` in `0..=MAX_SPIN_ANGLE` (360°); `strength` SHALL be in
`0..=100`, `flashes` in `1..=100`, and `duration` in `0..=360`; at most
`MAX_FIELD_PINS` (16) spins SHALL be accepted. Each pixel's mask SHALL be 1
inside the feather ring, 0 outside the ellipse, and a smoothstep between,
taking the spin with the largest mask; the pixel SHALL average taps along
the circular arc around that spin's rotation centre, centred on the pixel,
sweeping `blur_angle` inside the feather ring and, past it, keeping the
ring's arc length times the mask, so the blur only weakens toward the
outline; the strobe flashes SHALL mix in by strength.
An arc shorter than one pixel SHALL leave the pixel unchanged. The gather
SHALL run over the alpha plane too; Noise SHALL follow, weighted by the
mask; the kernel SHALL treat an empty spin list as a no-op and SHALL reject
non-finite or out-of-range values with `FilterError::InvalidParams` before
writing any pixel. Equal inputs SHALL produce bit-identical output. Oracle
expectation: no faithful ImageMagick operator exists; the mapping row is
no-equivalent with tolerance 0 and a non-empty note.

#### Scenario: A spin blurs inside its ellipse only

- **WHEN** a spin is applied to a striped picture
- **THEN** pixels inside the feather ring change and pixels outside the
  ellipse are bit-identical

#### Scenario: The smear shortens past the feather ring

- **WHEN** a spin's feather ring is pulled in toward its centre
- **THEN** along a ray from the centre the arc length peaks at the ring and
  falls toward the outline

#### Scenario: The blur follows circles round the rotation centre

- **WHEN** a spin centred on a picture of concentric rings is applied
- **THEN** the picture is unchanged up to rounding, while a picture of
  radial spokes changes

#### Scenario: A moved rotation centre moves the blur

- **WHEN** the same spin is applied with its rotation centre offset
- **THEN** the output differs from the centred spin's

#### Scenario: Strobe flashes separate the trail

- **WHEN** an impulse is spun with Strobe Strength 100 and four flashes
- **THEN** its arc shows separated bright taps instead of an even smear

#### Scenario: Invalid spins are rejected

- **WHEN** a spin has a non-finite value, a Blur Angle above 360, a zero
  radius, or the strobe values are out of range
- **THEN** the kernel returns `InvalidParams` and the buffer is unchanged

### Requirement: Combined kernel with spins

`Filter::BlurGallery` SHALL gain `spin: Vec<SpinPin>` and `spin_strobe:
SpinStrobe`. With spins, the kernel SHALL run the radius-field blur, the
Tilt-Shift smear, and the path gather first (when those effects are on),
then the spin gather over their result, then the effects: Bokeh weighted by
the radius field only, Noise by the per-pixel largest of the radius, path,
and spin weights. With no spins the output SHALL be bit-identical to the
kernel without them, and spins alone SHALL match `Filter::SpinBlur` with
the same strobe and noise.

#### Scenario: Spins alone match their own kernel

- **WHEN** the combined kernel runs with only spins
- **THEN** the output equals `Filter::SpinBlur`'s

#### Scenario: Spins add to a radius effect

- **WHEN** a Field pin and a spin are combined
- **THEN** the output differs from each effect alone
