## RENAMED Requirements

- FROM: `### Requirement: Surface blur (bilateral)`
- TO: `### Requirement: Surface blur`

## MODIFIED Requirements

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

### Requirement: Blur ImageMagick oracle and no-equivalent classification

The system SHALL ship `scripts/filter_oracle.py`, which applies an ImageMagick operator to a raw 8-bit image, and `crates/pictura-filters/tests/oracle.rs`, which diffs `apply` against it. The mapping table SHALL have exactly one row per `Filter` variant. Gaussian, Box, Blur, and Blur More SHALL be diffed (`-gaussian-blur 0xσ`, `-statistic mean NxN`, `-gaussian-blur 0x0.7`, `-gaussian-blur 0x2`) within their stated tolerances. Motion, Average, Radial, and Surface SHALL be classified as having no faithful ImageMagick operator and SHALL use tolerance 0 with property or known-value tests — for Motion because ImageMagick's `-motion-blur` is a one-sided Gaussian line kernel, not a symmetric uniform streak. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table covers every blur variant

- **WHEN** the oracle tests run
- **THEN** every blur variant has a table row, each no-equivalent row has tolerance 0, and each row carries a non-empty note

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes
