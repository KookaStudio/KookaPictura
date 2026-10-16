## MODIFIED Requirements

### Requirement: Fibers renders seeded directional fibers

The system SHALL provide `Filter::Fibers { variance: f64, strength: f64, color_a: [u8; 3], color_b: [u8; 3], seed: u64 }`. It SHALL replace the layer's RGB with deterministic streaks elongated along the vertical axis, blended from `color_b` (background) to `color_a` (foreground), leaving alpha untouched. The streaks SHALL layer smooth broad clumps under hard-edged hairs one or two pixels wide. `variance` SHALL shorten the streaks and shift the balance towards hairs, and `strength` SHALL lengthen them. A `variance` of 0 SHALL fill every pixel with the even blend of the two colours. Values of `variance` outside 0..=64 or `strength` outside 1..=64 SHALL be rejected with `FilterError::InvalidParams` before any mutation. The model is photorust's, tuned by eye against CS6, and is a behavioral approximation of a closed algorithm.

#### Scenario: Fibers run down the picture

- **WHEN** `Fibers` is applied at variance 12 and strength 4
- **THEN** the summed tone change between horizontal neighbours exceeds four times the summed change between vertical neighbours, and the output reaches within 16 levels of both colours

#### Scenario: Fibers parameters shape the field

- **WHEN** `Fibers` is applied at variance 8 and at variance 48 with the same seed and strength, and at strength 1 and strength 48 with the same seed and variance
- **THEN** each output is bit-identical across repeated runs at its own settings, all RGB values lie within the colour range, the variance-48 run changes more than twice as much down each column as the variance-8 run, and the strength-48 run changes less down a column than the strength-1 run

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** `Fibers` is called with `variance` below 0 or above 64, or `strength` below 1 or above 64, or either is NaN
- **THEN** it returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state
