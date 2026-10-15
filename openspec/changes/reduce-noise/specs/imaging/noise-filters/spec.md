## ADDED Requirements

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
