# noise-filters Specification

## Purpose
Add Noise, Median, and Despeckle with alpha preservation, seeded determinism, and an oracle classification.

## Requirements

### Requirement: Noise filter application and error contract

The system SHALL provide `pictura_filters::apply(filter: &Filter, buf: &mut PixelBuffer) -> Result<(), FilterError>`. It SHALL operate in place on a planar 8-bit buffer whose `channels` is 3 (RGB) or 4 (RGBA), treating channels 1 through 3 as the color planes and channel 4, when present, as alpha. It SHALL transform every color sample or return an error; it MUST NOT partially apply a filter and then fail. Malformed buffers MUST return `FilterError` instead of panicking.

#### Scenario: Apply a noise filter to a 3-channel planar buffer

- **WHEN** `apply` receives a noise `Filter` variant and a 3-channel planar buffer
- **THEN** the R, G, and B planes are rewritten in place and `Ok(())` is returned

#### Scenario: Reject an unsupported channel count

- **WHEN** the buffer has a channel count other than 3 or 4
- **THEN** `apply` returns `FilterError::Unsupported` and leaves the buffer unchanged

#### Scenario: Malformed buffers error instead of panicking

- **WHEN** `apply` receives an empty buffer, or a buffer whose `data.len()` does not equal `width * height * channels`, or a buffer with zero width or height
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Add Noise

The system SHALL implement `Filter::AddNoise { amount: f64, distribution: NoiseDistribution, monochromatic: bool, seed: u64 }`. `amount` is a percentage of half the tonal range: for `NoiseDistribution::Uniform` it SHALL add a delta drawn uniformly from `[−m, +m]` with `m = amount / 100 × 127.5`; for `NoiseDistribution::Gaussian` it SHALL add a zero-mean, bell-shaped delta whose spread is proportional to `amount`. `amount` SHALL be a finite percentage in `0..=400`; `amount == 0` SHALL be a no-op, and negative, non-finite, or `> 400` values SHALL be rejected with `FilterError::InvalidParams`. When `monochromatic` is true it SHALL draw one delta per pixel and apply it to all three color channels, preserving hue; otherwise it SHALL draw a delta per channel, producing colored speckle. The deltas SHALL come from a deterministic per-pixel hash seeded by `seed`. Oracle expectation: no faithful ImageMagick equivalent exists (noise streams differ), so statistical tests cover the distribution shape and same-seed tests cover reproducibility, and the divergence is documented.

#### Scenario: Uniform noise is zero-mean

- **WHEN** Add Noise with `Uniform` is applied to a uniform mid-grey buffer
- **THEN** the mean stays within tolerance of the input and the standard deviation grows with `amount`

#### Scenario: Gaussian noise has a bell-shaped histogram

- **WHEN** Add Noise with `Gaussian` and a sufficiently large amount is applied to a uniform buffer
- **THEN** the delta histogram is bell-shaped rather than flat

#### Scenario: Monochromatic noise preserves hue

- **WHEN** monochromatic Add Noise is applied to a colored buffer
- **THEN** the per-pixel channel differences R−G and G−B stay within tolerance of the input, while non-monochromatic Add Noise changes them

#### Scenario: Amount zero is a no-op

- **WHEN** Add Noise is applied with amount 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Out-of-range amount is rejected

- **WHEN** Add Noise is applied with a negative amount or an amount above 400
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

### Requirement: Median filter

The system SHALL implement `Filter::Median { radius: u32 }` as a per-channel rank filter that replaces each pixel with the median of the channel values in its `(2r+1)²` window, sampled with clamp-to-edge. `radius` SHALL span `0..=100`; `radius == 0` SHALL be a no-op and a radius above 100 SHALL be rejected with `FilterError::InvalidParams`. Oracle expectation: differential against ImageMagick `-median R` within an absolute tolerance of 1 per sample.

#### Scenario: Salt-and-pepper noise is removed

- **WHEN** Median with radius 1 is applied to an image containing isolated impulse pixels
- **THEN** the impulses are removed while a step edge is preserved within a small band

#### Scenario: Radius zero is a no-op

