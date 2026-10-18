## ADDED Requirements

### Requirement: Spin blur (rotational motion blur)

The system SHALL implement `Filter::SpinBlur { spins: Vec<SpinPin>, strobe:
SpinStrobe, noise }` with `SpinPin { x, y, radius_x, radius_y, angle,
feather, blur_angle, pivot_x, pivot_y }` and `SpinStrobe { strength,
flashes, duration }` at the crate root, labelled non-parity (a post-CS6
effect, approximated). A spin's centre and rotation-centre offset SHALL be
fractions of the buffer (outside allowed), its semi-axes fractions of the
buffer width in `0.0001..=10`, its `feather` in `0..=1`, and its
`blur_angle` in `0..=MAX_SPIN_ANGLE` (360°); `strength` SHALL be in
`0..=100`, `flashes` in `1..=100`, and `duration` in `0..=360`; at most
`MAX_FIELD_PINS` (16) spins SHALL be accepted. Each pixel's mask SHALL be 1
inside the feather ring, 0 outside the ellipse, and a smoothstep between,
taking the spin with the largest mask; the pixel SHALL average taps along
the circular arc around that spin's rotation centre, centred on the pixel,
sweeping `blur_angle` inside the feather ring and, past it, keeping the
ring's arc length times the mask, so the blur only weakens toward the
outline; the strobe flashes SHALL mix in by strength.
An arc shorter than one pixel SHALL leave the pixel unchanged. The gather
SHALL run over the alpha plane too; Noise SHALL follow, weighted by the
mask; the kernel SHALL treat an empty spin list as a no-op and SHALL reject
non-finite or out-of-range values with `FilterError::InvalidParams` before
writing any pixel. Equal inputs SHALL produce bit-identical output. Oracle
expectation: no faithful ImageMagick operator exists; the mapping row is
no-equivalent with tolerance 0 and a non-empty note.

#### Scenario: A spin blurs inside its ellipse only

- **WHEN** a spin is applied to a striped picture
- **THEN** pixels inside the feather ring change and pixels outside the
  ellipse are bit-identical

#### Scenario: The smear shortens past the feather ring

- **WHEN** a spin's feather ring is pulled in toward its centre
- **THEN** along a ray from the centre the arc length peaks at the ring and
  falls toward the outline

#### Scenario: The blur follows circles round the rotation centre

- **WHEN** a spin centred on a picture of concentric rings is applied
- **THEN** the picture is unchanged up to rounding, while a picture of
  radial spokes changes

#### Scenario: A moved rotation centre moves the blur

- **WHEN** the same spin is applied with its rotation centre offset
- **THEN** the output differs from the centred spin's

#### Scenario: Strobe flashes separate the trail

- **WHEN** an impulse is spun with Strobe Strength 100 and four flashes
- **THEN** its arc shows separated bright taps instead of an even smear

#### Scenario: Invalid spins are rejected

- **WHEN** a spin has a non-finite value, a Blur Angle above 360, a zero
  radius, or the strobe values are out of range
- **THEN** the kernel returns `InvalidParams` and the buffer is unchanged

### Requirement: Combined kernel with spins

`Filter::BlurGallery` SHALL gain `spin: Vec<SpinPin>` and `spin_strobe:
SpinStrobe`. With spins, the kernel SHALL run the radius-field blur, the
Tilt-Shift smear, and the path gather first (when those effects are on),
then the spin gather over their result, then the effects: Bokeh weighted by
the radius field only, Noise by the per-pixel largest of the radius, path,
and spin weights. With no spins the output SHALL be bit-identical to the
kernel without them, and spins alone SHALL match `Filter::SpinBlur` with
the same strobe and noise.

#### Scenario: Spins alone match their own kernel

- **WHEN** the combined kernel runs with only spins
- **THEN** the output equals `Filter::SpinBlur`'s

#### Scenario: Spins add to a radius effect

- **WHEN** a Field pin and a spin are combined
- **THEN** the output differs from each effect alone
