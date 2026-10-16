## ADDED Requirements

### Requirement: Smart Sharpen deconvolution

The system SHALL implement `Filter::SmartSharpen { amount: f64, radius: f64, reduce_noise: f64, remove: SharpenRemove, angle: f64, more_accurate: bool, shadow: TonalFade, highlight: TonalFade }`
with `SharpenRemove::{GaussianBlur, LensBlur, MotionBlur}` and
`TonalFade { amount: u8, width: u8, radius: u32 }` as a deconvolution of the
blur model `H = 0.62·core + 0.38·halo`. `core` SHALL be the separable 3-tap
`[0.17, 0.66, 0.17]`, inverted exactly. `halo` SHALL follow `remove` at
`s = 0.8·radius`: `GaussianBlur` a Gaussian of σ `s`, `LensBlur` a disc of
radius `s` whose rows end in fractional-weight pixels, and `MotionBlur` an
equal-weight bilinear line of length `2s` along `angle`. The motion line SHALL
follow Photoshop's counter-clockwise screen-angle convention (`+angle` runs
toward the upper right in this y-down buffer). The halo factor SHALL be solved
by relaxed Van Cittert iteration starting from the original, three iterations
by default. With the detail `d = deconvolved − original`, the output SHALL be
`out = original + d × amount/100 × gain`, where `gain = d²/(d² + floor²)` and
`floor = reduce_noise/100 × 10`; `gain` SHALL be exactly 1 when
`reduce_noise` is 0. `amount` SHALL span `1.0..=500.0` percent, where 0 is
rejected rather than a no-op; `radius` SHALL be finite and within
`0.1..=64.0`; `reduce_noise` SHALL span `0..=100`; and `angle` SHALL be finite
and within `-360..=360`. A value outside those ranges SHALL be rejected with
`FilterError::InvalidParams` before any mutation. Alpha SHALL be left
bit-identical and the output SHALL be deterministic. The model constants are
fitted to Photoshop screenshots and are a behavioral approximation of a
closed algorithm. Oracle expectation: ImageMagick has no deconvolution
operator, so every `remove` is classified no-equivalent with tolerance 0 and
covered by unit tests of the core inverse, the halo solve, and the kernels.

#### Scenario: A uniform image is unchanged

- **WHEN** Smart Sharpen is applied to a uniform-color buffer with any valid amount, radius, and `remove`
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Amount 0 is rejected

- **WHEN** Smart Sharpen is applied with `amount: 0.0`
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

#### Scenario: Fine detail gains more than under Unsharp Mask

- **WHEN** Smart Sharpen with `remove: GaussianBlur` and Unsharp Mask with threshold 0 are applied at amount 100 and radius 1 to a one-pixel alternation
- **THEN** the Smart Sharpen output range is more than twice the Unsharp Mask output range

#### Scenario: The core inverse undoes the core

- **WHEN** a signal blurred by the 3-tap core is passed through the core inverse
- **THEN** the original signal is recovered within 0.001

#### Scenario: The halo solve converges

- **WHEN** the halo factor is solved with the More Accurate iteration count for each `remove`
- **THEN** `0.62·x + 0.38·halo(x)` matches the original within 0.5 at every pixel

#### Scenario: Radius widens the halo

- **WHEN** Smart Sharpen with `remove: GaussianBlur` is applied to a step edge at radius 1 and at radius 6
- **THEN** the radius 6 run changes more pixels around the edge

#### Scenario: Lens Blur and Motion Blur change the result

- **WHEN** Smart Sharpen with `remove: LensBlur` or `remove: MotionBlur` is applied to an edge image
- **THEN** the output differs from the `GaussianBlur` run at the same amount and radius

#### Scenario: Reduce Noise holds back low-contrast detail

- **WHEN** Smart Sharpen is applied to a low-contrast edge with `reduce_noise: 0.0` and with `reduce_noise: 100.0`
- **THEN** the total change from the input is smaller in the `reduce_noise: 100.0` run

#### Scenario: Motion Blur follows Photoshop's angle

- **WHEN** Smart Sharpen with `remove: MotionBlur` and `angle: 45` is applied to a single bright pixel on gray
- **THEN** the change runs along the lower-left to upper-right diagonal and exceeds the change on the other diagonal

#### Scenario: Out-of-range parameters are rejected untouched

- **WHEN** Smart Sharpen is applied with amount below 1.0 or above 500.0, radius below 0.1 or above 64.0, `reduce_noise` above 100, or a non-finite angle
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

#### Scenario: Output is deterministic and alpha is preserved

- **WHEN** Smart Sharpen is applied twice to equal RGBA buffers
- **THEN** the two outputs are bit-identical and the alpha plane equals the input alpha plane bit for bit

## MODIFIED Requirements

### Requirement: Smart Sharpen More Accurate

`more_accurate` SHALL run the halo solve to convergence, eight iterations
instead of three. When `more_accurate` is true, the output SHALL differ from
the same call with `more_accurate` false for every `remove` on a detail
image, and the result SHALL be deterministic and leave alpha bit-identical.

#### Scenario: More Accurate changes every removal path

- **WHEN** Smart Sharpen with `remove: GaussianBlur`, `remove: LensBlur`, or `remove: MotionBlur` is applied to a detail image with `more_accurate: true` and with `more_accurate: false`
- **THEN** the `true` output differs from the `false` output in every case

#### Scenario: More Accurate is deterministic and preserves alpha

- **WHEN** Smart Sharpen with `more_accurate: true` is applied twice to equal RGBA buffers
- **THEN** the two outputs are bit-identical and the alpha plane equals the input alpha plane bit for bit

## REMOVED Requirements

### Requirement: Smart Sharpen

**Reason**: Smart Sharpen is no longer an unsharp mask against a swappable
blur. Its Gaussian path is no longer byte-identical to Unsharp Mask, and the
Lens and Motion paths deconvolve their blur instead of subtracting it.

**Migration**: Read "Smart Sharpen deconvolution", which keeps the `Filter`
fields, parameter ranges, rejection, determinism, alpha, Reduce Noise, and
Motion angle contracts. The "Gaussian Blur matches Unsharp Mask" scenario
becomes "Fine detail gains more than under Unsharp Mask".
