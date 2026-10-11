## ADDED Requirements

### Requirement: Shape blur

The system SHALL implement `Filter::ShapeBlur { shape: usize, radius: u32 }` as a convolution of each colour plane with the Custom Shape `shape` (an index into `CUSTOM_SHAPE_NAMES`) rasterized as antialiased coverage weights into a `(2r+1)²` square at its designed proportions, its longer side spanning the square. The kernel SHALL be rotated 180° before it is applied, so a single bright pixel spreads into the shape as drawn. Each output sample SHALL be `Σ w·v / Σ w`, rounded, with clamp-to-edge borders; alpha SHALL NOT be modified. `radius == 0` SHALL be a no-op. A shape index outside the catalogue or a radius above 1000 SHALL be refused with `FilterError::InvalidParams`. Oracle expectation: differential against ImageMagick `-define convolve:scale=! -morphology Convolve SxS:{weights}` with the same coverage weights and `S = 2*radius + 1`, exact.

#### Scenario: A flat region is unchanged

- **WHEN** Shape blur is applied to a uniform-color buffer with any shape and radius
- **THEN** every sample keeps its value

#### Scenario: A bright pixel takes the shape unflipped

- **WHEN** Shape blur with the right-pointing Arrow is applied to a black buffer holding one white pixel
- **THEN** the lit area is wide to the right of the pixel (the arrowhead) and a thin shaft to its left

#### Scenario: Radius zero is a no-op

- **WHEN** Shape blur is applied with radius 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Bad parameters are refused

- **WHEN** Shape blur is applied with a shape index past the catalogue or a radius of 1001
- **THEN** the call returns `FilterError::InvalidParams`

#### Scenario: Matches ImageMagick

- **WHEN** Shape blur with a given shape and radius is applied to the oracle image and ImageMagick applies `-morphology Convolve` with the same coverage weights, normalized
- **THEN** no sample differs
