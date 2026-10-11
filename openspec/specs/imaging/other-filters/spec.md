# other-filters Specification

## Purpose
Maximum, Minimum, Offset, High Pass, and Custom with edge handling, determinism, and an oracle classification.

## Requirements

### Requirement: Other filter application and error contract

The system SHALL provide `pictura_filters::apply(filter: &Filter, buf: &mut PixelBuffer) -> Result<(), FilterError>`. It SHALL operate in place on a planar 8-bit buffer whose `channels` is 3 (RGB) or 4 (RGBA), treating channels 1 through 3 as the color planes and channel 4, when present, as alpha. It SHALL transform every color sample or return an error; it MUST NOT partially apply a filter and then fail. Malformed buffers MUST return `FilterError` instead of panicking.

#### Scenario: Apply an Other filter to a 3-channel planar buffer

- **WHEN** `apply` receives an Other `Filter` variant and a 3-channel planar buffer
- **THEN** the R, G, and B planes are rewritten in place and `Ok(())` is returned

#### Scenario: Reject an unsupported channel count

- **WHEN** the buffer has a channel count other than 3 or 4
- **THEN** `apply` returns `FilterError::Unsupported` and leaves the buffer unchanged

#### Scenario: Malformed buffers error instead of panicking

- **WHEN** `apply` receives an empty buffer, or a buffer whose `data.len()` does not equal `width * height * channels`, or a buffer with zero width or height
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Alpha channel preservation for Other filters

For 4-channel buffers, every Other filter SHALL leave channel 4 bit-identical. Only channels 1 through 3 SHALL be modified.

#### Scenario: Every Other variant preserves alpha

- **WHEN** each Other variant is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Maximum and Minimum

The system SHALL implement `Filter::Maximum { radius: u32 }` and `Filter::Minimum { radius: u32 }`. For each color channel independently, every output sample SHALL be the maximum (`Maximum`) or the minimum (`Minimum`) of the input samples within the `(2r+1)²` square footprint centered on it, where `r` is `radius`. `radius == 0` SHALL be a bit-exact no-op. `radius` SHALL be clamped to 100; a request above 100 MUST behave as 100 and MUST NOT error. Both filters SHALL be deterministic. Oracle expectation: differential against ImageMagick `-morphology Dilate Square:{radius}` (Maximum) and `-morphology Erode Square:{radius}` (Minimum) — ImageMagick's `Square:N` argument is the radius, not the diameter — within an absolute tolerance of 0.

#### Scenario: Maximum grows bright regions and Minimum shrinks them

- **WHEN** Maximum and Minimum are each applied at the same non-zero radius to a white-on-black mask
- **THEN** the white region grows under Maximum and shrinks under Minimum

#### Scenario: Radius zero is a no-op

- **WHEN** Maximum or Minimum is applied with radius 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Radius above 100 clamps

- **WHEN** Maximum or Minimum is applied with radius 150 and with radius 100 to the same buffer
- **THEN** both outputs are bit-identical and no error is returned

#### Scenario: Matches ImageMagick

- **WHEN** Maximum or Minimum with a given radius is applied to the oracle image and ImageMagick applies the matching `Square:N` morphology
- **THEN** no sample differs by more than 1

### Requirement: Offset

The system SHALL implement `Filter::Offset { horizontal: i32, vertical: i32, wrap: bool, background: [u8; 3], transparent: bool }` as a translation of the colour planes by `(horizontal, vertical)` with clamp-to-edge or wrap edge handling, and of the transparency plane on a four-channel buffer. A positive `horizontal` SHALL move image content toward increasing x and a positive `vertical` toward increasing y. When `wrap` is true the planes SHALL wrap around (modulo the image dimensions); when `wrap` is false the exposed colour area SHALL be filled with `background` and the exposed transparency SHALL be 255 (`transparent == false`, CS6's "Set to Background") or 0 (`transparent == true`, CS6's "Set to Transparent"). `horizontal == 0 && vertical == 0` SHALL be a bit-exact no-op. Oracle expectation: differential against ImageMagick `-roll +h+v` for the `wrap == true` case within tolerance 0. The `wrap == false` background fill has no faithful ImageMagick operator, so it is classified as no-equivalent and covered by known-value tests with the divergence documented.

#### Scenario: Wrap re-enters on the opposite side

- **WHEN** Offset is applied with `wrap == true` and `horizontal` equal to the image width
- **THEN** the output equals the input and no content is lost

#### Scenario: Background fills the exposed area

- **WHEN** Offset is applied with `wrap == false` and `transparent == false` to an image whose content is distinct from `background`
- **THEN** every exposed colour pixel equals `background`, the exposed transparency is 255, and the moved content matches the input

#### Scenario: Transparent fills the exposed area clear

- **WHEN** Offset is applied with `wrap == false` and `transparent == true` to a four-channel image
- **THEN** every exposed colour pixel equals `background`, the exposed transparency is 0, and the moved content matches the input

#### Scenario: Zero offset is a no-op

- **WHEN** Offset is applied with `horizontal == 0 && vertical == 0`
- **THEN** the buffer is bit-exactly unchanged regardless of `wrap` and `transparent`

#### Scenario: Background fill has no faithful equivalent

- **WHEN** the oracle mapping table is checked
- **THEN** the Offset background-fill case has no ImageMagick operator, tolerance 0, and a non-empty no-equivalent note

### Requirement: High Pass

The system SHALL implement `Filter::HighPass { radius: f64 }` per color channel as `out = clamp(orig − gaussian(orig, sigma) + 128)`, where `sigma = sigma_from_radius(radius)` reuses the shared M6 Gaussian kernel and `clamp` bounds the result to `0..=255`. `radius` SHALL be finite and strictly greater than 0; a non-finite, zero, or negative radius SHALL be rejected with `FilterError::InvalidParams`. A uniform-color region SHALL map to mid-gray 128. Oracle expectation: verified against an ImageMagick `-compose mathematics` recipe if a faithful one is found; otherwise High Pass is classified as no-equivalent with the divergence documented and covered by known-value tests.

#### Scenario: A uniform field becomes mid-gray

- **WHEN** High Pass is applied to a uniform-color buffer
- **THEN** every output color sample equals 128 within 1 LSB

#### Scenario: Edge detail is retained

- **WHEN** High Pass is applied to an image containing a step edge
- **THEN** the deviation from 128 is larger at the edge than in the flat regions

#### Scenario: Invalid radius is rejected

- **WHEN** High Pass is applied with radius 0, a negative radius, or a non-finite radius such as NaN
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

#### Scenario: Oracle classification is recorded

- **WHEN** the oracle mapping table is checked
- **THEN** High Pass carries either a verified `-compose mathematics` recipe or a no-equivalent note with tolerance 0 and a non-empty divergence description

### Requirement: Custom

The system SHALL implement `Filter::Custom { kernel: [[f64; 5]; 5], scale: f64, offset: f64 }` as a 5×5 convolution over each color channel with clamp-to-edge sampling, f64 accumulation, and `out = clamp(Σ kernel[i][j] · src(x + i, y + j) / scale + offset)`. Kernel entries SHALL be traversed left-to-right, top-to-bottom with `kernel[2][2]` at the evaluated pixel. `scale` SHALL be finite and non-zero; `scale`, `offset`, and every kernel entry SHALL be finite. `scale == 0` or any non-finite parameter SHALL be rejected with `FilterError::InvalidParams`. The identity kernel (a single `1.0` at `kernel[2][2]`, all other entries `0.0`) with `scale 1` and `offset 0` SHALL be a bit-exact pass-through. Oracle expectation: differential against ImageMagick `-convolve` with the kernel-sum/scale normalization and the offset applied via `-evaluate add`, within an absolute tolerance of 0.

#### Scenario: The identity kernel is a pass-through

- **WHEN** Custom is applied with the unit impulse at `kernel[2][2]`, `scale 1`, and `offset 0`
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: The embedded 3x3 mean is a box blur

- **WHEN** Custom is applied with the embedded 3×3 all-ones kernel, `scale 9`, and `offset 0`
- **THEN** the result equals the box blur of the input within 1 LSB

#### Scenario: Scale zero is rejected

- **WHEN** Custom is applied with `scale == 0`
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

#### Scenario: Non-finite parameters are rejected

- **WHEN** Custom is applied with a non-finite kernel entry, `scale`, or `offset`
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

#### Scenario: Matches ImageMagick

- **WHEN** Custom with a given kernel is applied to the oracle image and ImageMagick applies the equivalent `-convolve` with the matching scale and bias
- **THEN** no sample differs by more than 1

### Requirement: Clamp-to-edge borders and tiny images for Other filters

Every neighborhood Other filter (Maximum, Minimum, High Pass, and Custom) SHALL sample with clamp-to-edge at the image borders, so an out-of-range index maps to the nearest edge sample. Offset SHALL use the same clamp rule for `wrap == false` and modulo for `wrap == true`. A 1×1 image and a 1-pixel-wide or 1-pixel-tall image MUST NOT panic and MUST return either a result or a typed error.

#### Scenario: A 1x1 image does not panic

- **WHEN** every Other variant is applied to a 1×1 buffer
- **THEN** no variant panics and each returns `Ok(())` or a `FilterError`

#### Scenario: Border pixels clamp to the edge

- **WHEN** Maximum, Minimum, High Pass, or Custom is applied to a small buffer and a border pixel is inspected
- **THEN** the result is consistent with repeating the edge sample rather than reading out of bounds

### Requirement: Deterministic Other output

Other filters SHALL be deterministic. The same filter applied to equal input buffers SHALL produce bit-identical output every run, with no random, time, or thread-order dependence.

#### Scenario: Repeated runs match

- **WHEN** each Other variant is applied to two clones of one buffer
- **THEN** the two output buffers are bit-identical

### Requirement: Other ImageMagick oracle and no-equivalent classification

The system SHALL ship `scripts/filter_oracle.py`, which applies an ImageMagick operator to a raw 8-bit image, and `crates/pictura-filters/tests/oracle.rs`, which diffs `apply` against it. The mapping table SHALL have exactly one row per Other `Filter` variant. Maximum and Minimum (`-morphology Dilate/Erode Square:{radius}`), Offset wrap (`-roll`), and Custom (`-convolve`) SHALL be diffed within their stated tolerances. Offset background fill, and High Pass unless a faithful `-compose mathematics` recipe is verified, SHALL be classified as no-equivalent with tolerance 0, a property or known-value test, and a non-empty note. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table covers every Other variant

- **WHEN** the oracle tests run
- **THEN** every Other variant has a table row, each no-equivalent row has tolerance 0, and each row carries a non-empty note

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes
