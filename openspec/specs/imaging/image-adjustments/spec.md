# image-adjustments Specification

## Purpose
The destructive adjustment contract and the CS6 adjustment kinds, with determinism and an oracle.

## Requirements

### Requirement: Destructive adjustment application contract

The system SHALL provide `pictura_adjust::apply(adjustment: &Adjustment, buf: &mut PixelBuffer) -> Result<(), AdjustError>`. It SHALL operate in place on a planar 8-bit buffer whose `channels` is 3 (RGB) or 4 (RGBA), treating channels 1, 2, and 3 as the composite color planes and channel 4, when present, as alpha. It SHALL transform every color sample or return an error; it MUST NOT partially apply an adjustment and then fail.

#### Scenario: Apply to a 3-channel planar buffer

- **WHEN** `apply` receives any supported `Adjustment` and a 3-channel planar buffer
- **THEN** the R, G, and B planes are rewritten in place and `Ok(())` is returned

#### Scenario: Reject an unsupported channel count

- **WHEN** the buffer has a channel count other than 3 or 4
- **THEN** `apply` returns `AdjustError::Unsupported` and leaves the buffer unchanged

### Requirement: Alpha channel preservation

For 4-channel buffers, `apply` SHALL leave channel 4 bit-identical whenever it
succeeds, and SHALL leave the whole buffer untouched when it refuses a generative
variant, so channel 4 is never modified by any adjustment. Only channels 1
through 3 SHALL be modified.

#### Scenario: All adjustment variants preserve alpha

- **WHEN** each of the 20 adjustment variants — the seventeen destructive variants and the refused fills `SolidFill`, `GradientFill`, and `PatternFill` — is passed to `apply` with an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Parameter validation and error signaling

`apply` SHALL return `AdjustError` instead of panicking when parameters are out of range or buffers are malformed. It SHALL reject a zero-pixel buffer and a buffer whose `data.len()` does not equal `width * height * channels`. Each parameterized adjustment SHALL validate its documented ranges: Levels gamma greater than zero and input black below input white; Curves at least 2 and at most 14 points with strictly increasing inputs; Brightness/Contrast brightness in `-150..=150` and contrast in `-50..=100`; Exposure gamma greater than zero and exposure/offset finite; Posterize levels in `2..=255`; Threshold level in `1..=255`; Hue/Saturation hue in `-180..=180` and saturation/lightness in `-100..=100`; Black & White weights in `-200..=300` percent; Photo Filter density in `0..=100`; Channel Mixer weights and constant no greater than 200 percent in absolute value; Vibrance and saturation in `-100..=100`; Color Balance values finite and within `-100..=100`; Selective Color corrections in `-100..=100`; Gradient Map at least two stops with strictly increasing locations each at most 4096. `SolidFill`, `GradientFill`, and `PatternFill` have no `apply` parameter validation because `apply` SHALL refuse them as `AdjustError::Unsupported` and leave the buffer unchanged.

#### Scenario: Out-of-range parameters return an error

- **WHEN** an adjustment is given a value outside its documented range, such as Levels gamma `0.0`, Curves with one point, Brightness/Contrast brightness `200`, Exposure gamma `0.0`, Posterize `1`, Threshold `0`, Hue/Saturation hue `200`, Black & White weight `999`, Photo Filter density `150`, Channel Mixer weight `999`, Vibrance `500`, Color Balance `200`, Selective Color correction `200`, Gradient Map with one stop or a location above 4096, or any Solid Fill, Gradient Fill, or Pattern Fill
- **THEN** `apply` returns `AdjustError::InvalidParams` or `AdjustError::Unsupported` and does not panic

#### Scenario: Malformed buffers error instead of panicking

- **WHEN** `apply` receives a zero-pixel buffer, a 2-channel buffer, or a buffer whose data length does not match `width * height * channels`
- **THEN** `apply` returns `AdjustError` and does not panic

### Requirement: Deterministic application

`apply` SHALL be deterministic. The same adjustment applied to equal input buffers SHALL produce bit-identical output every run, with no random, time, or thread-order dependence.

#### Scenario: Repeated runs match

- **WHEN** the same adjustment is applied to two clones of one buffer
- **THEN** the two output buffers are equal, including for `Auto(Color)` and `HueSaturation`

### Requirement: Levels adjustment

The system SHALL implement `Adjustment::Levels(LevelsParams { input_black, input_white, gamma, output_black, output_white })`, applying, per composite color channel, `t = clamp((v - input_black) / (input_white - input_black), 0, 1)`, then `t' = t^(1/gamma)`, then `out = output_black + t' * (output_white - output_black)`. Defaults SHALL be input black 0, input white 255, gamma 1.0, output black 0, output white 255; the documented input and output levels span 0 through 253 (scripting-DOM clamp) and gamma spans 0.10 through 9.99. Oracle expectation: differential against ImageMagick `-level B%,W%,gamma +level Ob%,Ow%` within an absolute tolerance of 1/255 per sample.

#### Scenario: Gamma above one lightens midtones

- **WHEN** Levels is applied with input black 0, input white 255, gamma 2.0 to a mid-grey sample
- **THEN** the output is lighter than the input

#### Scenario: Input and output points remap and clamp

- **WHEN** Levels maps input black 5 to output black 20 and input white 243 to output white 235
- **THEN** samples at or below 5 read 20, samples at or above 243 read 235, and every output stays within 20 through 235

#### Scenario: Matches ImageMagick

- **WHEN** Levels with gamma 2.0 is applied to the 8x8 oracle image and ImageMagick applies `-level 0%,100%,2.0 +level 0%,100%`
- **THEN** no sample differs by more than 1

### Requirement: Curves adjustment

The system SHALL implement `Adjustment::Curves(CurvesParams { points, red,
green, blue })` where `points` is the composite curve and `red`, `green`, and
`blue` are optional per-channel curves. The composite `points` curve SHALL have
2 through 14 control points in `(input, output)` order, where inputs strictly
increase; each present per-channel curve SHALL satisfy the same 2-through-14
strictly-increasing contract. Each curve SHALL be a monotone cubic Hermite
interpolation (Fritsch-Carlson tangents) evaluated into a 256-entry LUT. The
kernel SHALL apply each present per-channel curve to its own color plane and
then apply the composite `points` LUT to all three color planes. Two endpoints
`(0, 0)` and `(255, 255)` SHALL be the exact identity, so absent per-channel
curves and an identity composite curve leave the buffer bit-exactly unchanged.
Oracle expectation: no faithful ImageMagick operator exists, so the behavior is
covered by known-value and property tests (identity, control points,
monotonicity, per-channel isolation) rather than a differential test.

#### Scenario: Two-point curve is the identity

- **WHEN** Curves is applied with `points` `[(0, 0), (255, 255)]` and no per-channel curves
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: A control point is honored

- **WHEN** Curves is applied with `points` `[(0, 0), (100, 200), (255, 255)]` to a sample at 100
- **THEN** the output sample is 200

#### Scenario: Output is monotone

- **WHEN** Curves is applied to a 0 to 255 ramp with any valid monotone point set
- **THEN** output samples never decrease as the input increases

#### Scenario: A per-channel curve touches only its channel

- **WHEN** Curves is applied with a `red` curve and an identity composite `points` curve to a buffer whose R, G, and B planes differ
- **THEN** the green and blue planes are bit-exactly unchanged and only the red plane follows the `red` curve

#### Scenario: The composite curve applies after the per-channel curves

- **WHEN** Curves is applied with both a per-channel curve and a non-identity composite `points` curve
- **THEN** each color plane is first mapped through its per-channel curve (when present) and then through the composite curve

### Requirement: Brightness/Contrast adjustment

The system SHALL implement `Adjustment::BrightnessContrast(BrightnessContrastParams { brightness, contrast, use_legacy })`. Brightness SHALL default to 0 and span `-150..=150`; contrast SHALL default to 0 and span `-50..=100`; `use_legacy` SHALL default to false. Legacy mode SHALL apply an additive brightness shift then a linear contrast expansion about mid-grey with hard clipping; modern mode SHALL apply a gamma-shaped brightness change and a monotone S-curve through `(0,0)`, `(0.5,0.5)`, `(1,1)` whose central slope is `1 + contrast/100`. Zero brightness and zero contrast SHALL be an exact identity in both modes. Oracle expectation: ImageMagick `-brightness-contrast` diverges (observed max delta 14), so the behavior is covered by property tests for identity, monotonicity in brightness, and mean shift.

#### Scenario: Neutral parameters are identity

- **WHEN** Brightness/Contrast is applied with brightness 0 and contrast 0, in either mode
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Legacy brightness clips, modern contrast stays soft

- **WHEN** legacy brightness +20 is applied to a sample at 250
- **THEN** the output clips to 255
- **WHEN** modern contrast +50 is applied to a 0 to 255 ramp
- **THEN** the endpoints stay 0 and 255, shadows darken, highlights lighten, and the result is monotone

### Requirement: Exposure adjustment

The system SHALL implement `Adjustment::Exposure(ExposureParams { exposure, offset, gamma })` in linear light. It SHALL decode each 8-bit sample from sRGB to linear, apply `c * 2^exposure`, then `+ offset`, then raise to `gamma`, then re-encode to sRGB. Exposure SHALL default to 0.0 EV and span `-20.0..=20.0`; offset SHALL default to 0.0 and span `-0.5..=0.5`; gamma SHALL default to 1.0 and span 0.01 through 9.99. The identity tuple SHALL return before any math. Oracle expectation: Photoshop applies the transfer in linear light while ImageMagick operators run in encoded space, so no differential test is attempted; property tests cover the identity and the one-EV doubling of linear light.

#### Scenario: Identity exposure is bit-exact

- **WHEN** Exposure is applied with exposure 0.0, offset 0.0, and gamma 1.0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: One EV doubles linear light

- **WHEN** Exposure +1.0 is applied to a mid-grey sample
- **THEN** the decoded linear value of the output is approximately twice the decoded linear value of the input

### Requirement: Invert adjustment

The system SHALL implement `Adjustment::Invert`, computing `out = 255 - v` independently per composite color channel with no parameters. Applying Invert twice SHALL restore the original buffer. Oracle expectation: differential against ImageMagick `-negate` with tolerance 0.

#### Scenario: Known values and involution

- **WHEN** Invert is applied to `(0, 5, 200)`
- **THEN** the result is `(255, 250, 55)`, and applying Invert again restores `(0, 5, 200)`

#### Scenario: Matches ImageMagick

- **WHEN** Invert is applied to the 8x8 oracle image and ImageMagick applies `-negate`
- **THEN** the results are identical

### Requirement: Posterize adjustment

The system SHALL implement `Adjustment::Posterize(levels: u8)` with `levels` in `2..=255`, default 4. Per composite color channel it SHALL set `q = round(v * (levels - 1) / 255)` and output `q * 255 / (levels - 1)`. Levels 255 SHALL be the 8-bit identity and a no-op. The operation SHALL be idempotent. Oracle expectation: ImageMagick `-posterize` bins on an adjacent level (observed max delta 85), so a property test asserts every output is an allowed quantization level and that levels 255 is the identity.

#### Scenario: Quantizes to the target levels

- **WHEN** Posterize 4 is applied to a sample at 64
- **THEN** the output is 85, one of `{0, 85, 170, 255}`

#### Scenario: 255 levels is the identity

- **WHEN** Posterize 255 is applied to a 0 to 255 ramp
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Re-applying does not change output

- **WHEN** Posterize 4 is applied twice to the same buffer
- **THEN** the second application leaves the result unchanged

### Requirement: Threshold adjustment

The system SHALL implement `Adjustment::Threshold(level: u8)` with `level` in `1..=255`, default 128. It SHALL compute composite luminance `Y = 0.299*R + 0.587*G + 0.114*B` (Rec.601) and write 255 to all three color channels when `Y > level`, otherwise 0, so the output is neutral. Equality SHALL fall to black. Oracle expectation: ImageMagick `-threshold` uses Rec.709 luma and diverges up to 255, so property tests assert the output is a subset of `{0, 255}` and that the split is monotone in the level.

#### Scenario: Binarizes by luma

- **WHEN** Threshold 128 is applied to pure red (luma 76), black, white, and mid-grey
- **THEN** the outputs are black, black, white, and black respectively

#### Scenario: Level zero is rejected

- **WHEN** Threshold 0 is applied
- **THEN** `apply` returns `AdjustError::InvalidParams`

### Requirement: Desaturate adjustment

The system SHALL implement `Adjustment::Desaturate` with no parameters. Per pixel it SHALL compute `(min(R, G, B) + max(R, G, B)) / 2`, rounded, and write that value to all three color channels, which is HSL lightness with saturation zero. It SHALL be equivalent to Hue/Saturation with saturation -100 within 1 LSB. Oracle expectation: differential against ImageMagick `-modulate 100,0,100` with tolerance 1.

#### Scenario: Known value and neutral passthrough

- **WHEN** Desaturate is applied to `(12, 104, 22)` and to `(77, 77, 77)`
- **THEN** the outputs are `(58, 58, 58)` and `(77, 77, 77)` respectively

#### Scenario: Matches ImageMagick

- **WHEN** Desaturate is applied to the 8x8 oracle image and ImageMagick applies `-modulate 100,0,100`
- **THEN** no sample differs by more than 1

### Requirement: Auto adjustments

The system SHALL implement `Adjustment::Auto(AutoKind::Tone | AutoKind::Contrast | AutoKind::Color)`. Tone SHALL stretch each color channel independently between percentile bounds that discard 0.1 percent of the population at each extreme. Contrast SHALL stretch all three channels by one joint pair of bounds that discards 0.5 percent, preserving channel ratios so no cast is introduced. Color SHALL apply the per-channel stretch with 0.5 percent clipping, then scale each channel's midtone gamma so the mean near-neutral midtone converges to the shared mean. Bounds SHALL be derived from the buffer histogram. Oracle expectation: Adobe's Auto Color Correction solver is closed, so behavior is covered by property tests (per-channel stretch, joint no-cast, midtone neutralization).

#### Scenario: Auto Tone stretches the range

- **WHEN** Auto Tone is applied to a ramp whose values span 60 through 200
- **THEN** the dark end maps near 0 and the light end maps near 255

#### Scenario: Auto Contrast does not introduce a cast

- **WHEN** Auto Contrast is applied to a neutral grey ramp
- **THEN** every output pixel keeps R equal to G equal to B

#### Scenario: Auto Color pulls channels toward neutral

- **WHEN** Auto Color is applied to a ramp with a fixed channel offset
- **THEN** the per-channel sums move closer together than before

### Requirement: Hue/Saturation adjustment

The system SHALL implement `Adjustment::HueSaturation(HueSaturationParams { hue, saturation, lightness })`. Hue SHALL default to 0 and span `-180..=180`; saturation SHALL default to 0 and span `-100..=100`; lightness SHALL default to 0 and span `-100..=100`. It SHALL convert each pixel to HSL, rotate hue by the hue value, scale saturation by `1 + saturation/100`, adjust lightness toward white for positive values and toward black for negative values, and convert back. All-zero parameters SHALL be an exact identity, and saturation -100 SHALL produce a neutral grey at the pixel's HSL lightness. This change covers the composite (Master) path only. Oracle expectation: ImageMagick `-modulate` uses its own HSL space (observed max delta 45), so property tests cover the identity and increased channel spread under positive saturation.

#### Scenario: Neutral parameters are identity

- **WHEN** Hue/Saturation is applied with hue 0, saturation 0, and lightness 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Saturation minus 100 is grey

- **WHEN** Hue/Saturation is applied with saturation -100
- **THEN** all three color channels of every pixel are equal to the rounded HSL lightness

### Requirement: Black & White adjustment

The system SHALL implement `Adjustment::BlackWhite(BlackWhiteParams { red, yellow, green, cyan, blue, magenta, tint, tint_color })`. The six weights SHALL be percentages in `-200..=300` with defaults red 40, yellow 60, green 40, cyan 60, blue 20, magenta 80. It SHALL decompose each pixel into a neutral component plus a hue-sector pair and apply the matching weights so the result is a pure grey. When `tint` is set, it SHALL map the grey through the tint colour's hue and saturation; a black tint colour SHALL be rejected. Oracle expectation: no faithful ImageMagick operator exists, so known-value tests cover the neutral passthrough, the red weight, and tinting.

#### Scenario: Neutral grey passes through

- **WHEN** Black & White is applied with the default weights to a neutral grey `(100, 100, 100)`
- **THEN** the output is `(100, 100, 100)`

#### Scenario: Pure red uses the red weight

- **WHEN** the default weights are applied to `(200, 0, 0)`
- **THEN** the output is grey 80

#### Scenario: Tint shifts the grey

- **WHEN** tint is enabled with a sepia tint colour and the result is applied to a neutral grey
- **THEN** the red channel is greater than the blue channel

### Requirement: Photo Filter adjustment

The system SHALL implement `Adjustment::PhotoFilter(PhotoFilterParams { color, density, preserve_luminosity })`. Density SHALL default to 25 and span `0..=100`; `preserve_luminosity` SHALL default to true. Per pixel it SHALL compute `C' = C * (1 - d) + F * d` where `d = density/100` and `F` is the filter colour, then rescale toward the input luminance when `preserve_luminosity` is set. Density 0 SHALL be a no-op. Oracle expectation: no faithful ImageMagick operator exists, so property tests cover warming, luminosity preservation, and the density-zero no-op.

#### Scenario: Density zero is a no-op

- **WHEN** Photo Filter is applied with density 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Warming filter preserves luminosity

- **WHEN** a warm filter `(255, 180, 80)` is applied at density 25 with luminosity preservation to neutral greys
- **THEN** red exceeds blue and each output luminance is within 3 of the input

### Requirement: Channel Mixer adjustment

The system SHALL implement `Adjustment::ChannelMixer(ChannelMixerParams { monochrome, red, green, blue, constant })`. Source weights and constants SHALL be percentages no greater than 200 in absolute value, with the identity matrix `[100, 0, 0]`, `[0, 100, 0]`, `[0, 0, 100]` and zero constants. Per output channel it SHALL compute `out = (w[0]*R + w[1]*G + w[2]*B) / 100 + constant/100 * 255`, clamped. Monochrome SHALL use the red row as one source mix for all three outputs. Oracle expectation: ImageMagick `-color-matrix` takes fractions while Photoshop uses percent weights (observed max delta 252), so property tests cover the exact identity and a hand-computed blend.

#### Scenario: Identity matrix is a no-op

- **WHEN** the identity matrix is applied
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Known blend

- **WHEN** the red row is set to sources `[50, 100, 0]` and the pixel is `(50, 100, 200)`
- **THEN** the red output is 125 and green and blue are unchanged

### Requirement: Vibrance adjustment

The system SHALL implement `Adjustment::Vibrance(VibranceParams { vibrance, saturation })`. Both values SHALL default to 0 and span `-100..=100`. It SHALL convert each pixel to HSL, apply the saturation scale `S * (1 + saturation/100)`, and add a vibrance delta that falls off as `1 - S` and is damped in the skin-tone hue band, then clamp saturation to `[0, 1]`. Saturation -100 SHALL produce a neutral grey. Oracle expectation: no faithful ImageMagick operator exists, so property tests cover the identity, the diminishing boost, and saturation -100.

#### Scenario: Neutral parameters are identity

- **WHEN** Vibrance is applied with vibrance 0 and saturation 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Saturation minus 100 is grey

- **WHEN** Vibrance is applied with saturation -100
- **THEN** all three color channels of the pixel are equal

### Requirement: Color Balance adjustment

The system SHALL implement `Adjustment::ColorBalance(ColorBalanceParams { shadows, midtones, highlights, preserve_luminosity })`. Each tonal band SHALL hold three shifts in the order cyan-red, magenta-green, yellow-blue, each defaulting to 0 and spanning `-100..=100`. Per pixel it SHALL compute `Y = 0.299*R + 0.587*G + 0.114*B`, weight the three bands with overlapping parabola windows that peak at `Y = 0`, `Y = 0.5`, and `Y = 1`, add the weighted per-channel shifts, and clamp. `preserve_luminosity` SHALL default to true and rescale the result toward the input luminance. A midtone-only shift SHALL leave black untouched. Oracle expectation: no faithful ImageMagick operator exists, so property tests cover the midtone red shift and luminosity preservation.

#### Scenario: Midtone shift moves grey toward red

- **WHEN** a midtones red shift of +100 is applied to mid-grey
- **THEN** the red channel increases and black is unchanged

#### Scenario: Luminosity is preserved

- **WHEN** a midtone shift is applied with `preserve_luminosity` set to true to mid-grey
- **THEN** the output luminance is within 3 of the input

### Requirement: ImageMagick differential oracle and no-equivalent classification

The system SHALL ship `scripts/adjust_oracle.py`, which applies an ImageMagick operator to a raw 8-bit image, and `crates/pictura-adjust/tests/oracle.rs`, which diffs `apply` against it. The test table SHALL have exactly one row for each destructive adjustment variant plus a no-equivalent row for the generative `GradientFill` and `PatternFill`. `SolidFill` is also refused fill content with no `apply`, so it has no oracle row; its refusal is covered by the alpha/refusal tests. Only Levels, Invert, and Desaturate SHALL be diffed (`-level ... +level ...`, `-negate`, `-modulate 100,0,100`), with tolerances of 1, 0, and 1. The other seventeen adjustments SHALL be classified as having no faithful ImageMagick operator and SHALL use tolerance 0: BlackWhite, PhotoFilter, GradientMap, GradientFill, PatternFill, Vibrance, ColorBalance, Auto, Curves, Exposure, BrightnessContrast, HueSaturation, ChannelMixer, SelectiveColor, ColorLookup, Posterize, and Threshold. The differential tests SHALL skip with a message when `magick` is not on `PATH` and MUST NOT be marked `#[ignore]`.

#### Scenario: Mapping table matches the implemented contract

- **WHEN** the oracle tests run
- **THEN** the mapping table has 20 rows, every no-equivalent row has tolerance 0, and each row carries a non-empty note

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes

### Requirement: Gradient Map adjustment

The system SHALL implement `Adjustment::GradientMap(GradientMapParams {
stops, reverse })`, where each `GradientStop { location, color }` carries a
Photoshop gradient position in `0..=4096` and an RGB colour, and `reverse`
defaults to false. It SHALL map each pixel's Rec.601 luminance `L` through the
gradient: when `reverse` is set it SHALL sample `1 − L` instead, it SHALL
interpolate linearly between the two stops bracketing the sampled position, it
SHALL clamp sampled positions below the first stop and above the last stop to
the endpoint colours, and it SHALL write the sampled colour to the three colour
channels while leaving alpha untouched. `apply` SHALL reject a stop list with
fewer than two stops, a non-increasing `location`, or any `location` above
4096. Oracle expectation: no faithful ImageMagick operator exists, so property
tests cover the identity of a black-to-white ramp over a neutral grey range, the
reversed mapping, and the endpoint clamp.

#### Scenario: Black to white gradient is the identity on neutral greys

- **WHEN** Gradient Map is applied with stops at location 0 colour `[0, 0, 0]` and location 4096 colour `[255, 255, 255]` to a neutral grey ramp
- **THEN** each grey sample maps to itself within 1 LSB

#### Scenario: Reverse flips the lookup

- **WHEN** the same black-to-white gradient is applied with `reverse` true
- **THEN** an input of black maps near white and an input of white maps near black

#### Scenario: Samples outside the stop range clamp to the endpoints

- **WHEN** a two-stop gradient places its stops at locations 1024 and 3072
- **THEN** samples below location 1024 read the first stop's colour and samples above location 3072 read the last stop's colour

#### Scenario: A three-stop gradient honors the interior stop

- **WHEN** a gradient has a distinct interior stop at location 2048
- **THEN** a pixel whose luminance samples location 2048 reads the interior stop's colour rather than a blend of the outer stops

#### Scenario: Invalid stops are rejected

