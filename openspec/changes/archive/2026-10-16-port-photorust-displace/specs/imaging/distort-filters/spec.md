## ADDED Requirements

### Requirement: Displace

The system SHALL implement `Filter::Displace { map: Arc<DisplaceMap>, horizontal_scale: f64, vertical_scale: f64, fit: DisplaceFit, wrap: bool }`. A `DisplaceMap` SHALL hold two 8-bit planes of equal size, horizontal and vertical. Each destination pixel SHALL read the source, bilinearly, at an offset of `(h − 128) × horizontal_scale / 100` pixels across and `(v − 128) × vertical_scale / 100` pixels down, where `h` and `v` are the map's samples for that pixel, so a sample of 128 moves nothing and 100% reaches 127 px at 255 and 128 px at 0. `DisplaceFit::StretchToFit` SHALL scale the map over the buffer, sampling it bilinearly, and `DisplaceFit::Tile` SHALL repeat it at its own size from the buffer's top-left corner. `wrap` SHALL read positions past an edge from the opposite edge (Wrap Around); otherwise it SHALL repeat the nearest edge pixel (Repeat Edge Pixels). A map whose samples are all 128, or scales that are both 0, SHALL leave the buffer bit-identical. Scales outside `-999.0..=999.0` SHALL return `FilterError::InvalidParams`. The alpha plane SHALL be displaced the same way as colour. Oracle expectation: ImageMagick's `-compose displace` with `compose:args` of `1.275 × scale` is a faithful operator, and Repeat Edge Pixels SHALL match it within 8 levels on the oracle test image.

#### Scenario: A neutral map is a no-op

- **WHEN** Displace is applied with a map of all 128, at any scale, to a buffer with translucent pixels
- **THEN** the output equals the input exactly

#### Scenario: Full deflection at 100% reaches about 128 pixels

- **WHEN** Displace is applied at 100% with a horizontal plane of 255 to a picture with a vertical line
- **THEN** the line moves 127 pixels to the left; with a plane of 0 at 50% it moves 64 pixels to the right

#### Scenario: The two planes drive the two axes

- **WHEN** Displace is applied once with only the horizontal plane away from 128 and once with only the vertical plane away from 128
- **THEN** the first shifts content only sideways, the second only up or down, and the two results differ

#### Scenario: One plane twice shifts along the diagonal

- **WHEN** both planes hold the same samples and the scales differ
- **THEN** each pixel moves by the same deflection times each axis's scale

#### Scenario: Stretch and tile place the map differently

- **WHEN** a two-pixel map is applied once with `StretchToFit` and once with `Tile`
- **THEN** stretching spreads each map sample over half the buffer, and tiling alternates them per column

#### Scenario: The undefined-areas modes differ

- **WHEN** Displace pulls content past the right edge with `wrap` true and with `wrap` false
- **THEN** Wrap Around brings in the left edge's pixels and Repeat Edge Pixels repeats the right edge's

#### Scenario: The ImageMagick oracle agrees

- **WHEN** Displace with Repeat Edge Pixels and the oracle's ramp map is diffed against `MAP -compose displace -define compose:args=XxY -composite` with `X = 1.275 × scale`
- **THEN** every sample is within 8 levels at scales 4×4 and 6×−5
