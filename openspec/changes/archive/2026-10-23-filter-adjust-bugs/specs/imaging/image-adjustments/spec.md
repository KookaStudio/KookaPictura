## MODIFIED Requirements

### Requirement: Auto adjustments

The system SHALL implement `Adjustment::Auto(AutoKind::Tone | AutoKind::Contrast | AutoKind::Color)`. Tone SHALL stretch each color channel independently between percentile bounds that discard 0.1 percent of the population at each extreme. Contrast SHALL stretch all three channels by one joint pair of bounds that discards 0.5 percent, preserving channel ratios so no cast is introduced. Color SHALL apply the per-channel stretch with 0.5 percent clipping, then scale each channel's midtone gamma so the mean near-neutral midtone converges to the shared mean. Bounds SHALL be derived from the histogram of the population being adjusted. When the adjustment is applied destructively through an active selection, the histogram, percentile bounds, and midtone means SHALL be computed from the selected pixels only (coverage greater than zero), while the result SHALL still be written through that selection. Oracle expectation: Adobe's Auto Color Correction solver is closed, so behavior is covered by property tests (per-channel stretch, joint no-cast, midtone neutralization, selection-only statistics).

#### Scenario: Auto Tone stretches the range

- **WHEN** Auto Tone is applied to a ramp whose values span 60 through 200
- **THEN** the dark end maps near 0 and the light end maps near 255

#### Scenario: Auto Contrast does not introduce a cast

- **WHEN** Auto Contrast is applied to a neutral grey ramp
- **THEN** every output pixel keeps R equal to G equal to B

#### Scenario: Auto Color pulls channels toward neutral

- **WHEN** Auto Color is applied to a ramp with a fixed channel offset
- **THEN** the per-channel sums move closer together than before

#### Scenario: Auto uses the selection histogram [ia_auto_selection]

- **WHEN** a two-ramp image has its left half selected and Auto Tone, Auto Contrast, or Auto Color is applied
- **THEN** the selected ramp is stretched using only the selected pixels' histogram and the unselected ramp is unchanged

### Requirement: Vibrance adjustment

The system SHALL implement `Adjustment::Vibrance(VibranceParams { vibrance, saturation })`. Both values SHALL default to 0 and span `-100..=100`. It SHALL convert each pixel to HSL, apply the saturation scale `S * (1 + saturation/100)`, and add a vibrance delta that falls off as `1 - S` and is damped in the skin-tone hue band, then clamp saturation to `[0, 1]`. A pixel whose input saturation is 0 SHALL remain achromatic: the vibrance delta SHALL be skipped at `S == 0`, and the `1 - S` falloff SHALL be unchanged. Saturation -100 SHALL produce a neutral grey. Oracle expectation: no faithful ImageMagick operator exists, so property tests cover the identity, the diminishing boost, neutral-grey preservation, and saturation -100.

#### Scenario: Neutral parameters are identity

- **WHEN** Vibrance is applied with vibrance 0 and saturation 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Saturation minus 100 is grey

- **WHEN** Vibrance is applied with saturation -100
- **THEN** all three color channels of the pixel are equal

#### Scenario: Neutral gray stays neutral [ia_vibrance_neutral]

- **WHEN** Vibrance is applied with vibrance 100 and saturation 0 to a 50% neutral-grey pixel
- **THEN** the output equals the input grey and has no hue