- **WHEN** Median is applied with radius 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Matches ImageMagick

- **WHEN** Median with a given radius is applied to the oracle image and ImageMagick applies `-median R`
- **THEN** no sample differs by more than 1

### Requirement: Despeckle

The system SHALL implement `Filter::Despeckle` with no parameters. It SHALL detect edges by gradient or color difference and apply a smoothing operation only to pixels not on an edge, so isolated noise is smoothed while strong edges are preserved. Oracle expectation: no faithful ImageMagick equivalent exists (the edge detector and smoothing operator are closed), so a property test covers the isolated-noise smoothing with a preserved edge profile, and the divergence is documented.

#### Scenario: Isolated noise is smoothed

- **WHEN** Despeckle is applied to an image with isolated speckle over a smooth region
- **THEN** the speckle is reduced relative to the input

#### Scenario: Strong edges are preserved

- **WHEN** Despeckle is applied to an image containing a strong step edge
- **THEN** the edge profile stays within tolerance of the input

### Requirement: Seeded RNG determinism

Add Noise SHALL be the only random noise filter. It SHALL drive its RNG from the `seed` field, so two applies with the same seed and input produce bit-identical output, two applies with different seeds produce different output, and a re-apply of the same filter reproduces the first result exactly. Median and Despeckle SHALL be deterministic and SHALL NOT touch the RNG.

#### Scenario: Same seed is bit-identical

- **WHEN** Add Noise with the same seed is applied to two clones of one buffer
- **THEN** the two output buffers are bit-identical

#### Scenario: Different seeds differ

- **WHEN** Add Noise is applied to the same buffer with two different seeds
- **THEN** the two output buffers differ

#### Scenario: Median and Despeckle are deterministic

- **WHEN** Median or Despeckle is applied twice to equal input buffers
- **THEN** the outputs are bit-identical

### Requirement: Clamp-to-edge borders and tiny images for noise

Every neighborhood noise filter SHALL sample with clamp-to-edge at the image borders and SHALL clamp its window to the available source. A 1×1 image and a 1-pixel-wide or 1-pixel-tall image MUST NOT panic and MUST return either a result or a typed error.

#### Scenario: A tiny image does not panic

- **WHEN** every noise variant is applied to a 1×1 buffer and to a 1-pixel-wide buffer
- **THEN** no variant panics and each returns `Ok(())` or a `FilterError`

#### Scenario: Border pixels clamp to the edge

- **WHEN** Median is applied to a small buffer and a pixel at the border is inspected
- **THEN** the result is consistent with repeating the edge sample rather than reading outside the buffer

### Requirement: Noise ImageMagick oracle and no-equivalent classification

The system SHALL ship `scripts/filter_oracle.py` and `crates/pictura-filters/tests/oracle.rs`. The mapping table SHALL have exactly one row per `Filter` variant. Median SHALL be diffed against `-median R` within its stated tolerance. Add Noise and Despeckle SHALL be classified as having no faithful ImageMagick operator and SHALL use tolerance 0 with statistical, same-seed, or property tests. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table covers every noise variant

- **WHEN** the oracle tests run
- **THEN** every noise variant has a table row, each no-equivalent row has tolerance 0, and each row carries a non-empty note

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes

### Requirement: Dust & Scratches

The system SHALL implement `Filter::DustAndScratches { radius: u32, threshold: u32 }`.
For each color plane it SHALL compute the median over the `(2r+1)²` neighborhood
sampled clamp-to-edge, and SHALL replace the center sample with that median only
when the absolute difference between the center and the median is greater than
`threshold`; otherwise the sample SHALL be left unchanged. `threshold` 0 SHALL
therefore behave as Median (every dissimilar sample is replaced) and `threshold`
255 SHALL replace nothing. `radius` SHALL span `1..=16` and `threshold` SHALL
span `0..=255`; a value outside either range SHALL be rejected with
`FilterError::InvalidParams` before any mutation. A uniform-color plane SHALL be
bit-exactly unchanged. Alpha SHALL be left bit-identical. Oracle expectation: no
faithful ImageMagick equivalent exists (the local statistic and gate are closed),
so property tests cover the gate and the radius, and the divergence is
documented.

#### Scenario: A scratch at threshold 0 is replaced by the median

- **WHEN** Dust & Scratches with radius 1 and threshold 0 is applied to a smooth plane containing an isolated bright speck
- **THEN** the speck sample takes the local median while the surrounding smooth samples are unchanged

#### Scenario: Threshold 255 replaces nothing

- **WHEN** Dust & Scratches with threshold 255 is applied to a smooth plane containing an isolated bright speck
- **THEN** the speck sample is left bit-identical

#### Scenario: A uniform plane is unchanged

- **WHEN** Dust & Scratches is applied to a uniform-color plane
- **THEN** the plane is bit-exactly the same as the input

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** Dust & Scratches is applied with radius 0 or radius 17, or with threshold 256
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

#### Scenario: Alpha is preserved

- **WHEN** Dust & Scratches is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Reduce Noise

The system SHALL implement `Filter::ReduceNoise(ReduceNoiseParams)` over the
colour planes; alpha SHALL be left bit-identical. Each colour plane SHALL be
smoothed by an edge-preserving bilateral filter whose range sigma grows with the
channel's Strength (`0..=10`) and shrinks with its Preserve Details (`0..=100`),
so Strength 0 is a no-op and, at equal Strength, a higher Preserve Details
retains more edge contrast. When Reduce Color Noise (`0..=100`) is above 0 the
colour SHALL be blurred and recombined with the original luminance, lowering
chroma variance without changing luminance. When Sharpen Details (`0..=100`) is
above 0 an unsharp pass SHALL be added on luminance. When Remove JPEG Artifacts
is set, the two samples straddling each 8-pixel block seam SHALL be averaged
when their step is below a fixed gate. The filter SHALL be deterministic (no
RNG) and SHALL reject any parameter outside its range with
`FilterError::InvalidParams` before mutating the buffer. Oracle expectation: no
faithful ImageMagick equivalent exists (the kernel is closed and ImageMagick has
no bilateral or Reduce-Noise operator), so property tests and a kernel digest
cover it, and the divergence is documented.

#### Scenario: Strength lowers luminance noise monotonically

- **WHEN** Reduce Noise is applied to a noisy grey image at increasing Strength
  with Preserve Details 0
- **THEN** the luminance variance does not grow with Strength and is lower at
  high Strength

#### Scenario: Preserve Details retains edge contrast

- **WHEN** Reduce Noise is applied to a subtle noisy edge at equal Strength with
  Preserve Details 0 and 100
- **THEN** the 100 output retains more contrast than the 0 output

#### Scenario: Reduce Color Noise lowers chroma variance

- **WHEN** Reduce Noise with Reduce Color Noise above 0 is applied to a grey
  image with per-channel jitter
- **THEN** the chroma variance falls while the luminance is kept

#### Scenario: Remove JPEG Artifacts softens block seams

- **WHEN** Remove JPEG Artifacts is applied to an image of 8×8 blocks
- **THEN** the step across the block seams is reduced relative to the same
  settings with it off

#### Scenario: Deterministic and alpha-preserving

- **WHEN** Reduce Noise is applied twice to equal RGBA buffers
- **THEN** the outputs are bit-identical and the alpha plane is unchanged

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** Reduce Noise is applied with a negative or over-range Strength,
  Preserve Details, Reduce Color Noise or Sharpen Details
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is
  bit-identical to its prior state

### Requirement: Noise alpha follows the kernel

For 4-channel buffers, Median, Despeckle, and Dust & Scratches SHALL run their kernel over channel 4 as they do over the colour planes, and a channel 4 that is 255 everywhere SHALL stay 255. Add Noise SHALL leave channel 4 bit-identical; only channels 1 through 3 SHALL be modified.

#### Scenario: A rank filter moves a soft edge

- **WHEN** Median, Despeckle, or Dust & Scratches is applied to an RGBA buffer whose alpha is a soft-edged disc with a stray speck
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: Add Noise preserves alpha

- **WHEN** Add Noise is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit
