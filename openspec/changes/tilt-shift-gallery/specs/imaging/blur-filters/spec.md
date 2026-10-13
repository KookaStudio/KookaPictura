## ADDED Requirements

### Requirement: Tilt-shift blur (band focus variable blur)

The system SHALL implement `Filter::TiltShift { pins: Vec<TiltPin>, bokeh,
noise }` with `TiltPin { x, y, angle, focus_above, focus_below, fade_above,
fade_below, distortion, symmetric, blur }` at the crate root. `x` and `y`
SHALL be fractions of the buffer (outside allowed); `angle` SHALL turn the
band clockwise in degrees; the four extents SHALL be fractions of the buffer
height in `0..=MAX_TILT_EXTENT` (10), measured across the band, `focus_*`
from the centre line to each solid line and `fade_*` from each solid line to
its dashed line; `distortion` SHALL be in `-100..=100`; and `blur` SHALL be
the full radius in buffer pixels in `0..=MAX_FIELD_RADIUS`. At most
`MAX_FIELD_PINS` (16) pins SHALL be accepted. A pin's falloff SHALL be 0
within its focus extent on that side, 1 beyond focus plus fade, and a
smoothstep between; the per-pixel radius SHALL be the product of the pins'
falloffs times the inverse-square-distance blend of their amounts. A
non-zero distortion SHALL smear the blurred zone below the band (both zones
when `symmetric`) along the radius from the pin centre when positive and
around it when negative, over a length proportional to the local radius,
before the Blur Effects / Noise passes; a pixel whose radius is 0 SHALL stay
unchanged. The blur and smear SHALL run over the alpha plane too, so a
layer's edge feathers beyond the band (the effects passes stay
colour-only); the kernel SHALL treat an
empty pin list as a no-op, and SHALL reject non-finite or out-of-range
values with `FilterError::InvalidParams` before writing any pixel. Equal
inputs SHALL produce bit-identical output. Oracle expectation: no faithful
ImageMagick operator exists; the mapping row is no-equivalent with tolerance
0 and a non-empty note.

#### Scenario: The focus band is untouched and the far rows blur

- **WHEN** Tilt-Shift is applied with one horizontal band to a noisy picture
- **THEN** every pixel within the focus extents is bit-identical and the
  rows beyond the fade change

#### Scenario: The fade ramps and each side keeps its extents

- **WHEN** an impulse is blurred inside the band, in the fade, and beyond
  it, or the band's two sides are given different focus extents
- **THEN** the spread grows from the band through the fade, and each side
  stays sharp exactly as far as its own extent

#### Scenario: Rotation and two bands

- **WHEN** a band is rotated 90°, or two separate bands are applied
- **THEN** the sharp area turns vertical, or both bands stay sharp while the
  gap between them blurs

#### Scenario: Distortion acts below the band unless symmetric

- **WHEN** a positive distortion is applied, then the same with
  `symmetric`, then a negative one
- **THEN** only the zone below changes from the undistorted result, then
  both zones do, and the negative result differs from the positive while
  the band stays untouched

#### Scenario: A layer's edge feathers beyond the band only

- **WHEN** Tilt-Shift is applied to a four-channel buffer with hard alpha
  stripes
- **THEN** the band rows keep their alpha bit-identical and the far rows'
  alpha softens

#### Scenario: Invalid parameters are rejected without mutation

- **WHEN** a pin has a non-finite value, a negative or over-10 extent, a
  distortion outside -100..100, a blur outside 0..500, or more than 16 pins
  are given
- **THEN** `apply` returns `FilterError::InvalidParams` and the buffer is
  unchanged

### Requirement: Combined Blur Gallery kernel

The system SHALL implement `Filter::BlurGallery { field: Vec<BlurPin>, iris:
Vec<IrisPin>, tilt: Vec<TiltPin>, bokeh, noise }`. Each non-empty list SHALL
build its radius field exactly as its own kernel does, and each pixel SHALL
take the largest of those radii (inferred; Adobe's combine is closed) in one
shared variable-blur pass; Tilt-Shift's distortion smear SHALL follow when
its pins distort, then the Blur Effects / Noise passes. An effect alone SHALL
produce bytes identical to its own kernel. All three lists empty SHALL be a
no-op; each list SHALL be validated as its own kernel validates it, before
any pixel is written. Equal inputs SHALL produce bit-identical output.
Oracle expectation: no-equivalent with tolerance 0 and a non-empty note.

#### Scenario: One effect alone matches its own kernel

- **WHEN** the combined kernel runs with only Field, only Iris, or only
  Tilt-Shift pins, with the effects on
- **THEN** each result is byte-identical to that effect's own kernel

#### Scenario: Each effect blurs the other's sharp area

- **WHEN** an iris and a band are combined
- **THEN** a band-row pixel the iris blurs changes, an iris-core pixel the
  band blurs changes, and a pixel sharp in both is bit-identical

