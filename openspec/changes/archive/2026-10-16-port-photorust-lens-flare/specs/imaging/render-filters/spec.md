## MODIFIED Requirements

### Requirement: Lens Flare renders a deterministic additive light pass

The system SHALL provide `Filter::LensFlare { brightness: f64, center: (f64, f64), lens: LensType }` with `LensType` values `Zoom`, `Prime35`, `Prime105`, and `MoviePrime`. It SHALL add light onto the existing RGB: a bright core at `center` (fractions of the width and height, clamped into 0..=1), a glow and halo around it, rays (or, for `MoviePrime`, a horizontal streak), and tinted ghosts strung along the line from the flare through the middle of the frame. Results SHALL be clamped to 0..=255 and alpha SHALL be left untouched. Every size SHALL be a fraction of the image's half-diagonal, so the same parameters on a scaled copy produce a scaled copy of the flare. `brightness` outside 10..=300 SHALL be rejected with `FilterError::InvalidParams` before any mutation. The filter SHALL be fully deterministic (no seed). The model is photorust's, tuned by eye against CS6, and is a behavioral approximation of a closed algorithm.

#### Scenario: Brightness is monotone at the core

- **WHEN** `LensFlare` runs on a dark frame at centre (0.5, 0.5), and at brightness 25, 100, and 300 at centre (0.2, 0.2)
- **THEN** the pixel under the centre is blown out to 255 while an off-axis corner stays dark, and an off-axis sample rises strictly with brightness

#### Scenario: Lens type and center change the geometry

- **WHEN** `LensFlare` runs at centre (0.25, 0.25), near the left edge on black, and with each of the four `LensType` values
- **THEN** the placed centre is brighter than an off-axis point by more than 100 levels, the right half along the ghost axis carries more than twice the light of a row well off it, the four lens outputs all differ, each output is bit-identical across repeated runs, and an off-frame centre (−3, 5) renders identically to (0, 1)

#### Scenario: A flare is the same picture at any size

- **WHEN** the same `LensFlare` runs on a 600×400 frame and a 150×100 frame
- **THEN** outside blown-out blocks, each small-frame pixel is within 20 levels of the mean of the 4×4 large-frame block under it

#### Scenario: Out-of-range brightness is rejected untouched

- **WHEN** `LensFlare` is called with brightness below 10, above 300, or NaN
- **THEN** it returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state
