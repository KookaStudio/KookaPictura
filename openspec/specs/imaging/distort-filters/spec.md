# distort-filters Specification

## Purpose
The Distort filter family using bilinear inverse mapping, with undefined-area, tiny-image, and determinism rules.

## Requirements

### Requirement: Distort filter application and error contract

The system SHALL provide `pictura_filters::apply(filter: &Filter, buf: &mut PixelBuffer) -> Result<(), FilterError>`. It SHALL operate in place on a planar 8-bit buffer whose `channels` is 3 (RGB) or 4 (RGBA), treating channels 1 through 3 as the color planes and channel 4, when present, as alpha. It SHALL transform every color sample or return an error; it MUST NOT partially apply a filter and then fail. Malformed buffers MUST return `FilterError` instead of panicking.

#### Scenario: Apply a Distort filter to a 3-channel planar buffer

- **WHEN** `apply` receives one of the nine Distort `Filter` variants and a 3-channel planar buffer
- **THEN** the R, G, and B planes are rewritten in place and `Ok(())` is returned

#### Scenario: Reject an unsupported channel count

- **WHEN** the buffer has a channel count other than 3 or 4
- **THEN** `apply` returns `FilterError::Unsupported` and leaves the buffer unchanged

#### Scenario: Malformed buffers error instead of panicking

- **WHEN** `apply` receives an empty buffer, or a buffer whose `data.len()` does not equal `width * height * channels`, or a buffer with zero width or height
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Parameter validation for Distort filters

The system SHALL validate each Distort filter's parameters before writing any sample, and a rejected parameter MUST leave the buffer unchanged. `Twirl` `angle` SHALL be within `-999.0..=999.0`; `Pinch` and `Spherize` `amount` SHALL be within `-100.0..=100.0`; `Ripple` `amount` SHALL be within `-999.0..=999.0`; `Wave` `generators` SHALL be within `1..=999`, each of `wavelength.0` and `amplitude.0` SHALL be within `1.0..=998.0`, `wavelength.1` SHALL be at least `wavelength.0 + 1.0`, `amplitude.1` SHALL be at least `amplitude.0 + 1.0`, and each of `scale.0` and `scale.1` SHALL be within `1.0..=100.0`. `Shear` `curve` SHALL contain at least two finite points whose `x` values are strictly increasing, with each `x` and each `y` within `-1.0..=1.0`. `ZigZag` `amount` SHALL be within `-100.0..=100.0` and `ridges` SHALL be within `0..=20`. `OceanRipple` `size` SHALL be within `1..=15` and `magnitude` SHALL be within `0..=20`. Any value outside its range, and any non-finite `f64` parameter, SHALL be rejected with `FilterError::InvalidParams` and MUST NOT panic. The `SpherizeMode`, `RippleSize`, `WaveType`, `PolarKind`, `ShearFill`, and `ZigZagStyle` enums are closed sets and require no range check.

#### Scenario: Out-of-range scalar amounts are rejected

- **WHEN** Twirl is applied with `angle` `-1000.0` or `1000.0`, Pinch is applied with `amount` `-101.0` or `101.0`, Spherize is applied with `amount` `-101.0` or `101.0`, Ripple is applied with `amount` `-1000.0` or `1000.0`, or ZigZag is applied with `amount` `-101.0` or `101.0`
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: Non-finite parameters are rejected

- **WHEN** any Distort variant is applied with a `NaN` or infinite `f64` parameter
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

#### Scenario: Out-of-range Wave parameters are rejected

- **WHEN** Wave is applied with `generators` 0 or 1000, `wavelength` min 0 or 999, `wavelength` max not greater than its min, or `scale` 0 or 101
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: Out-of-range Shear, ZigZag, and Ocean Ripple parameters are rejected

- **WHEN** Shear is applied with fewer than two curve points, a non-finite curve coordinate, or `x` values that are not strictly increasing, ZigZag is applied with `ridges` `21`, or Ocean Ripple is applied with `size` `0` or `16`, or `magnitude` `21`
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: Boundary values are accepted

- **WHEN** Twirl `angle` `-999.0` or `999.0`, Pinch / Spherize `amount` `-100.0` or `100.0`, Ripple `amount` `-999.0` or `999.0`, Wave `generators` `1` or `999` with the minimum `wavelength` / `amplitude` pair and `scale` exactly `1.0`, ZigZag `amount` `-100.0` or `100.0` with `ridges` `0` or `20`, Ocean Ripple `size` `1` or `15` with `magnitude` `0` or `20`, and Shear with the minimum two-point strictly increasing curve are applied
- **THEN** each returns `Ok(())` with no division-by-zero or empty-region panic

### Requirement: Bilinear inverse-mapping resampling

Every Distort filter SHALL be an inverse-mapping warp: for each destination pixel it SHALL compute a source coordinate and sample the input there. A non-integral source coordinate SHALL be interpolated bilinearly from its four nearest samples; an integral coordinate SHALL return that sample exactly. Twirl, Pinch, Spherize, Ripple, Polar Coordinates, ZigZag, and Ocean Ripple SHALL sample with clamp-to-edge at the image borders; Wave SHALL use the edge policy fixed by `repeat_edge`; Shear SHALL use the edge policy fixed by `fill`. The interpolation MUST read no sample outside the image.

#### Scenario: A non-integral source coordinate interpolates four samples

- **WHEN** a destination pixel maps to a fractional source coordinate and the four surrounding samples are known
- **THEN** the output equals their bilinear blend within 1 LSB

#### Scenario: An identity warp is a no-op

- **WHEN** a Distort filter's displacement is the identity (Twirl angle 0, Pinch amount 0, Spherize amount 0, Ripple amount 0, ZigZag amount 0, or a flat Shear curve)
- **THEN** the output equals the input within 1 LSB

### Requirement: Undefined-area edge handling

For a source coordinate outside the image, Twirl, Pinch, Spherize, Ripple, Polar Coordinates, ZigZag, and Ocean Ripple SHALL clamp to the nearest edge sample. Wave SHALL honor its `repeat_edge` flag: when `repeat_edge` is `true` it SHALL repeat the nearest edge sample, and when `repeat_edge` is `false` it SHALL wrap around and sample from the opposite edge. Shear SHALL honor its `fill`: `ShearFill::RepeatEdgePixels` SHALL repeat the nearest edge sample, and `ShearFill::WrapAround` SHALL wrap around and sample from the opposite edge. The `Wave` and `Shear` modes MUST be observably different where the displacement reaches beyond the image bounds.

#### Scenario: Wave repeat-edge and wrap differ off-canvas

- **WHEN** Wave with the same seed and parameters but `repeat_edge` `true` and `false` is applied to a buffer whose displacement reaches beyond an edge
- **THEN** the two outputs differ, the repeat-edge output equals a clamp-to-edge sample, and the wrap output equals a sample from the opposite edge

#### Scenario: Shear wrap and repeat-edge differ off-canvas

- **WHEN** Shear with the same curve but `ShearFill::WrapAround` and `ShearFill::RepeatEdgePixels` is applied to a buffer whose displacement moves rows beyond an edge
- **THEN** the two outputs differ, the repeat-edge output equals a clamp-to-edge sample, and the wrap output equals a sample from the opposite edge

#### Scenario: Radial filters clamp at the border

- **WHEN** Twirl, Pinch, Spherize, Ripple, Polar Coordinates, ZigZag, or Ocean Ripple maps a destination pixel to a source coordinate outside the image
- **THEN** the sample is taken from the nearest edge and the call does not panic

### Requirement: Tiny-image safety for Distort filters

A 1×1 image MUST NOT panic and SHALL be returned bit-identical for every Distort filter. A 1-pixel-wide or 1-pixel-tall image MUST NOT panic and SHALL return either `Ok(())` or a typed `FilterError`, with no division by zero and no out-of-bounds read in the center or radius computation.

#### Scenario: A 1x1 image is a no-op

- **WHEN** every Distort variant is applied to a 1×1 buffer
- **THEN** no variant panics, each returns `Ok(())`, and the buffer is bit-identical to the input

#### Scenario: A 1-pixel-wide or 1-pixel-tall image does not panic

- **WHEN** every Distort variant is applied to a 1×N and an N×1 buffer
- **THEN** no variant panics and each returns `Ok(())` or a `FilterError`