- **WHEN** Gradient Map is given fewer than two stops, duplicate or decreasing locations, or a location above 4096
- **THEN** `apply` returns `AdjustError::InvalidParams` and does not panic

#### Scenario: Alpha is preserved

- **WHEN** Gradient Map is applied to an RGBA buffer
- **THEN** the alpha plane is bit-identical to the input

### Requirement: Gradient Fill adjustment model

The system SHALL provide `Adjustment::GradientFill(GradientFillParams {
stops, reverse, kind, angle_deg, scale })`, where `stops` is a list of the
existing `GradientStop { location, color }` positions in `0..=4096` with an RGB
colour, `kind` is a `GradientKind` of Linear, Radial, Angle, Reflected, or
Diamond, `angle_deg` is the rotation in degrees, `scale` is a percent defaulting
to 100, and `reverse` defaults to false. A gradient fill is composited
generatively rather than applied to the backdrop, so `apply` SHALL refuse it as
`AdjustError::Unsupported` and SHALL leave the buffer unchanged.

#### Scenario: The model carries the decoded parameters

- **WHEN** a `GradientFill` is constructed with two stops, kind Radial, angle 45, scale 100, and reverse false
- **THEN** it is a distinct `Adjustment` variant carrying those values

#### Scenario: Apply refuses a Gradient Fill

- **WHEN** `apply` is called with `Adjustment::GradientFill` and any buffer
- **THEN** it returns `AdjustError::Unsupported` and the buffer is unchanged

### Requirement: Pattern Fill adjustment model

The system SHALL provide `Adjustment::PatternFill(PatternFillParams { pattern_id,
scale, link_with_layer, origin })`, where `pattern_id` is the id of a pattern in
the document's pattern library, `scale` is a percentage defaulting to 100,
`link_with_layer` defaults to true and anchors the tiling to the layer
rectangle, and `origin` is a pixel offset defaulting to `(0, 0)` that shifts the
tiled sample. A pattern fill is composited generatively rather than applied to
the backdrop, so `apply` SHALL refuse it as `AdjustError::Unsupported` and SHALL
leave the buffer unchanged.

#### Scenario: The model carries the decoded parameters

- **WHEN** a `PatternFill` is constructed with a pattern id, scale 50, `link_with_layer` false, and origin `(3, 4)`
- **THEN** it is a distinct `Adjustment` variant carrying those values

#### Scenario: Apply refuses a Pattern Fill

- **WHEN** `apply` is called with `Adjustment::PatternFill` and any buffer
- **THEN** it returns `AdjustError::Unsupported` and the buffer is unchanged

### Requirement: Selective Color adjustment

The system SHALL implement `Adjustment::SelectiveColor(SelectiveColorParams {
method, ranges })` where `method` is `SelectiveColorMethod::Relative` or
`SelectiveColorMethod::Absolute` and `ranges` is nine `SelectiveRange` records in
reds, yellows, greens, cyans, blues, magentas, whites, neutrals, and blacks
order, each holding `c`, `m`, `y`, and `k` `i16` corrections in `-100..=100`
(default 0). `apply` SHALL reject any correction outside `-100..=100` with
`AdjustError::InvalidParams`. When all nine ranges are zero the kernel SHALL
return the buffer bit-exactly unchanged. Otherwise, per composite pixel, it
SHALL run libpsd's integer pipeline: convert RGB to an integer CMYK with
`c = (255 - r - k) * 255 / (255 - k)` and `k = min(255 - r, 255 - g, 255 - b)`
(and zero c/m/y when `k == 255`), compute the hue as an integer in `0..=359`
from the max/min channels, and copy the source ink to the destination. For the
six hue families (i = 1..=6 at model indices 0..=5) it SHALL compute the window
`r0 = -105 + i*60`, `r1 = r0 + 30`, `r2 = r1 + 30`, `r3 = r2 + 30`; when
`r0 <= hue < r3` it SHALL set the opacity to `255` inside `[r1, r2)`, to
`(hue - r0) * 255 / 30` below `r1`, and to `(r3 - hue) * 255 / 30` above `r2`,
then add, per ink component, `dst += src * corr * opacity / 25500` for Relative
and `dst += 255 * corr * opacity / 25500` for Absolute. For whites, neutrals, and
blacks (i = 7..=9 at model indices 6..=8) it SHALL select by the source black ink
(`i == 7` when `src_black == 0`, `i == 8` when `0 < src_black < 255`, `i == 9`
when `src_black == 255`) with no hue opacity and add, per ink component,
`dst += src * corr / 100` for Relative and `dst += 255 * corr / 100` for
Absolute. All divisions SHALL truncate toward zero, matching C integer division.
The destination ink SHALL be clamped to `0..=255` and converted back to RGB with
`out = (65535 - (ink * (255 - k) + (k << 8))) >> 8`. Oracle expectation: the
conversion is profile-free and its integer RGB-to-CMYK-to-RGB round-trip can
shift a channel by about 2 LSB, so there is no ImageMagick equivalent and no
Photoshop pixel-parity claim; known-value tests cover the identity, the Relative
and Absolute hue windows, and the tonal ranges.

#### Scenario: Neutral Selective Color is the exact identity

- **WHEN** Selective Color is applied with the relative method and all nine ranges zero to any buffer
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: The Relative hue window scales the existing ink

- **WHEN** Selective Color is applied with the relative method and the reds range `m = 50`, the other ranges zero, to a pixel `(200, 100, 50)`
- **THEN** the output is `(200, 61, 51)`, matching libpsd's `src * corr * opacity / 25500` on the reds/yellows overlap

#### Scenario: The Absolute hue window adds full ink

- **WHEN** Selective Color is applied with the absolute method and the reds range `y = 100`, the other ranges zero, to a pixel `(200, 100, 50)`
- **THEN** the output is `(200, 101, 1)`

#### Scenario: Blacks range lightens pure black

- **WHEN** Selective Color is applied with the relative method and the blacks range `k = -100`, the other ranges zero, to a pixel `(0, 0, 0)`
- **THEN** the output is `(255, 255, 255)`

#### Scenario: Whites range adds cyan to pure white

- **WHEN** Selective Color is applied with the absolute method and the whites range `c = 100`, the other ranges zero, to a pixel `(255, 255, 255)`
- **THEN** the output is `(1, 255, 255)`

#### Scenario: Out-of-range corrections are rejected

- **WHEN** a `SelectiveRange` carries a correction above 100 or below -100
- **THEN** `apply` returns `AdjustError::InvalidParams` and does not panic

### Requirement: Color Lookup adjustment

The system SHALL implement `Adjustment::ColorLookup(ColorLookupParams)` where
`ColorLookupParams` carries a `ColorLookupKind` (3-D LUT, abstract profile, or
device link) and an optional parsed three-dimensional lookup
`Lut3d { size, points }` whose points are RGB triples in the `.CUBE` order (red
index fastest, then green, then blue) and whose `size` is the number of grid
steps per axis. `apply` SHALL sample the lookup trilinearly per pixel: normalize
each channel to `0.0..=1.0`, scale by `size - 1`, take the eight surrounding
nodes, and interpolate with the fractional part, writing the sampled RGB to the
colour channels and leaving alpha untouched. When the lookup is `None` (an
abstract-profile or device-link kind, or an embedded format other than a valid
`.CUBE`), `apply` SHALL leave the buffer bit-exactly unchanged. A `Lut3d` with a
size below 2, a size above 64, a point count that is not `size³`, or any
non-finite component SHALL be rejected as `AdjustError::InvalidParams`. Oracle
expectation: no ImageMagick operator applies an arbitrary `.cube` and
Photoshop's sampling is not independently reproducible here, so there is no
Adobe pixel-parity claim; known-value tests cover the identity, exact node
mapping (fixing the red-fastest point order), a trilinear blend, and the
`None` no-op.

#### Scenario: An identity lookup is the identity

- **WHEN** Color Lookup is applied with a `size`-2 identity lookup to any buffer
- **THEN** every colour channel is within 1 LSB of its input and alpha is bit-identical

#### Scenario: A corner node maps exactly

- **WHEN** a lookup's corner node is a distinct colour and a pixel sits exactly on that node
- **THEN** the output is that node's colour within 1 LSB

#### Scenario: The red-fastest point order is honored

- **WHEN** a `size`-3 lookup colours only the node at red 1, green 0, blue 0 and a pixel maps exactly to it
- **THEN** the output is that node's colour, proving the point order is red-fastest

#### Scenario: A missing lookup is a no-op

- **WHEN** `apply` is called with `ColorLookupParams` whose lookup is `None`
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: An invalid lookup is rejected

- **WHEN** a `Lut3d` has a size below 2 or above 64, a point count that is not `size³`, or a non-finite component
- **THEN** `apply` returns `AdjustError::InvalidParams` and does not panic

#### Scenario: Alpha is preserved

- **WHEN** Color Lookup is applied to an RGBA buffer
- **THEN** the alpha plane is bit-identical to the input

### Requirement: Shadows/Highlights adjustment

The system SHALL implement `Adjustment::ShadowsHighlights(ShadowsHighlightsParams { shadows_amount, highlights_amount })` as a pointwise operator. Both amounts SHALL default to `0.0` and span `0.0..=100.0`. It SHALL derive a 256-entry luminance delta table: a shadow weight that peaks at black and fades to zero by mid-grey scaled by `(shadows_amount / 100) * 0.35 * 255`, minus a highlight weight that peaks at white and fades to zero by mid-grey scaled by `(highlights_amount / 100) * 0.30 * 255`; each pixel's Rec.601 luminance selects its delta, which is added to all three colour channels and clamped to `0..=255`. Both amounts zero SHALL be an exact identity, alpha SHALL be preserved, and a non-finite or out-of-range amount SHALL be rejected as `AdjustError::InvalidParams` without changing the buffer.

#### Scenario: Lifting shadows and pulling highlights

- **WHEN** Shadows/Highlights with a positive shadow amount is applied to a buffer containing a dark pixel, or with a positive highlight amount to a bright pixel
- **THEN** the dark pixel lightens or the bright pixel darkens respectively

#### Scenario: Neutral amounts are identity

- **WHEN** Shadows/Highlights is applied with both amounts zero
- **THEN** the buffer is bit-exactly unchanged and alpha is preserved

#### Scenario: Invalid amounts are rejected

- **WHEN** either amount is non-finite or outside `0.0..=100.0`
- **THEN** `apply` returns `AdjustError::InvalidParams` and does not panic

### Requirement: Replace Color adjustment

The system SHALL implement `Adjustment::ReplaceColor(ReplaceColorParams { samples, fuzziness, localized, hue, saturation, lightness })`. `samples` is a list of `(x, y, rgb)` picks; `fuzziness` spans `0.0..=200.0`; `hue` spans `-180.0..=180.0` degrees; `saturation` and `lightness` span `-100.0..=100.0`. For each pixel the system SHALL compute a weight as the maximum over samples of a Chebyshev colour-distance ramp (exact match when fuzziness is 0), optionally attenuated by a Gaussian of the pixel-to-sample distance when `localized` is set, then blend the pixel toward its hue-rotated, saturation- and lightness-shifted HSL result by that weight. An empty sample list SHALL be an exact identity, alpha SHALL be preserved, and out-of-range or non-finite parameters SHALL be rejected as `AdjustError::InvalidParams`.

#### Scenario: Zero fuzziness selects exact samples only

- **WHEN** Replace Color with fuzziness 0 and one sample is applied to a buffer
- **THEN** only pixels exactly equal to the sample colour change, and no others

#### Scenario: Fuzziness shifts near colours

- **WHEN** Replace Color with a positive fuzziness and a non-zero hue is applied to pixels near the sample colour
- **THEN** those pixels shift toward the new hue

#### Scenario: Empty samples is identity and alpha is preserved

- **WHEN** Replace Color is applied with an empty sample list, or to an RGBA buffer
- **THEN** the buffer is unchanged, and in the RGBA case the alpha plane is bit-identical

### Requirement: Destructive Color Lookup dialog

`Image ▸ Adjustments ▸ Color Lookup` SHALL open a dialog offering the CS6 preset looks `None`, `Warm Contrast`, `Cool Shadows`, `Faded Film`, `Bleach Bypass`, `Crisp Warm`, and `Moonlight`, defaulting to `None` (the identity). Choosing a preset SHALL rebuild the adjustment's embedded lookup so that `None` leaves the image unchanged and any other preset changes the colour channels while preserving alpha. OK SHALL commit one history state named for the adjustment and Cancel SHALL restore the pre-dialog pixels.

#### Scenario: The dialog opens on the identity look

- **WHEN** the Color Lookup dialog is opened and accepted without changing the preset
- **THEN** the image is unchanged and one adjustment state is recorded

#### Scenario: A non-identity preset changes the image

- **WHEN** `Warm Contrast` is selected and the dialog is accepted
- **THEN** the colour channels change and alpha is preserved

### Requirement: Destructive adjustment dialog enablement

Every destructive `Image ▸ Adjustments` entry backed by an engine kind SHALL be enabled only when the active layer is an editable pixel layer, and choosing it SHALL preview on the canvas without recording history until OK commits exactly one state named for the adjustment; Cancel SHALL restore the pre-dialog pixels bit-identically.

#### Scenario: Cancel restores the pre-dialog pixels

- **WHEN** a destructive adjustment dialog is opened, previewed, and cancelled
- **THEN** the document pixels equal the pre-dialog pixels bit for bit and no history state is recorded

#### Scenario: Refused with no editable layer

- **WHEN** an adjustment dialog is invoked with no document or with a group or adjustment layer active
- **THEN** the command is disabled or refused and the document is unchanged

### Requirement: Photo Filter colour picker samples the image

While the Color Picker opened from the Photo Filter dialog's Color swatch is
open, the canvas SHALL show the eyedropper cursor, and a left click on the
image SHALL set the picker's colour to the document composite at that pixel
as it was before the dialog's preview. The click SHALL reach the canvas even
though both dialogs block input to their parent windows, and it MUST NOT run
the active tool or record history. A click outside the document SHALL leave
the colour unchanged. Closing the picker SHALL end the sampling.

#### Scenario: A canvas click samples the base image

- **WHEN** the Photo Filter preview is on and the user clicks a pixel of the
  image while the Color Picker is open
- **THEN** the picker takes that pixel's un-filtered composite colour, and
  accepting it sets the Color swatch and selects Use: Color

#### Scenario: Closing the picker ends sampling

- **WHEN** the Color Picker closes
- **THEN** the canvas sampler is disarmed and a later click reaches the
  active tool again
