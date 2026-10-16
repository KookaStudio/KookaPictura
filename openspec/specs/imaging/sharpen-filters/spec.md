# sharpen-filters Specification

## Purpose
Sharpen, Sharpen More, Sharpen Edges, and Unsharp Mask with clamped parameters and an oracle.

## Requirements

### Requirement: Sharpen filter application and error contract

The system SHALL provide `pictura_filters::apply(filter: &Filter, buf: &mut PixelBuffer) -> Result<(), FilterError>`. It SHALL operate in place on a planar 8-bit buffer whose `channels` is 3 (RGB) or 4 (RGBA), treating channels 1 through 3 as the color planes and channel 4, when present, as alpha. It SHALL transform every color sample or return an error; it MUST NOT partially apply a filter and then fail. Malformed buffers MUST return `FilterError` instead of panicking.

#### Scenario: Apply a sharpen to a 3-channel planar buffer

- **WHEN** `apply` receives a sharpen `Filter` variant and a 3-channel planar buffer
- **THEN** the R, G, and B planes are rewritten in place and `Ok(())` is returned

#### Scenario: Reject an unsupported channel count

- **WHEN** the buffer has a channel count other than 3 or 4
- **THEN** `apply` returns `FilterError::Unsupported` and leaves the buffer unchanged

#### Scenario: Malformed buffers error instead of panicking

- **WHEN** `apply` receives an empty buffer, or a buffer whose `data.len()` does not equal `width * height * channels`, or a buffer with zero width or height
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Sharpen and Sharpen More

The system SHALL implement `Filter::Sharpen` and `Filter::SharpenMore` with no parameters, as Unsharp Mask at fixed settings: radius `sharpen::SHARPEN_RADIUS` (3.0, σ 1), threshold 0, and amount `sharpen::SHARPEN_AMOUNT` (50 %) for `Sharpen` and `sharpen::SHARPEN_MORE_AMOUNT` (100 %) for `SharpenMore`. A uniform-color buffer SHALL be unchanged. `SharpenMore`'s deviation from the input SHALL be at least as large as `Sharpen`'s on the same edge image. Repeated application SHALL add contrast gradually rather than amplifying fine detail without bound. Oracle expectation: differential against ImageMagick `-unsharp 0x1+0.5+0` for `Sharpen` within tolerance 2 and `-unsharp 0x1+1+0` for `SharpenMore` within tolerance 3.

#### Scenario: The kernel response is a known value

- **WHEN** `Sharpen` and `UnsharpMask { amount: SHARPEN_AMOUNT, radius: SHARPEN_RADIUS, threshold: 0 }` are applied to the same edge buffer, and likewise `SharpenMore` with `SHARPEN_MORE_AMOUNT`
- **THEN** each pair of outputs is byte-identical, and a flat neighborhood is unchanged

#### Scenario: Sharpen More is stronger

- **WHEN** `Sharpen` and `SharpenMore` are each applied to the same edge image
- **THEN** the total deviation from the input is larger for `SharpenMore` than for `Sharpen`

#### Scenario: Repeated Sharpen stays bounded

- **WHEN** `Sharpen` is applied three times to a fine low-contrast pattern
- **THEN** the contrast grows but stays well short of the full tonal range

#### Scenario: Matches ImageMagick

- **WHEN** `Sharpen` is applied to the oracle image and ImageMagick applies `-unsharp 0x1+0.5+0`, and `SharpenMore` against `-unsharp 0x1+1+0`
- **THEN** no sample differs by more than 2 for `Sharpen` and 3 for `SharpenMore`

### Requirement: Sharpen Edges

The system SHALL implement `Filter::SharpenEdges` with no parameters. It SHALL measure edge strength as the Sobel gradient magnitude of the Rec.601 brightness of a σ 1 Gaussian blur of the image, and SHALL add `original − blurred` per colour channel, weighted by a smoothstep of that strength between fixed floor and ceiling levels (6 and 36). Below the floor the pixel SHALL be unchanged, so flat regions keep their values. Oracle expectation: no faithful ImageMagick equivalent exists (an edge-gated sharpen), so property tests cover the flat-region no-op and the edge contrast increase, and the divergence is documented.

#### Scenario: Flat regions are unchanged

- **WHEN** `SharpenEdges` is applied to a buffer with a flat area wider than the blur and Sobel reach
- **THEN** the flat-area samples beyond that reach are bit-exactly unchanged

#### Scenario: Edge contrast increases

- **WHEN** `SharpenEdges` is applied to a step edge
- **THEN** the contrast across the edge increases relative to the input

### Requirement: Unsharp Mask

