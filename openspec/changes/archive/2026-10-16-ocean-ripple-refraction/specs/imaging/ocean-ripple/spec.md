## ADDED Requirements

### Requirement: Ocean Ripple refracts through a seeded bumpy surface

`Filter::OceanRipple { size, magnitude, seed }` SHALL displace each destination pixel's source coordinate along the slope of a seeded, smoothly interpolated noise surface. The surface's cell size SHALL grow with `size`. The displacement SHALL grow faster than linearly with `magnitude`, and a `magnitude` of 0 SHALL leave the buffer unchanged. The same seed and parameters SHALL produce bit-identical output, and a different seed SHALL produce a different one. The model is tuned by eye against CS6 renders and is a behavioral approximation of a closed algorithm.

#### Scenario: Reach follows Magnitude

- **WHEN** Ocean Ripple is applied to a horizontal ramp rising 4 levels per pixel at (size, magnitude) = (14, 2), (9, 9) and (2, 12), with seed 1
- **THEN** the mean horizontal shift away from the clamped edges is under 1 px at (14, 2), over 2 px at (9, 9), and larger at (2, 12) than at (9, 9)

#### Scenario: Seeded and deterministic

- **WHEN** Ocean Ripple is applied twice with seed 7 and once with seed 8 at size 9 and magnitude 20
- **THEN** the two seed-7 results are bit-identical, the seed-8 result differs, and both differ from the input

### Requirement: Ocean Ripple defaults to CS6's settings

The `ocean-ripple` kind SHALL default to Ripple Size 9, Ripple Magnitude 9, and Seed 1, and its dialog SHALL open on those values.

#### Scenario: Default parameters

- **WHEN** the `ocean-ripple` kind is resolved with no parameters
- **THEN** it yields size 9, magnitude 9, and seed 1
