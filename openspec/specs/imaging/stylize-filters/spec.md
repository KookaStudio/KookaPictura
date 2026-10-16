# stylize-filters Specification

## Purpose
Emboss, Find Edges, and Solarize with alpha preservation, edge handling, determinism, and an oracle classification.

## Requirements

### Requirement: Stylize filter application and error contract

The system SHALL provide `pictura_filters::apply(filter: &Filter, buf: &mut PixelBuffer) -> Result<(), FilterError>`. It SHALL operate in place on a planar 8-bit buffer whose `channels` is 3 (RGB) or 4 (RGBA), treating channels 1 through 3 as the color planes and channel 4, when present, as alpha. It SHALL transform every color sample or return an error; it MUST NOT partially apply a filter and then fail. Malformed buffers MUST return `FilterError` instead of panicking.

#### Scenario: Apply a Stylize filter to a 3-channel planar buffer

- **WHEN** `apply` receives a Stylize `Filter` variant and a 3-channel planar buffer
- **THEN** the R, G, and B planes are rewritten in place and `Ok(())` is returned

#### Scenario: Reject an unsupported channel count

- **WHEN** the buffer has a channel count other than 3 or 4
- **THEN** `apply` returns `FilterError::Unsupported` and leaves the buffer unchanged

#### Scenario: Malformed buffers error instead of panicking

- **WHEN** `apply` receives an empty buffer, or a buffer whose `data.len()` does not equal `width * height * channels`, or a buffer with zero width or height
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Alpha channel preservation for Stylize filters

For 4-channel buffers, every Stylize filter SHALL leave channel 4 bit-identical. Only channels 1 through 3 SHALL be modified.

#### Scenario: Every Stylize variant preserves alpha

- **WHEN** each Stylize variant is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Emboss

The system SHALL implement `Filter::Emboss { angle: f64, height: f64, amount: f64 }`. It SHALL difference each colour channel across `height` pixels along `angle`, the direction the light comes from, add a 128 mid-gray bias, and scale the deviation from mid-gray by `amount` percent, so flat areas become mid-gray and edges carry a light and a dark side traced in the original colour, as CS6 Help describes ("converting its fill color to gray and tracing the edges with the original fill color"). A gray image SHALL stay gray. `angle` SHALL be finite and within `-360.0..=360.0`; `height` and `amount` SHALL be finite and strictly greater than 0. A value outside those ranges SHALL be rejected with `FilterError::InvalidParams`. Oracle expectation: classified as no-equivalent (ImageMagick `-emboss` has no angle, height, or amount), covered by property tests.

#### Scenario: The output is gray

- **WHEN** Emboss is applied to a gray image (R, G, and B planes equal)
- **THEN** every output pixel has R, G, and B equal to one another; a colour image keeps its edge colours (see "Coloured edges keep their colour")

#### Scenario: Coloured edges keep their colour

- **WHEN** Emboss is applied to a colour image with coloured edges
- **THEN** some output pixel has R different from B

#### Scenario: Negating the angle swaps the highlight and shadow

- **WHEN** Emboss is applied to the same edge image with the light at angle A and at the opposite side, A + 180
- **THEN** the highlight side of the edge under one becomes the shadow side under the other

#### Scenario: A uniform field becomes mid-gray

- **WHEN** Emboss is applied to a uniform-color buffer
- **THEN** every output color sample equals 128 within 1 LSB

#### Scenario: Invalid parameters are rejected

- **WHEN** Emboss is applied with an angle outside `-360.0..=360.0`, or with a non-finite angle, height, or amount, or with `height <= 0` or `amount <= 0`
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

### Requirement: Find Edges

The system SHALL implement `Filter::FindEdges` with no parameters. It SHALL compute a Sobel gradient magnitude per color channel and render edges dark on a light field as `out = 255 − clamp(magnitude)`. A constant-color image SHALL map to a uniform 255. Oracle expectation: ImageMagick `-edge` uses a different edge detector and the opposite polarity (bright edges on black), so Find Edges is classified as no-equivalent with tolerance 0 and covered by property and known-value tests, with the polarity inversion documented.

#### Scenario: A constant image is uniformly white

- **WHEN** Find Edges is applied to a constant-color buffer
- **THEN** every output color sample equals 255

#### Scenario: An edge renders darker than its surroundings

- **WHEN** Find Edges is applied to an image containing a step edge
- **THEN** at least one pixel on the edge has a lower value than the flat regions on either side

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Find Edges has no ImageMagick operator, tolerance 0, and a non-empty no-equivalent note that records the polarity inversion

### Requirement: Solarize

The system SHALL implement `Filter::Solarize` with no parameters. For each color channel it SHALL apply the fixed 50% curve `out = if v >= 128 { 255 − v } else { v }`, so values at or above 128 are inverted and values below 128 are unchanged. Solarize SHALL NOT be an involution: applying it twice differs from applying it once. Oracle expectation: differential against ImageMagick `-solarize 50%` within tolerance 0 at 8-bit.

#### Scenario: Values below the threshold are unchanged

- **WHEN** Solarize is applied to a buffer whose color samples are all below 128
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Values at or above the threshold are inverted

- **WHEN** Solarize is applied to a buffer with a sample at 200
- **THEN** that sample becomes 55

#### Scenario: Applying twice is not the same as applying once

- **WHEN** Solarize is applied twice to a non-uniform buffer
- **THEN** the result differs from a single application

#### Scenario: Matches ImageMagick

- **WHEN** Solarize is applied to the oracle image and ImageMagick applies `-solarize 50%`
- **THEN** no sample differs by more than 0

### Requirement: Clamp-to-edge borders and tiny images for Stylize filters

Every neighborhood Stylize filter (Emboss and Find Edges) SHALL sample with clamp-to-edge at the image borders, so an out-of-range index maps to the nearest edge sample. Solarize is pointwise. A 1×1 image and a 1-pixel-wide or 1-pixel-tall image MUST NOT panic and MUST return either a result or a typed error.

#### Scenario: A 1x1 image does not panic

- **WHEN** every Stylize variant is applied to a 1×1 buffer
- **THEN** no variant panics and each returns `Ok(())` or a `FilterError`

#### Scenario: Border pixels clamp to the edge

- **WHEN** Emboss or Find Edges is applied to a small buffer and a border pixel is inspected
- **THEN** the result is consistent with repeating the edge sample rather than reading out of bounds

### Requirement: Deterministic Stylize output

Stylize filters SHALL be deterministic. The same filter applied to equal input buffers SHALL produce bit-identical output every run, with no random, time, or thread-order dependence.

#### Scenario: Repeated runs match

- **WHEN** each Stylize variant is applied to two clones of one buffer
- **THEN** the two output buffers are bit-identical

### Requirement: Stylize ImageMagick oracle and no-equivalent classification

The system SHALL ship `scripts/filter_oracle.py`, which applies an ImageMagick operator to a raw 8-bit image, and `crates/pictura-filters/tests/oracle.rs`, which diffs `apply` against it. The mapping table SHALL have exactly one row per Stylize `Filter` variant. Solarize SHALL be diffed (`-solarize 50%`) within tolerance 0. Emboss SHALL be diffed against `-emboss` if that kernel is verified faithful, otherwise classified as no-equivalent. Find Edges SHALL be classified as no-equivalent with tolerance 0, a property or known-value test, and a non-empty note recording the polarity inversion. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table covers every Stylize variant

- **WHEN** the oracle tests run
- **THEN** every Stylize variant has a table row, each no-equivalent row has tolerance 0, and each row carries a non-empty note

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes

### Requirement: Extrude

The system SHALL implement `Filter::Extrude { kind: ExtrudeType, size: u32, depth: f32, level_based: bool, solid_front: bool, mask_incomplete: bool }`
with `ExtrudeType::{Blocks, Pyramids}`. It SHALL divide the image into cells of
`size` pixels and render each as a 3D block (a front face plus four shaded
sides) or a pyramid (four triangular sides to an apex), protruding by `depth`;
towers SHALL be drawn back-to-front. When `level_based` is true the protrusion
SHALL follow the cell's average luma, otherwise it SHALL come from a coordinate
hash. When `solid_front` is true (Blocks) the front face SHALL be filled with
the cell average, otherwise it SHALL sample the source picture. When
`mask_incomplete` is true partial cells on the right and bottom edge SHALL be
hidden. `size` SHALL span `2..=255` and `depth` SHALL be finite and within
`1.0..=255.0`; a value outside those ranges SHALL be rejected with
`FilterError::InvalidParams` before any mutation. Alpha SHALL be left
bit-identical, and the output SHALL be deterministic (the coordinate hash
carries no seed). Oracle expectation: no faithful ImageMagick equivalent exists
(the extrusion renderer is closed), so property tests cover the level ordering
and the front faces, and the divergence is documented.

