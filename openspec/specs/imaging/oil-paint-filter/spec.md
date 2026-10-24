# oil-paint-filter Specification

## Purpose
The Oil Paint filter with its error contract, alpha preservation, parameter validation, and determinism.

## Requirements

### Requirement: Oil Paint application and error contract

The system SHALL implement the CS6 Oil Paint filter as a `Filter::OilPaint`
variant and apply it through the shared `pictura_filters::apply(filter, buf)`
entry point. `apply` SHALL transform every color sample of a planar 8-bit buffer
whose channel count is 3 (RGB) or 4 (RGBA) and return `Ok(())`, or return a
`FilterError` without partially applying; malformed buffers MUST error instead
of panicking.

#### Scenario: Apply Oil Paint to a color buffer

- **WHEN** `apply` receives `Filter::OilPaint` and a 3- or 4-channel planar buffer
- **THEN** the color planes are rewritten in place and `Ok(())` is returned

#### Scenario: Malformed buffers error

- **WHEN** the buffer is empty, has zero width or height, or an inconsistent length
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Alpha channel preservation

For 4-channel buffers, Oil Paint SHALL leave channel 4 bit-identical.

#### Scenario: Oil Paint preserves alpha

- **WHEN** `Filter::OilPaint` is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Parameter validation

The system SHALL validate every Oil Paint parameter before writing any sample and
SHALL reject out-of-range or non-finite values with `FilterError::InvalidParams`
without panicking. The accepted ranges SHALL be: `stylization 0.0..=10.0`,
`cleanliness 0.0..=10.0`, `scale 0.0..=10.0`, `bristle_detail 0.0..=10.0`,
`angular_direction 0.0..=360.0`, `shine 0.0..=10.0`.

#### Scenario: Out-of-range parameters are rejected

- **WHEN** Oil Paint is applied with a parameter outside its range
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: Non-finite parameters are rejected

- **WHEN** Oil Paint is applied with a `NaN` or infinite parameter
- **THEN** `apply` returns `FilterError::InvalidParams` without panicking

#### Scenario: Boundary values are accepted

- **WHEN** Oil Paint is applied at the minimum and maximum of every range
- **THEN** each call returns `Ok(())` without a panic

### Requirement: Determinism

Oil Paint SHALL be deterministic: the same input and parameters SHALL produce
bit-identical output on repeated applies. Because it introduces no randomness it
SHALL NOT take a `seed` parameter.

#### Scenario: Repeat applies match exactly

- **WHEN** Oil Paint is applied twice with identical parameters
- **THEN** the two outputs are bit-identical

### Requirement: Oil Paint

The system SHALL implement `Filter::OilPaint { stylization, cleanliness, scale, bristle_detail, angular_direction, shine }` as a CPU behavioural model of the CS6 GPU effect — a deliberate, documented non-parity divergence because CS6 hard-requires a supported GPU. It SHALL apply edge-aware directional smoothing followed by relief/lighting shading, where `stylization` controls stroke smoothness, `cleanliness` stroke length, `scale` apparent paint thickness, `bristle_detail` groove contrast, `angular_direction` the light azimuth, and `shine` the specular strength.

#### Scenario: Non-empty effect with valid range

- **WHEN** Oil Paint is applied to a textured test image
- **THEN** the output differs from the input and every sample stays in `0..=255`

#### Scenario: Stylization and scale change the result

- **WHEN** Oil Paint is applied at `stylization` 0 and 10 and separately at `scale` 0 and 10
- **THEN** each pair of outputs differs

### Requirement: Oil Paint dialog exposes the engine parameter range

The Oil Paint dialog SHALL expose Shine as a `0..=10` control opening on
`5.5`, matching the engine's accepted `0.0..=10.0` range and the `shine / 10`
normalization both the CPU and GPU kernels apply. The slider SHALL reach the
maximum without clipping below it.

#### Scenario: Shine reaches the engine range [op_shine_range]

- **WHEN** the Oil Paint dialog opens and Shine is dragged to its maximum
- **THEN** the control's range is 0 through 10, its opening value is 5.5, and the committed parameter is 10