The system SHALL implement `Filter::UnsharpMask { amount: f64, radius: f64, threshold: u8 }` as `out = original + (original − blurred) × amount`, where `blurred` is the shared Gaussian kernel from the blur family and the difference is gated by `threshold`. `amount` SHALL default to 100 and span `1.0..=500.0` percent; `radius` SHALL default to 1.0 and span `0.1..=250.0`; `threshold` SHALL default to 0 and span `0..=255`. The gate SHALL leave a pixel unchanged when its absolute difference from the blurred value is below `threshold`, and `threshold == 0` SHALL sharpen every pixel. `amount` outside `1.0..=500.0` and `radius <= 0` SHALL be rejected with `FilterError::InvalidParams`. Oracle expectation: differential against ImageMagick `-unsharp 0xRxA+T` within an absolute tolerance of 6 per sample (ImageMagick's internal blur differs slightly), with any radius-to-sigma mapping divergence documented.

#### Scenario: A uniform image is unchanged

- **WHEN** Unsharp Mask is applied to a uniform-color buffer
- **THEN** the buffer is bit-exactly unchanged within 1 LSB

#### Scenario: Overshoot grows with amount

- **WHEN** Unsharp Mask is applied to a step edge at increasing amounts with a fixed radius
- **THEN** the edge overshoot grows with the amount

#### Scenario: The threshold gates low-contrast pixels

- **WHEN** two adjacent pixels differ by less than the threshold
- **THEN** they are unchanged, and when they differ by the threshold or more they are sharpened

#### Scenario: Out-of-range amount or radius is rejected

- **WHEN** Unsharp Mask is applied with amount 0 or radius 0
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

#### Scenario: Matches ImageMagick

- **WHEN** Unsharp Mask with a given amount, radius, and threshold is applied to the oracle image and ImageMagick applies the matching `-unsharp 0xRxA+T`
- **THEN** no sample differs by more than 6, or the divergence is recorded in the no-equivalent documentation

### Requirement: Sharpen parameter validation and clamping

Each parameterized sharpen filter SHALL reject out-of-range or non-finite parameters with `FilterError::InvalidParams` instead of panicking. Sharpen output SHALL be clamped only at the final 8-bit store; it MUST NOT wrap around on overflow or underflow. Each filter SHALL transform every color sample or return an error, never a partial result.

#### Scenario: Out-of-range Unsharp Mask parameters return an error

- **WHEN** Unsharp Mask is applied with amount 0, amount 501, radius 0, or radius 251
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic

#### Scenario: Overflow clamps instead of wrapping

- **WHEN** sharpening pushes a sample above 255 or below 0
- **THEN** the output clamps to 255 or 0 and does not wrap to the opposite extreme

### Requirement: Clamp-to-edge borders and tiny images for sharpen

Every sharpen kernel SHALL sample with clamp-to-edge at the image borders and SHALL clamp its kernel support to the available source. A 1×1 image and a 1-pixel-wide or 1-pixel-tall image MUST NOT panic and MUST return either a result or a typed error.

#### Scenario: A tiny image does not panic

- **WHEN** every sharpen variant is applied to a 1×1 buffer and to a 1-pixel-wide buffer
- **THEN** no variant panics and each returns `Ok(())` or a `FilterError`

#### Scenario: Border pixels clamp to the edge

- **WHEN** a sharpen filter is applied to a small buffer and a pixel at the border is inspected
- **THEN** the result is consistent with repeating the edge sample rather than reading out of bounds

### Requirement: Deterministic sharpen output

Sharpen filters SHALL be deterministic. The same filter applied to equal input buffers SHALL produce bit-identical output every run, with no random, time, or thread-order dependence.

#### Scenario: Repeated runs match

- **WHEN** each sharpen variant is applied to two clones of one buffer
- **THEN** the two output buffers are bit-identical

### Requirement: Sharpen ImageMagick oracle and no-equivalent classification

The system SHALL ship `scripts/filter_oracle.py` and `crates/pictura-filters/tests/oracle.rs`. The mapping table SHALL have exactly one row per `Filter` variant. Unsharp Mask, Sharpen, and Sharpen More SHALL be diffed against `-unsharp` within their stated tolerances. SharpenEdges SHALL be classified as having no faithful ImageMagick operator and SHALL use tolerance 0 with known-value or property tests, including the USM-not-an-edge-detector behavior. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table covers every sharpen variant

- **WHEN** the oracle tests run
- **THEN** every sharpen variant has a table row, each no-equivalent row has tolerance 0, and each row carries a non-empty note

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes

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

### Requirement: Smart Sharpen Shadow/Highlight tonal fade

`shadow` and `highlight` SHALL each be a `TonalFade { amount: u8, width: u8, radius: u32 }` that scales the sharpening contribution within the shadow (or
highlight) tonal band, in the spirit of Blend If. `TonalFade::default()` SHALL be
`{ amount: 0, width: 50, radius: 1 }`. `amount` and `width` SHALL span `0..=100`
and `radius` SHALL span `1..=100`; a value outside those ranges SHALL be rejected
with `FilterError::InvalidParams` before any mutation. `amount` 0 SHALL be a
no-op. A high `shadow.amount` SHALL reduce the change in dark tones relative to a
low one, and `highlight.amount` SHALL act analogously in light tones.

#### Scenario: Zero fade amount is a no-op

- **WHEN** Smart Sharpen is applied with `shadow.amount: 0` and `highlight.amount: 0` and again with the default `TonalFade` values
- **THEN** the two outputs are bit-identical

#### Scenario: Shadow fade dampens dark-tone sharpening

- **WHEN** Smart Sharpen is applied to a dark edge with a high `shadow.amount` and with a low one
- **THEN** the total change in the dark tones is smaller under the high `shadow.amount`

#### Scenario: Highlight fade dampens light-tone sharpening

- **WHEN** Smart Sharpen is applied to a bright edge with a high `highlight.amount` and with a low one
- **THEN** the total change in the light tones is smaller under the high `highlight.amount`

#### Scenario: Out-of-range tonal parameters are rejected untouched

- **WHEN** Smart Sharpen is applied with `shadow.amount` or `shadow.width` above 100, `shadow.radius` below 1 or above 100, or the analogous `highlight` values out of range
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state

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

### Requirement: Sharpen alpha follows the kernel

For 4-channel buffers, every sharpen filter SHALL run its kernel over channel 4 as it does over the colour planes, so the layer edge sharpens with the colour, and a channel 4 that is 255 everywhere SHALL stay 255. Sharpen Edges SHALL sharpen channel 4 only where its colour edge mask does.

#### Scenario: A sharpen moves a soft edge

- **WHEN** a sharpen variant is applied to an RGBA buffer whose alpha is a soft-edged disc
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: An opaque buffer stays opaque

- **WHEN** a sharpen variant is applied to an RGBA buffer whose alpha is 255 everywhere
- **THEN** every alpha sample is still 255