#### Scenario: Level-based depth follows source brightness

- **WHEN** Extrude with `level_based: true` and `solid_front: true` at full depth renders a dark cell next to a bright cell
- **THEN** the bright cell's front is painted over the dark neighbour

#### Scenario: Solid front faces carry the cell average

- **WHEN** Extrude with `solid_front: true` renders a whole cell
- **THEN** every pixel of that cell's front face carries the same average color

#### Scenario: Output is deterministic

- **WHEN** Extrude with the same parameters is applied to two clones of one buffer
- **THEN** the two outputs are bit-identical

#### Scenario: Out-of-range size or depth is rejected untouched

- **WHEN** Extrude is applied with `size` below 2 or above 255, or a `depth` that is non-finite, below 1.0, or above 255.0
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

#### Scenario: Alpha is preserved

- **WHEN** Extrude is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Tiles

The system SHALL implement `Filter::Tiles { count: u32, offset: u32, fill: TileFill, foreground: [u8; 3], background: [u8; 3] }`
with
`TileFill::{BackgroundColor, ForegroundColor, InverseImage, UnalteredImage}`.
It SHALL break the image into `count` square tiles counted across the shorter
side, displace each tile by a coordinate-hash amount up to `offset` percent of
the tile size, and fill the exposed gaps per `fill`: the `background` color, the
`foreground` color, the inverse of the source pixels, or the source pixels
unchanged. `count` SHALL span `1..=99` and `offset` SHALL span `1..=99`
percent; a value outside those ranges SHALL be rejected with
`FilterError::InvalidParams` before any mutation. Alpha SHALL be left
bit-identical, and the output SHALL be deterministic (the coordinate hash
carries no seed). Oracle expectation: no faithful ImageMagick equivalent exists
(the tiling and fill semantics are closed), so property tests cover the fill
rules and determinism, and the divergence is documented.

#### Scenario: The background fill is flat

- **WHEN** Tiles with `fill: BackgroundColor` is applied
- **THEN** every pixel left uncovered by a displaced tile equals the `background` color

#### Scenario: Inverse Image negates the ground

- **WHEN** Tiles with `fill: InverseImage` is applied to a flat buffer
- **THEN** every uncovered pixel equals the source value subtracted from 255

#### Scenario: Unaltered Image leaves a flat picture alone

- **WHEN** Tiles with `fill: UnalteredImage` is applied to a flat buffer
- **THEN** the buffer is bit-identical to the input

#### Scenario: Output is deterministic

- **WHEN** Tiles with the same parameters is applied to two clones of one buffer
- **THEN** the two outputs are bit-identical

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** Tiles is applied with `count` 0 or 100, or `offset` 0 or 100
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

#### Scenario: Alpha is preserved

- **WHEN** Tiles is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Trace Contour

The system SHALL implement `Filter::TraceContour { level: u8, edge: ContourEdge }`
with `ContourEdge::{Lower, Upper}`. For each color channel it SHALL mark a
sample as a crossing when `edge` is `Upper`, the sample is at or above `level`,
and at least one of its four-connected neighbours is below `level`, or when
`edge` is `Lower`, the sample is below `level`, and at least one neighbour is at
or above `level`. A marked sample SHALL become 0 in that channel and every other
sample SHALL become 255 in that channel. `level` is a `u8`, so no out-of-range
value can be constructed. A flat field SHALL map to 255 everywhere, alpha SHALL
be left bit-identical, and the output SHALL be deterministic. Oracle
expectation: no faithful ImageMagick equivalent exists (the contour test is
closed), so property tests cover the flat no-op and the two edge directions, and
the divergence is documented.

#### Scenario: A flat image has no contour

- **WHEN** Trace Contour is applied to a constant-color buffer
- **THEN** every color sample equals 255 in every channel

