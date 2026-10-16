## ADDED Requirements

### Requirement: Lens blur (depth-map aperture convolution)

The system SHALL implement `Filter::LensBlur { depth: Option<Arc<DepthMap>>, focal: u8, invert: bool, shape: IrisShape, radius: u32, curvature: u8, rotation: f64, specular_brightness: u8, specular_threshold: u8, noise_amount: u8, noise_distribution: NoiseDistribution, monochromatic: bool, faster: bool, seed: u64 }`. `DepthMap` SHALL be a one-plane 8-bit map (0 near/front, 255 far) and `IrisShape` SHALL be one of `Triangle`, `Square`, `Pentagon`, `Hexagon`, `Heptagon`, `Octagon` (3 through 8 blades). Each pixel's circle-of-confusion radius SHALL be `radius × |d − focal| / 255`, where `d` is the depth sample after `invert`; a `None` map SHALL act as a uniform depth of 255. The kernel SHALL convolve the color planes with an antialiased aperture mask of the chosen iris shape, rounded toward a circle by `curvature` (0 = polygon, 100 = circle) and rotated by `rotation` degrees, so an out-of-focus point light spreads into the aperture shape. Source pixels at or above `specular_threshold` SHALL contribute an additive highlight weighted by `specular_brightness`, so an out-of-focus bright spot blooms rather than dims. The kernel SHALL re-inject noise of `noise_amount` in `noise_distribution` (`Uniform`/`Gaussian`), shared across channels when `monochromatic`, seeded by `seed` so equal inputs and seeds produce bit-identical output. A depth map whose dimensions differ from the buffer SHALL be sampled with a clamped bilinear stretch. `radius`, `curvature`, `specular_brightness`, and `noise_amount` SHALL span `0..=100`; `rotation` SHALL be finite; a value outside those ranges SHALL be rejected with `FilterError::InvalidParams` before any pixel is written. Alpha SHALL be left to the render layer's alpha pass. Oracle expectation: no faithful ImageMagick operator exists (ImageMagick has no depth-aware aperture blur), so property and known-value tests cover the behavior and the mapping row is no-equivalent with tolerance 0 and a non-empty note.

#### Scenario: In-focus depth stays sharp

- **WHEN** Lens Blur is applied with a depth map equal to the focal distance everywhere, a non-zero radius, and no noise
- **THEN** every output sample equals the input within 1 LSB

#### Scenario: A depth ramp blurs with distance

- **WHEN** Lens Blur with focal 0 is applied to a buffer whose depth map is black at the bottom and white at the top
- **THEN** the bottom edge stays sharp and the top edge's transition is wider than the input's

#### Scenario: A point light repeats the iris shape

- **WHEN** an out-of-focus impulse is blurred with a triangle iris and low curvature, and again with the iris rotated
- **THEN** the spread support follows the polygon rather than a disc, and rotating the iris rotates the support

#### Scenario: Specular highlights bloom

- **WHEN** an out-of-focus bright spot at or above `specular_threshold` is blurred with `specular_brightness` at 100 and again at 0
- **THEN** the spot is brighter with the boost than without it

#### Scenario: Noise is seeded and deterministic

- **WHEN** the same Lens Blur with a non-zero noise amount is applied twice, and again with a different seed
- **THEN** the equal seeds are bit-identical, the different seed differs, monochromatic noise leaves the three channels equal, and alpha is bit-identical throughout

#### Scenario: A differently sized depth map is stretched

- **WHEN** a 1×2 black/white depth map is stretched over a taller buffer
- **THEN** the blur increases smoothly along the buffer instead of in two hard steps

#### Scenario: Invalid parameters are rejected without mutation

- **WHEN** Lens Blur is applied with radius 101 or a non-finite rotation
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is bit-exactly unchanged

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Lens Blur has no ImageMagick operator, tolerance 0, and a non-empty no-equivalent note
