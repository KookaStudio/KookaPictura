# pixelate-filters Specification

## Purpose
Mosaic, Crystallize, Facet, Fragment, Mezzotint, Pointillize, and Color Halftone with validation and oracle classification.

## Requirements

### Requirement: Pixelate filter application and error contract

The system SHALL provide `pictura_filters::apply(filter: &Filter, buf: &mut PixelBuffer) -> Result<(), FilterError>`. It SHALL operate in place on a planar 8-bit buffer whose `channels` is 3 (RGB) or 4 (RGBA), treating channels 1 through 3 as the color planes and channel 4, when present, as alpha. It SHALL transform every color sample or return an error; it MUST NOT partially apply a filter and then fail. Malformed buffers MUST return `FilterError` instead of panicking.

#### Scenario: Apply a Pixelate filter to a 3-channel planar buffer

- **WHEN** `apply` receives a Pixelate `Filter` variant and a 3-channel planar buffer
- **THEN** the R, G, and B planes are rewritten in place and `Ok(())` is returned

#### Scenario: Reject an unsupported channel count

- **WHEN** the buffer has a channel count other than 3 or 4
- **THEN** `apply` returns `FilterError::Unsupported` and leaves the buffer unchanged

#### Scenario: Malformed buffers error instead of panicking

- **WHEN** `apply` receives an empty buffer, or a buffer whose `data.len()` does not equal `width * height * channels`, or a buffer with zero width or height
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Parameter validation for Pixelate filters

The system SHALL validate each Pixelate filter's parameters before writing any sample. `Mosaic` `cell_size` SHALL be within `2..=200`; `Crystallize` `cell_size` SHALL be within `3..=300`; `Pointillize` `cell_size` SHALL be within `3..=300`; `ColorHalftone` `max_radius` SHALL be within `4..=127` and every entry of `angles` SHALL be finite. A value outside those ranges, a non-finite angle, or a `MezzotintType` outside the enumerated set SHALL be rejected with `FilterError::InvalidParams` and MUST NOT panic. `Facet` and `Fragment` take no parameters.

#### Scenario: Out-of-range cell sizes are rejected

- **WHEN** Mosaic is applied with `cell_size` 1 or 201, Crystallize with `cell_size` 2 or 301, or Pointillize with `cell_size` 2 or 301
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: Out-of-range Color Halftone arguments are rejected

- **WHEN** Color Halftone is applied with `max_radius` 3 or 128, or with a non-finite angle
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

#### Scenario: Minimum cell sizes are accepted

- **WHEN** Mosaic is applied with `cell_size` 2, Crystallize with `cell_size` 3, Pointillize with `cell_size` 3, and Color Halftone with `max_radius` 4
- **THEN** each returns `Ok(())` with no division-by-zero or empty-region panic

### Requirement: Mosaic

The system SHALL implement `Filter::Mosaic { cell_size: u32 }`. It SHALL partition the color planes into `cell_size × cell_size` square blocks, compute each block's mean color, and write that mean to every pixel in the block. A partially covered block at the image edge SHALL average over its available pixels. Mosaic SHALL be deterministic. Oracle expectation: differential against ImageMagick box downsample then point upsample (a block average) at a cell size that divides both image dimensions, within an absolute tolerance of 0; non-dividing cell sizes drift because the partial-edge-block handling differs, and that divergence is recorded.

#### Scenario: Every pixel in a block equals the block mean

- **WHEN** Mosaic is applied to a buffer containing a known block and the block's mean is computed independently
- **THEN** every pixel in that block equals the mean within 1 LSB

#### Scenario: Larger cells produce larger blocks

- **WHEN** Mosaic is applied at `cell_size` 4 and at `cell_size` 16 to the same non-uniform buffer
- **THEN** the larger cell size yields fewer distinct blocks and a coarser output

#### Scenario: Matches ImageMagick

- **WHEN** Mosaic is applied to the oracle image at a cell size dividing both dimensions and ImageMagick box-downsamples then point-upsamples by the same cell size
- **THEN** no sample differs

### Requirement: Crystallize

The system SHALL implement `Filter::Crystallize { cell_size: u32, seed: u64 }`. For each `cell_size × cell_size` grid cell it SHALL scatter one seed point jittered by the seeded RNG, assign every pixel to its nearest seed, and fill that pixel with the mean color of the pixels assigned to the same seed. `cell_size` SHALL be within `3..=300`. The same seed applied to equal input buffers SHALL produce bit-identical output; a different seed SHALL be permitted to differ. Oracle expectation: no faithful ImageMagick operator, so Crystallize is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: Larger cells produce larger polygons

- **WHEN** Crystallize is applied at `cell_size` 4 and at `cell_size` 40 to the same buffer with the same seed
- **THEN** the larger cell size yields fewer, larger regions of constant color

#### Scenario: The same seed is reproducible

- **WHEN** Crystallize is applied twice with the same seed to two clones of one buffer
- **THEN** the two output buffers are bit-identical

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Crystallize has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Facet

The system SHALL implement `Filter::Facet` with no parameters. It SHALL perform a fixed number of iterative passes that replace each pixel with the local average of similar-valued neighbors, flattening gradual detail into patches while leaving strong edges between unlike colors distinct. Oracle expectation: no faithful ImageMagick operator, so Facet is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: A gradient is flattened

- **WHEN** Facet is applied to a smooth gradient
- **THEN** the output contains broader regions of constant color than the input

#### Scenario: A strong edge is preserved

- **WHEN** Facet is applied to an image with a step edge between distinct colors
- **THEN** the two sides remain distinguishable rather than being averaged into one

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Facet has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Fragment

The system SHALL implement `Filter::Fragment` with no parameters. It SHALL create four copies of the color planes, offset them by a small fixed `(dx, dy)`, and average them, producing a deterministic ghost. On a flat-color region Fragment SHALL be a bit-exact no-op. Oracle expectation: no faithful ImageMagick operator; Fragment is covered by a known-value test that checks the four-tap offset average, classified as no-equivalent with tolerance 0 and the observed delta recorded.

#### Scenario: The result equals the four-copy average

- **WHEN** Fragment is applied to a buffer and the average of the four offset copies is computed independently
- **THEN** the output equals that average within 1 LSB

#### Scenario: A flat region is a no-op

- **WHEN** Fragment is applied to a uniform-color buffer
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Fragment has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Mezzotint

The system SHALL implement `Filter::Mezzotint { kind: MezzotintType, seed: u64 }`. It SHALL generate a procedural dot, line, or stroke pattern selected by `kind` over the image, using the seeded RNG so the same seed reproduces the result bit for bit. A grayscale image SHALL use the luma pattern; a color image SHALL retain fully saturated color patterns. Different `kind` values SHALL produce visibly different patterns. Oracle expectation: no faithful ImageMagick operator, so Mezzotint is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: Each type produces a distinct pattern

- **WHEN** Mezzotint is applied with two different `MezzotintType` values to the same buffer and seed
- **THEN** the two outputs differ

#### Scenario: The same seed is reproducible

- **WHEN** Mezzotint is applied twice with the same `kind` and seed to two clones of one buffer
- **THEN** the two output buffers are bit-identical

#### Scenario: A different seed changes the pattern

- **WHEN** Mezzotint is applied with two different seeds to the same buffer
- **THEN** the two outputs differ

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Mezzotint has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Pointillize

The system SHALL implement `Filter::Pointillize { cell_size: u32, background: [u8; 3], seed: u64 }`. It SHALL scatter dots whose radius is proportional to `cell_size`, fill each dot with the local source color, and leave `background` as the canvas between dots. `cell_size` SHALL be within `3..=300`. The same seed applied to equal input buffers SHALL produce bit-identical output. Oracle expectation: no faithful ImageMagick operator, so Pointillize is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: The background shows between dots

- **WHEN** Pointillize is applied to a buffer whose content differs from `background`
- **THEN** at least one pixel outside the dot coverage equals `background`

#### Scenario: Dot size scales with cell size

- **WHEN** Pointillize is applied at `cell_size` 4 and at `cell_size` 40 with the same seed
- **THEN** the larger cell size yields visibly larger dots

#### Scenario: The same seed is reproducible

- **WHEN** Pointillize is applied twice with the same seed to two clones of one buffer
- **THEN** the two output buffers are bit-identical

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Pointillize has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Color Halftone

The system SHALL implement `Filter::ColorHalftone { max_radius: u32, angles: [f64; 4] }`. Per color channel it SHALL lay down a rotated grid at `angles[channel]` whose spacing derives from `max_radius`, replace each cell with a dot whose radius is proportional to the cell's brightness, and combine the channels. A grayscale image SHALL use `angles[0]`; a 3-channel image SHALL use the first three angles. Other channel counts SHALL degrade gracefully by reusing an available angle rather than reading out of bounds. `max_radius` SHALL be within `4..=127` and every angle SHALL be finite. Oracle expectation: no faithful ImageMagick operator, so Color Halftone is classified as no-equivalent with tolerance 0 and property tests, and the observed delta SHALL be recorded.

#### Scenario: Increasing the radius enlarges the dots

- **WHEN** Color Halftone is applied at `max_radius` 5 and at `max_radius` 50 to a non-uniform buffer
- **THEN** the larger radius produces larger halftone dots

#### Scenario: Grayscale responds to one angle

- **WHEN** Color Halftone is applied to a single-channel buffer with different values in `angles[1..3]`
- **THEN** the output depends only on `angles[0]` and does not panic

#### Scenario: A three-channel image uses the first three angles

- **WHEN** Color Halftone is applied to a 3-channel RGB buffer
- **THEN** each color plane is screened at its corresponding angle from `angles[0..3]`

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Color Halftone has no ImageMagick operator, tolerance 0, a non-empty no-equivalent note, and a recorded observed delta

### Requirement: Clamp-to-edge borders and tiny images for Pixelate filters

Every neighborhood Pixelate filter SHALL sample with clamp-to-edge at the image borders, so an out-of-range index maps to the nearest edge sample. When a cell extends beyond the image bounds it SHALL aggregate over the available pixels rather than reading out of bounds. A 1×1 image and a 1-pixel-wide or 1-pixel-tall image MUST NOT panic and MUST return either a result or a typed error.

#### Scenario: A 1x1 image does not panic

- **WHEN** every Pixelate variant is applied to a 1×1 buffer
- **THEN** no variant panics and each returns `Ok(())` or a `FilterError`

#### Scenario: A cell larger than the image does not panic

- **WHEN** Mosaic with `cell_size` 200 or Crystallize with `cell_size` 300 is applied to a 2×3 buffer
- **THEN** no variant panics and each returns `Ok(())` or a `FilterError`

#### Scenario: Border pixels clamp to the edge

- **WHEN** a neighborhood Pixelate filter is applied to a small buffer and a border pixel is inspected
- **THEN** the result is consistent with repeating the edge sample rather than reading out of bounds

### Requirement: Deterministic Pixelate output

Pixelate filters SHALL be deterministic. Mosaic, Facet, Fragment, and Color Halftone SHALL produce bit-identical output for equal input with no random, time, or thread-order dependence. Crystallize, Mezzotint, and Pointillize SHALL be seeded: applying the same seed to equal input buffers SHALL produce bit-identical output every run.

#### Scenario: Non-random variants are repeatable

- **WHEN** Mosaic, Facet, Fragment, or Color Halftone is applied to two clones of one buffer
- **THEN** the two output buffers are bit-identical

#### Scenario: Seeded variants are repeatable

- **WHEN** Crystallize, Mezzotint, or Pointillize is applied twice with the same seed to two clones of one buffer
- **THEN** the two output buffers are bit-identical

### Requirement: Pixelate ImageMagick oracle and no-equivalent classification

The system SHALL ship `scripts/filter_oracle.py`, which applies an ImageMagick operator to a raw 8-bit image, and `crates/pictura-filters/tests/oracle.rs`, which diffs `apply` against it. The mapping table SHALL have exactly one row per Pixelate `Filter` variant. Mosaic SHALL be diffed against the box-downsample / point-upsample block average within its stated tolerance. Crystallize, Facet, Fragment, Mezzotint, Pointillize, and Color Halftone SHALL be classified as no-equivalent with tolerance 0, a property or known-value test, a non-empty note, and the observed delta recorded. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table covers every Pixelate variant

- **WHEN** the oracle tests run
- **THEN** every Pixelate variant has a table row, each no-equivalent row has tolerance 0, and each row carries a non-empty note

#### Scenario: Mosaic matches the oracle

- **WHEN** Mosaic is applied to the oracle image and ImageMagick performs the equivalent block average
- **THEN** no sample differs by more than the stated tolerance

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes

### Requirement: Pixelate alpha follows the kernel

For 4-channel buffers, Mosaic, Crystallize, and Fragment SHALL run their kernel over channel 4 as they do over the colour planes, and a channel 4 that is 255 everywhere SHALL stay 255. Facet, Mezzotint, Pointillize, and Color Halftone SHALL leave channel 4 bit-identical; only channels 1 through 3 SHALL be modified.

#### Scenario: Mosaic, Crystallize, and Fragment move a soft edge

- **WHEN** Mosaic, Crystallize, or Fragment is applied to an RGBA buffer whose alpha is a soft-edged disc
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: The value-clustering variants preserve alpha

- **WHEN** Facet, Mezzotint, Pointillize, or Color Halftone is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit
