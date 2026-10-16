## ADDED Requirements

### Requirement: Field blur (pin-interpolated variable blur)

The system SHALL implement `Filter::FieldBlur { pins: Vec<BlurPin> }` with
`BlurPin { x: f64, y: f64, blur: f64 }` at the crate root. `x` and `y` SHALL
be fractions of the buffer (0..1 inside the image; a pin outside SHALL be
allowed so it can anchor the border), and `blur` SHALL be a radius in buffer
pixels spanning `0..=MAX_FIELD_RADIUS` (500). At most `MAX_FIELD_PINS` (16)
pins SHALL be accepted. Each pixel's blur radius SHALL be the
inverse-square-distance weighted average of the pin amounts
(`r(p) = Σ wᵢ·blurᵢ / Σ wᵢ`, `wᵢ = 1/(dᵢ² + ε)`), so the field is that pin's
amount at a pin and a smooth gradient between pins. The kernel SHALL apply a
variable-radius blur whose strength follows `r(p)` and SHALL leave a pixel
with `r(p) = 0` unchanged; an empty pin list SHALL be a no-op. The kernel
SHALL blur every plane it is given, alpha included, so a layer's edge
feathers. Non-finite coordinates or amounts, an amount above `MAX_FIELD_RADIUS`,
and more than `MAX_FIELD_PINS` pins SHALL be rejected with
`FilterError::InvalidParams` before any pixel is written. The kernel SHALL be
deterministic: equal inputs SHALL produce bit-identical output. Oracle
expectation: no faithful ImageMagick operator exists (ImageMagick has no
pin-interpolated variable blur), so property and known-value tests cover the
behavior and the mapping row is no-equivalent with tolerance 0 and a
non-empty note.

#### Scenario: A pin gradient blurs more toward the larger amount

- **WHEN** Field Blur is applied with one pin at the left with blur 0 and one
  pin at the right with blur 40 to an edge or impulse buffer
- **THEN** the left region stays sharp, the right region's transition is
  wider, and the transition widens monotonically from left to right

#### Scenario: A pin outside the image anchors the border

- **WHEN** Field Blur is applied with a sharp pin outside one edge and a
  blurred pin inside the opposite half
- **THEN** the border near the outside pin stays sharper than the interior
  near the blurred pin

#### Scenario: Zero pins is a no-op

- **WHEN** Field Blur is applied with no pins
- **THEN** every byte of the buffer is unchanged

#### Scenario: Invalid parameters are rejected without mutation

- **WHEN** Field Blur is applied with a non-finite coordinate, a negative or
  over-500 blur amount, or more than 16 pins
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is
  bit-exactly unchanged

#### Scenario: Alpha blurs with the colour

- **WHEN** Field Blur is applied to a four-channel buffer with a hard alpha
  edge under a blurring pin, and again with a zero amount
- **THEN** the edge feathers, and the zero amount leaves alpha bit-identical

#### Scenario: No faithful equivalent is documented

- **WHEN** the oracle mapping table is checked
- **THEN** Field Blur has no ImageMagick operator, tolerance 0, and a
  non-empty no-equivalent note

### Requirement: Field Blur Blur Effects and Noise passes

When `bokeh.enabled`, the kernel SHALL apply the Blur Gallery's Blur Effects
to the blurred pixels only (field weight > 0): `light_bokeh` (`0..=100`)
SHALL multiply a pixel by `1 + 2 × light_bokeh/100 × luma/255 × weight`, so a
bright, well-blurred pixel blooms brilliantly toward white; `bokeh_color`
(`0..=100`) SHALL push the lightened, non-clipped pixels away from their
luminance, and `light_range` (two `0..=255` values) SHALL gate both to that
tonal window. When `noise.enabled`, the kernel SHALL re-inject grain scaled
by `amount` (`0..=100`) and the field weight, damped in bright areas by
`highlights` (`0..=100`), drawn with the `Gaussian` model (a near-Gaussian
per-pixel draw), the `Uniform` model (a uniform per-pixel draw), or the
`Grain` model (interpolated lattice cells sized by `size`, blended toward
per-pixel jitter by `roughness`), monochromatic at `color` 0 and per-channel
at 100, seeded by `seed` so equal inputs and seeds produce bit-identical
output. `light_bokeh`, `bokeh_color`, `amount`, `size`, `roughness`, `color`,
and `highlights` outside `0..=100` SHALL be rejected with
`FilterError::InvalidParams` before any pixel is written. The passes SHALL
write the colour planes only.

#### Scenario: Light Bokeh brightens only the blurred band

- **WHEN** Field Blur with a sharp pin and a blurred pin is applied to a flat
  mid-grey buffer with Light Bokeh 100 over the whole tonal range
- **THEN** the blurred end brightens while a pixel at the sharp pin is
  unchanged, and Light Bokeh 0 with Bokeh Color 0 is a no-op

#### Scenario: Bokeh Color adds chroma to the lightened areas

- **WHEN** Field Blur with Bokeh Color 100 is applied to a flat tinted buffer
- **THEN** the blurred end's channel spread grows while a pixel at the sharp
  pin is unchanged

#### Scenario: Light Range gates the bokeh

- **WHEN** the range is 100..255 and the buffer's tone is 50
- **THEN** the blurred pixels are unchanged, while a tone of 200 in range
  brightens

#### Scenario: Noise is seeded and follows its controls

- **WHEN** the Noise pass with a non-zero amount is applied twice, and again
  with a different seed, colour 100, highlights 100, or another model
- **THEN** equal seeds are bit-identical, a different seed differs, colour 100
  separates the channels where colour 0 keeps them equal, highlights 100
  damps the grain in a bright buffer, Gaussian/Uniform/Grain differ from one
  another, and amount 0 is a no-op

#### Scenario: Invalid effect parameters are rejected without mutation

- **WHEN** the bokeh or noise values fall outside their ranges or are
  non-finite
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is
  bit-exactly unchanged
