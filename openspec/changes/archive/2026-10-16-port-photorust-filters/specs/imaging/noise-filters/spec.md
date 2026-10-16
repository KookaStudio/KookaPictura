## MODIFIED Requirements

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