### Requirement: Deterministic Distort output

Twirl, Pinch, Spherize, Ripple, Polar Coordinates, Shear, and ZigZag SHALL produce bit-identical output for equal input and parameters, with no time or thread-order dependence. Wave and Ocean Ripple SHALL be seeded: applying the same `seed` and parameters to equal input buffers SHALL produce bit-identical output every run, and a different `seed` SHALL be permitted to differ.

#### Scenario: The unseeded variants are repeatable

- **WHEN** Twirl, Pinch, Spherize, Ripple, Polar Coordinates, Shear, or ZigZag is applied twice to two clones of one buffer
- **THEN** the two output buffers are bit-identical

#### Scenario: Wave and Ocean Ripple are reproducible from their seed

- **WHEN** Wave or Ocean Ripple is applied twice with the same seed and parameters to two clones of one buffer
- **THEN** the two output buffers are bit-identical

### Requirement: Twirl

The system SHALL implement `Filter::Twirl { angle: f64 }`. It SHALL rotate the source coordinate about the image center by an angle that decreases with distance from the center, so the center rotates most and the edge least, and the sign of `angle` SHALL set the direction of rotation. `angle` SHALL be within `-999.0..=999.0`, and `angle` `0.0` SHALL be a no-op. Oracle expectation: no faithful ImageMagick operator (ImageMagick `-swirl` uses a different angular falloff), so Twirl is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: The center rotates more than the edge

- **WHEN** Twirl is applied to a buffer with a marker offset from the center, at the center radius and at a larger radius
- **THEN** the sample near the center is displaced by a larger angle than the sample near the edge

#### Scenario: Opposite angles rotate in opposite directions

- **WHEN** Twirl is applied with `angle` `+A` and with `angle` `-A` to the same buffer
- **THEN** the two outputs are related by a reflection of the rotation direction

#### Scenario: Zero angle is a no-op

- **WHEN** Twirl is applied with `angle` `0.0`
- **THEN** the output equals the input within 1 LSB

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Twirl has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Pinch

The system SHALL implement `Filter::Pinch { amount: f64 }`. It SHALL remap the source coordinate along the normalized radius through a monotone curve so that a positive `amount` moves pixels toward the selection center and a negative `amount` moves them away, with larger magnitudes increasing the effect. `amount` SHALL be within `-100.0..=100.0`, and `amount` `0.0` SHALL be a no-op. Oracle expectation: no faithful ImageMagick operator (ImageMagick `-implode` / `-explode` use a global nonlinear model, not Photoshop's radial remap), so Pinch is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: Positive amount contracts toward the center

- **WHEN** Pinch is applied with a positive `amount` to a radial test pattern
- **THEN** features at a given radius move toward the center and the center is the fixed point

#### Scenario: Negative amount expands away from the center

- **WHEN** Pinch is applied with a negative `amount` to a radial test pattern
- **THEN** features at a given radius move away from the center

#### Scenario: Zero amount is a no-op

- **WHEN** Pinch is applied with `amount` `0.0`
- **THEN** the output equals the input within 1 LSB

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Pinch has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Spherize

The system SHALL implement `Filter::Spherize { amount: f64, mode: SpherizeMode }`. It SHALL apply a radial inverse-mapping warp that wraps the selection around a sphere, and `SpherizeMode` SHALL restrict the axes: `Normal` displaces both, `HorizontalOnly` displaces only the horizontal axis, and `VerticalOnly` displaces only the vertical axis. `amount` SHALL be within `-100.0..=100.0`, and `amount` `0.0` SHALL be a no-op. Oracle expectation: no faithful ImageMagick operator, so Spherize is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: Horizontal Only displaces one axis

- **WHEN** Spherize with `SpherizeMode::HorizontalOnly` is applied to a test pattern
- **THEN** samples move along the horizontal axis only, and a purely vertical displacement is not produced

#### Scenario: Vertical Only displaces the other axis

- **WHEN** Spherize with `SpherizeMode::VerticalOnly` is applied to the same test pattern
- **THEN** samples move along the vertical axis only

#### Scenario: Zero amount is a no-op

- **WHEN** Spherize is applied with `amount` `0.0` in any mode
- **THEN** the output equals the input within 1 LSB

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Spherize has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Ripple

The system SHALL implement `Filter::Ripple { amount: f64, size: RippleSize }`. It SHALL add a periodic sinusoidal displacement whose magnitude is proportional to `amount` and whose spatial frequency is fixed by `RippleSize` (`Small`, `Medium`, `Large`). `amount` SHALL be within `-999.0..=999.0`, and `amount` `0.0` SHALL be a no-op. Oracle expectation: no faithful ImageMagick operator (ImageMagick `-wave` uses a different periodic kernel), so Ripple is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: Zero amount is a no-op

- **WHEN** Ripple is applied with `amount` `0.0` in any `RippleSize`
- **THEN** the output equals the input within 1 LSB

#### Scenario: Larger amount increases displacement

- **WHEN** Ripple is applied at a small `amount` and at a larger `amount` to the same buffer
- **THEN** the larger `amount` produces a larger maximum displacement

#### Scenario: Size changes spatial frequency

- **WHEN** Ripple is applied at `RippleSize::Small` and at `RippleSize::Large` with the same `amount`
- **THEN** the two outputs have different spatial frequencies

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Ripple has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Wave

The system SHALL implement `Filter::Wave { generators: u32, wavelength: (f64, f64), amplitude: (f64, f64), kind: WaveType, scale: (f64, f64), seed: u64, repeat_edge: bool }`. It SHALL sum `generators` wave generators, each with a wavelength drawn from `wavelength` and an amplitude drawn from `amplitude` using the seeded RNG, a shape selected by `WaveType` (`Sine`, `Triangle`, `Square`), and axis-wise scaling from `scale`, to form the displacement field. `generators` SHALL be within `1..=999`; each range min SHALL be within `1.0..=998.0` with its max at least `min + 1.0`; each `scale` SHALL be within `1.0..=100.0`. The same `seed` SHALL reproduce the field; `repeat_edge` SHALL select the undefined-area policy. Oracle expectation: no faithful ImageMagick operator (ImageMagick `-wave` has a different generator model), so Wave is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: The same seed is reproducible

- **WHEN** Wave is applied twice with the same parameters and seed to two clones of one buffer
- **THEN** the two output buffers are bit-identical

#### Scenario: A different seed changes the field

- **WHEN** Wave is applied with two different seeds and all other parameters equal
- **THEN** the two outputs are permitted to differ

#### Scenario: The generator count changes the field

- **WHEN** Wave is applied with `generators` `1` and with a larger `generators` count, same seed
- **THEN** the two outputs differ

#### Scenario: Each wave type produces a distinct field

- **WHEN** Wave is applied with `WaveType::Sine`, `WaveType::Triangle`, and `WaveType::Square`, same seed
- **THEN** the three outputs differ

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Wave has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Distort oracle classification

The system SHALL extend `scripts/filter_oracle.py` and `crates/pictura-filters/tests/oracle.rs` so the mapping table has exactly one row per Distort `Filter` variant. Because Adobe's warp kernels and falloff curves are closed, Twirl, Pinch, Spherize, Ripple, and Wave SHALL each be classified as no-equivalent with tolerance 0, a property or known-value test, a non-empty note, and the observed delta against the closest ImageMagick operator (`-swirl`, `-implode` / `-explode`, or `-wave`) recorded. ZigZag and Ocean Ripple SHALL likewise be classified as no-equivalent with tolerance 0 and a property or known-value test, recording the observed delta against their closest operators (`-swirl` and `-wave`). Polar Coordinates and Shear SHALL likewise be classified as no-equivalent with tolerance 0 and a property or known-value test, recording the measured maximum and mean per-sample delta against their closest operators (`-distort Polar` / `-distort DePolar`, and `-shear`): Polar Coordinates max 189 / mean 58 for `-distort Polar` (RectangularToPolar) and max 194 / mean 59 for `-distort DePolar` (PolarToRectangular); Shear max 255 / mean 18–23 for `-shear`. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table covers every Distort variant

- **WHEN** the oracle tests run
- **THEN** every Distort variant has a table row with a property or known-value test, tolerance 0, a non-empty no-equivalent note and a recorded observed delta, and Polar Coordinates and Shear each name `-distort DePolar` / `-distort Polar` or `-shear` with the measured maximum and mean per-sample delta recorded

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes

### Requirement: Polar Coordinates

The system SHALL implement `Filter::PolarCoordinates { kind: PolarKind }`. It SHALL apply a coordinate transform with bilinear resampling: `PolarKind::RectangularToPolar` SHALL reinterpret each destination pixel's rectangular coordinate as a polar coordinate, and `PolarKind::PolarToRectangular` SHALL reinterpret each destination pixel's polar coordinate as a rectangular coordinate, so the two directions form an invertible pair up to resampling error. `PolarKind` is a closed set that requires no range check. Oracle expectation: the closest ImageMagick operators are `-distort DePolar` (rectangular to polar) and `-distort Polar` (polar to rectangular), but neither is faithful, so Polar Coordinates SHALL be classified as no-equivalent with tolerance 0 and a property or known-value test, recording the measured maximum and mean per-sample delta against them (max 189 / mean 58 for `-distort Polar`, max 194 / mean 59 for `-distort DePolar`).

#### Scenario: Rectangular to polar changes the arrangement

- **WHEN** Polar Coordinates `RectangularToPolar` is applied to a test pattern
- **THEN** the output differs from the input and expresses the rectangular pattern in polar coordinates about the image center

#### Scenario: The transform pair round-trips within resampling tolerance

- **WHEN** Polar Coordinates `RectangularToPolar` is followed by `PolarToRectangular` on the same image dimensions
- **THEN** the result approximates the original within the recorded resampling tolerance

#### Scenario: The two directions differ

- **WHEN** Polar Coordinates `RectangularToPolar` and `PolarToRectangular` are applied to the same buffer
- **THEN** the two outputs differ from each other

#### Scenario: No faithful equivalent is asserted without measurement

- **WHEN** the oracle mapping table is checked
- **THEN** Polar Coordinates has a row naming `-distort DePolar` / `-distort Polar`, tolerance 0, a non-empty no-equivalent note, and the recorded measured maximum and mean delta

### Requirement: ZigZag

The system SHALL implement `Filter::ZigZag { amount: f64, ridges: u32, style: ZigZagStyle }`. It SHALL apply a radial displacement whose magnitude scales with `amount` and whose number of direction reversals from the center to the edge is set by `ridges`, with `ZigZagStyle` selecting the direction: `AroundCenter` rotates about the center, `OutFromCenter` pushes radially, and `PondRipples` biases diagonally. `amount` SHALL be within `-100.0..=100.0` and `ridges` SHALL be within `0..=20`; any value outside its range, and any non-finite `amount`, SHALL be rejected with `FilterError::InvalidParams` and MUST NOT panic. `amount` `0.0` SHALL be a no-op, and `ridges` `0` SHALL produce a single displacement direction with no reversals. `ZigZagStyle` is a closed set that requires no range check. Oracle expectation: no faithful ImageMagick operator (ImageMagick `-swirl` has a different falloff and no ridge model), so ZigZag is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: Zero amount is a no-op

- **WHEN** ZigZag is applied with `amount` `0.0`
- **THEN** the output equals the input within 1 LSB

#### Scenario: Ridges set the reversal count

- **WHEN** ZigZag is applied with `ridges` `0` and with a larger `ridges` count at the same `amount`
- **THEN** the two outputs differ and the larger count introduces more direction reversals from center to edge

#### Scenario: Each style produces a distinct field

- **WHEN** ZigZag is applied with `AroundCenter`, `OutFromCenter`, and `PondRipples`, same parameters
- **THEN** the three outputs differ

#### Scenario: Out-of-range parameters are rejected

- **WHEN** ZigZag is applied with `amount` `-101.0` or `101.0`, or `ridges` `21`
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** ZigZag has no faithful ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Ocean Ripple

The system SHALL implement `Filter::OceanRipple { size: u32, magnitude: u32, seed: u64 }`. It SHALL add a seeded random ripple displacement whose spatial frequency is set by `size` and whose amplitude is set by `magnitude`. `size` SHALL be within `1..=15` and `magnitude` SHALL be within `0..=20`; any value outside its range SHALL be rejected with `FilterError::InvalidParams` and MUST NOT panic, and a rejected parameter MUST leave the buffer unchanged. Applying the same `seed` and parameters to equal input buffers SHALL produce bit-identical output every run, and a different `seed` SHALL be permitted to differ. `magnitude` `0` SHALL be a no-op. Oracle expectation: no faithful ImageMagick operator (ImageMagick `-wave` has a different generator model), so Ocean Ripple is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: The same seed is reproducible

- **WHEN** Ocean Ripple is applied twice with the same `size`, `magnitude`, and `seed` to two clones of one buffer
- **THEN** the two output buffers are bit-identical

#### Scenario: A different seed changes the field

- **WHEN** Ocean Ripple is applied with two different seeds and all other parameters equal
- **THEN** the two outputs are permitted to differ

#### Scenario: Zero magnitude is a no-op

- **WHEN** Ocean Ripple is applied with `magnitude` `0`
- **THEN** the output equals the input within 1 LSB

#### Scenario: Size changes spatial frequency

- **WHEN** Ocean Ripple is applied with `size` `1` and with a larger `size` at the same `magnitude` and `seed`
- **THEN** the two outputs have different spatial frequencies

#### Scenario: Out-of-range parameters are rejected

- **WHEN** Ocean Ripple is applied with `size` `0` or `16`, or `magnitude` `21`
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Ocean Ripple has no faithful ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Displace

The system SHALL implement `Filter::Displace { map: Arc<DisplaceMap>, horizontal_scale: f64, vertical_scale: f64, fit: DisplaceFit, wrap: bool }`. A `DisplaceMap` SHALL hold two 8-bit planes of equal size, horizontal and vertical. Each destination pixel SHALL read the source, bilinearly, at an offset of `(h − 128) × horizontal_scale / 100` pixels across and `(v − 128) × vertical_scale / 100` pixels down, where `h` and `v` are the map's samples for that pixel, so a sample of 128 moves nothing and 100% reaches 127 px at 255 and 128 px at 0. `DisplaceFit::StretchToFit` SHALL scale the map over the buffer, sampling it bilinearly, and `DisplaceFit::Tile` SHALL repeat it at its own size from the buffer's top-left corner. `wrap` SHALL read positions past an edge from the opposite edge (Wrap Around); otherwise it SHALL repeat the nearest edge pixel (Repeat Edge Pixels). A map whose samples are all 128, or scales that are both 0, SHALL leave the buffer bit-identical. Scales outside `-999.0..=999.0` SHALL return `FilterError::InvalidParams`. The alpha plane SHALL be displaced the same way as colour. Oracle expectation: ImageMagick's `-compose displace` with `compose:args` of `1.275 × scale` is a faithful operator, and Repeat Edge Pixels SHALL match it within 8 levels on the oracle test image.

#### Scenario: A neutral map is a no-op

- **WHEN** Displace is applied with a map of all 128, at any scale, to a buffer with translucent pixels
- **THEN** the output equals the input exactly

#### Scenario: Full deflection at 100% reaches about 128 pixels

- **WHEN** Displace is applied at 100% with a horizontal plane of 255 to a picture with a vertical line
- **THEN** the line moves 127 pixels to the left; with a plane of 0 at 50% it moves 64 pixels to the right

#### Scenario: The two planes drive the two axes

- **WHEN** Displace is applied once with only the horizontal plane away from 128 and once with only the vertical plane away from 128
- **THEN** the first shifts content only sideways, the second only up or down, and the two results differ

#### Scenario: One plane twice shifts along the diagonal

- **WHEN** both planes hold the same samples and the scales differ
- **THEN** each pixel moves by the same deflection times each axis's scale

#### Scenario: Stretch and tile place the map differently

- **WHEN** a two-pixel map is applied once with `StretchToFit` and once with `Tile`
- **THEN** stretching spreads each map sample over half the buffer, and tiling alternates them per column

#### Scenario: The undefined-areas modes differ

- **WHEN** Displace pulls content past the right edge with `wrap` true and with `wrap` false
- **THEN** Wrap Around brings in the left edge's pixels and Repeat Edge Pixels repeats the right edge's

#### Scenario: The ImageMagick oracle agrees

- **WHEN** Displace with Repeat Edge Pixels and the oracle's ramp map is diffed against `MAP -compose displace -define compose:args=XxY -composite` with `X = 1.275 × scale`
- **THEN** every sample is within 8 levels at scales 4×4 and 6×−5

### Requirement: Shear row shift

The system SHALL implement `Filter::Shear { curve: Vec<(f64, f64)>, fill: ShearFill }`. It SHALL shift each row horizontally by the control-point `curve`: a cubic Hermite interpolation of the `(position, offset)` points with Catmull-Rom tangents, with `position` and `offset` in `-1.0..=1.0`, maps a row's normalized vertical position (top row `-1.0`, bottom row `1.0`) to a horizontal displacement of `offset × width/2` pixels, and the row SHALL be resampled bilinearly at the displaced source coordinate. The interpolation SHALL be continuous in slope at every control point, and a two-point curve SHALL be exactly the straight line between its points. A positive `offset` SHALL move that row's content to the right. `curve` SHALL contain at least two finite points with strictly increasing `position`; otherwise the system SHALL return `FilterError::InvalidParams`. A curve whose `offset` values are uniformly `0.0` SHALL be a no-op. `ShearFill::WrapAround` SHALL wrap columns shifted off-canvas around to the opposite edge, and `ShearFill::RepeatEdgePixels` SHALL repeat the nearest edge sample; the two modes MUST be observably different where a row's displacement moves columns beyond the image bounds. `ShearFill` is a closed set that requires no range check. Oracle expectation: the closest ImageMagick operator is `-shear {angle}x0` (a horizontal whole-canvas shear that grows and background-fills the canvas), but it is not faithful, so Shear SHALL be classified as no-equivalent with tolerance 0 and a property or known-value test. The measured delta against `-shear -26.565x0 -crop 16x16+4+0 +repage` for the curve `[(-1,-0.5),(1,0.5)]` is max 255 / mean 18.4 for `RepeatEdgePixels` (mean 23.4 for `WrapAround`).

#### Scenario: A flat curve is a no-op

- **WHEN** Shear is applied with a curve whose `offset` values are all `0.0`
- **THEN** the output equals the input within 1 LSB

#### Scenario: A curved control-point set displaces rows horizontally

- **WHEN** Shear is applied with a curve that is `0.0` at the top row and bends to a non-zero `offset` lower down
- **THEN** the top row is unchanged, the lower rows shift horizontally, and a row with a positive `offset` moves its content to the right

#### Scenario: A two-point curve is a straight line

- **WHEN** Shear is applied with exactly two control points at `(-1.0, -0.5)` and `(1.0, 0.5)`
- **THEN** the horizontal displacement grows linearly with the row position, with no easing at the ends

#### Scenario: An invalid curve is rejected

- **WHEN** Shear is applied with fewer than two points, a non-finite coordinate, `position` values that are not strictly increasing, or a `position` / `offset` outside `-1.0..=1.0`
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: The two fill modes differ off-canvas

- **WHEN** Shear with the same curve but `ShearFill::WrapAround` and `ShearFill::RepeatEdgePixels` is applied to a buffer whose displacement moves rows beyond an edge
- **THEN** the two outputs differ

#### Scenario: No faithful equivalent is asserted without measurement

- **WHEN** the oracle mapping table is checked
- **THEN** Shear has a row naming `-shear {angle}x0`, tolerance 0, a non-empty no-equivalent note, and the recorded measured maximum and mean delta

### Requirement: Distort alpha follows the kernel

For 4-channel buffers, every Distort filter SHALL resample channel 4 through the same warp as the colour planes, so the layer edge moves with the colour, and a channel 4 that is 255 everywhere SHALL stay 255.

#### Scenario: A warp moves a soft edge

- **WHEN** a Distort variant is applied to an RGBA buffer whose alpha is a soft-edged disc
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: An opaque buffer stays opaque

- **WHEN** a Distort variant is applied to an RGBA buffer whose alpha is 255 everywhere
- **THEN** every alpha sample is still 255