#### Scenario: Lower and Upper ink opposite sides of a step

- **WHEN** Trace Contour with `edge: Upper` and with `edge: Lower` is applied to the same dark-to-bright step
- **THEN** `Upper` marks the bright side and `Lower` marks the dark side

#### Scenario: Shifting the level moves the contour

- **WHEN** Trace Contour is applied at level 0 and at level 255 to the same ramp
- **THEN** level 0 crosses nothing and level 255 marks the peak

#### Scenario: Output is deterministic and alpha is preserved

- **WHEN** Trace Contour is applied twice to equal RGBA buffers
- **THEN** the two outputs are bit-identical and the alpha plane equals the input alpha plane bit for bit

### Requirement: Wind

The system SHALL implement `Filter::Wind { method: WindMethod, from_right: bool }`
with `WindMethod::{Wind, Blast, Stagger}`. It SHALL draw horizontal streaks
from the upwind side toward the downwind side, only where the upwind pixel's
luma exceeds the current pixel's by a fixed gate. `method` SHALL select the
streak length, the fraction of edges that receive a streak, and the strength
retained at the far end (`Wind` 12/55/0.0, `Blast` 32/85/0.55, `Stagger`
16/70/0.25), and `Stagger` SHALL additionally drift each streak vertically.
`from_right` true SHALL blow toward the left and false toward the right. The
filter SHALL be deterministic (the candidate hash carries no seed), SHALL leave
alpha bit-identical, and MUST NOT wrap or overflow at the final clamp. Oracle
expectation: no faithful ImageMagick equivalent exists (the streak geometry is
closed), so property tests cover the flat-ground no-op, the direction mirror,
and determinism, and the divergence is documented.

#### Scenario: Flat ground is untouched

- **WHEN** Wind is applied with any method to a flat buffer
- **THEN** the buffer is bit-identical to the input

#### Scenario: Direction mirrors the displacement

- **WHEN** Wind with `from_right: true` and with `from_right: false` is applied to the same buffer
- **THEN** the two outputs differ

#### Scenario: Output is deterministic and alpha is preserved

- **WHEN** Wind is applied twice to equal RGBA buffers
- **THEN** the two outputs are bit-identical and the alpha plane equals the input alpha plane bit for bit

### Requirement: Diffuse filter

The system SHALL implement `Filter::Diffuse { mode }` where `mode` is `Normal`, `DarkenOnly`, `LightenOnly`, or `Anisotropic`. For a coordinate-hashed choice it SHALL replace each pixel with one of its eight neighbours (Normal), or with the neighbour only when it is darker (Darken Only) or lighter (Lighten Only); Anisotropic SHALL blend each pixel along its edge over four passes. Alpha SHALL be preserved, output SHALL be deterministic (a fixed coordinate hash, with no per-apply re-roll), and `apply` SHALL report a `FilterError` rather than panic on a malformed buffer.

#### Scenario: The modes change colour and preserve alpha

- **WHEN** each Diffuse mode is applied to a non-uniform RGBA buffer
- **THEN** the colour planes change and the alpha plane is bit-identical

#### Scenario: Diffuse is deterministic

- **WHEN** Diffuse is applied twice to clones of one buffer
- **THEN** the two outputs are bit-identical

### Requirement: Glowing Edges filter

The system SHALL implement `Filter::GlowingEdges { width, brightness, smoothness }` with `width` spanning `1..=14`, `brightness` `0..=20`, and `smoothness` `1..=15`. It SHALL blur a copy of the colour planes by the smoothness, take a Sobel gradient magnitude per channel, dilate it by half the width, and scale it into the colour planes by `brightness / 5`, clamping to `0..=255`. Alpha SHALL be preserved, output SHALL be deterministic, and an out-of-range parameter SHALL be rejected as `FilterError::InvalidParams` without mutating the buffer.

#### Scenario: Glowing Edges lights the edges and preserves alpha

- **WHEN** Glowing Edges is applied to a non-uniform RGBA buffer
- **THEN** the colour planes change on an otherwise black ground and the alpha plane is bit-identical

#### Scenario: Out-of-range parameters are rejected

- **WHEN** width, brightness, or smoothness is outside its documented range
- **THEN** the operator returns `FilterError::InvalidParams` and does not mutate the buffer
