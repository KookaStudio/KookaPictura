## ADDED Requirements

### Requirement: Iris blur (elliptical focus variable blur)

The system SHALL implement `Filter::IrisBlur { pins: Vec<IrisPin>, bokeh,
noise }` with `IrisPin { x, y, radius_x, radius_y, angle, squareness,
feather, blur }` at the crate root. `x` and `y` SHALL be fractions of the
buffer (outside allowed); `radius_x` and `radius_y` SHALL be the outline's
semi-axes as fractions of the buffer width in
`0.0001..=MAX_IRIS_RADIUS` (10); `angle` SHALL rotate the `radius_x` axis
clockwise in degrees; `squareness` (`0..=100`) SHALL select the
superellipse exponent `p = 2 / (1 − 0.009·squareness)`; `feather`
(`0..=1`) SHALL be the sharp core as a fraction of the outline; and `blur`
SHALL be the exterior radius in buffer pixels in `0..=MAX_FIELD_RADIUS`. At
most `MAX_FIELD_PINS` (16) pins SHALL be accepted. A pin's falloff SHALL be
0 where the normalized superellipse distance `ρ ≤ feather`, 1 where
`ρ ≥ 1`, and a smoothstep between; the per-pixel radius SHALL be the product
of the pins' falloffs times the inverse-square-distance blend of their
amounts. The kernel SHALL run the shared Blur Gallery variable blur and its
Blur Effects / Noise passes weighted by the normalized radius, SHALL leave a
zero-radius pixel unchanged, SHALL write the colour planes only, SHALL treat
an empty pin list as a no-op, and SHALL reject non-finite or out-of-range
values with `FilterError::InvalidParams` before writing any pixel. Equal
inputs SHALL produce bit-identical output. Oracle expectation: no faithful
ImageMagick operator exists; the mapping row is no-equivalent with tolerance
0 and a non-empty note.

#### Scenario: The sharp core is untouched and the exterior blurs

- **WHEN** Iris Blur is applied with one circular pin to a noisy picture
- **THEN** every pixel inside the feather core is bit-identical and pixels
  outside the outline change

#### Scenario: The fade band ramps

- **WHEN** an impulse is blurred inside the core, in the fade band, and
  outside the outline
- **THEN** the core impulse does not spread and the spread grows from the
  fade band to the exterior

#### Scenario: Rotation, squareness, and feather shape the falloff

- **WHEN** a wide pin is rotated 90°, a pin's squareness is raised to 100,
  or its feather is widened
- **THEN** the sharp and blurred axes swap, a diagonal point outside the
  ellipse falls inside the near-square, and more of the radius stays sharp

#### Scenario: Two pins keep two sharp islands

- **WHEN** Iris Blur is applied with two separate pins
- **THEN** both centres stay sharp and the region between them blurs

#### Scenario: Invalid parameters are rejected without mutation

- **WHEN** a pin has a non-finite value, a zero or over-10 radius, a
  squareness above 100, a feather outside 0..1, a blur outside 0..500, or
  more than 16 pins are given
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is
  unchanged
