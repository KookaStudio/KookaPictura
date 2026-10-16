## ADDED Requirements

### Requirement: Diffuse Glow blooms highlights toward white

The system SHALL provide `Filter::DiffuseGlow { graininess: u32, glow_amount: u32, clear_amount: u32, seed: u64 }`. It SHALL mix every colour channel of each pixel toward white by one fraction per pixel. The fraction SHALL rise with the pixel's blurred luminance, so that light passes spill past their edges as a halo. Raising `glow_amount` SHALL lower the threshold, steepen the ramp, and widen the halo. A `glow_amount` of 0 SHALL add no glow. Lowering `clear_amount` SHALL lift the whole picture toward white. `graininess` SHALL speckle the fraction with seeded per-pixel noise. The filter SHALL never darken a channel and SHALL leave alpha untouched. Values of `graininess` above 10, or of `glow_amount` or `clear_amount` above 20, SHALL be rejected with `FilterError::InvalidParams` before any mutation. The model is fitted to CS6 renders and is a behavioral approximation of a closed algorithm.

#### Scenario: No glow, full clear and no grain is the identity

- **WHEN** `DiffuseGlow` is applied with graininess 0, glow 0 and clear 20
- **THEN** the buffer is bit-identical to its prior state

#### Scenario: Highlights bloom and spill

- **WHEN** `DiffuseGlow` is applied with graininess 0, glow 10 and clear 15 to a dark (40) half beside a light (220) half
- **THEN** the light half rises above 245, dark pixels far from the edge stay below 60, and dark pixels beside the edge are more than 20 levels lighter than the far ones

#### Scenario: Clear Amount thins the veil

- **WHEN** `DiffuseGlow` is applied to a flat dark area (red 40) at glow 2 with clear 2 and clear 15, and at glow 1 with clear 6
- **THEN** the clear-2 result is lifted above 150, the clear-15 result stays below 60, and the clear-6 result lies between 55 and 80

#### Scenario: Grain is seeded and never darkens

- **WHEN** `DiffuseGlow` is applied twice with graininess 10 and the same seed, and once with another seed
- **THEN** the same-seed results are bit-identical, the other seed differs, no channel is darker than the source, alpha is unchanged, and a flat area spreads by more than 20 levels

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** `DiffuseGlow` is called with graininess 11, glow amount 21, or clear amount 21
- **THEN** it returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

### Requirement: Diffuse Glow is a Distort Filter Gallery entry

The app SHALL map the `diffuse-glow` kind to `Filter::DiffuseGlow` from four parameters: Graininess (0–10, default 6), Glow Amount (0–20, default 10), Clear Amount (0–20, default 15), and Seed (0–999, default 1). The Filter Gallery SHALL list Diffuse Glow first in its Distort category, and `Filter ▸ Distort ▸ Diffuse Glow` SHALL open its dialog.

#### Scenario: The gallery lists Diffuse Glow under Distort

- **WHEN** the Filter Gallery builds its categories
- **THEN** the Distort category's first entry is the `diffuse-glow` kind

#### Scenario: Default parameters

- **WHEN** the `diffuse-glow` kind is resolved with no parameters
- **THEN** it yields graininess 6, glow amount 10, clear amount 15, and seed 1
