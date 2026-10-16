## ADDED Requirements

### Requirement: Path blur (path-following motion blur)

The system SHALL implement `Filter::PathBlur { paths: Vec<BlurPath>, motion:
PathMotion, noise }` with `BlurPath { points, start_speed, end_speed }` and
`PathMotion { speed, taper, centered, rear_sync, strobe_strength,
strobe_flashes }` at the crate root, labelled non-parity (a post-CS6
effect, approximated). A path SHALL have 2..=`MAX_PATH_POINTS` (32) points
as fractions of the buffer (outside allowed), describing the Catmull-Rom
curve through them, and end-point speeds in buffer pixels in
`0..=MAX_END_POINT_SPEED` (500). `speed` SHALL be in `0..=MAX_PATH_SPEED`
(500 %), `taper` and `strobe_strength` in `0..=100`, and `strobe_flashes` in
`1..=100`; at most `MAX_FIELD_PINS` (16) paths SHALL be accepted. Each
pixel's motion vector SHALL be the inverse-square-distance blend of each
path's nearest-point unit tangent times the end speeds interpolated by arc
length, scaled by `speed / 100`; the pixel SHALL average taps along the
field's streamline over that length, centred on the pixel when `centered`
and behind it otherwise, with taper, strobe, and Rear Sync Flash weights.
A trail shorter than one pixel SHALL leave the pixel unchanged. The gather
SHALL run over the alpha plane too; Noise SHALL follow, weighted by the
trail length; the kernel SHALL treat an empty path list as a no-op and
SHALL reject non-finite or out-of-range values with
`FilterError::InvalidParams` before writing any pixel. Equal inputs SHALL
produce bit-identical output. Oracle expectation: no faithful ImageMagick
operator exists; the mapping row is no-equivalent with tolerance 0 and a
non-empty note.

#### Scenario: A horizontal path blurs along rows only

- **WHEN** one horizontal path is applied to a picture whose rows are each
  one colour, and to a picture of vertical stripes
- **THEN** the row picture is bit-identical and the stripes change

#### Scenario: Centered Blur off trails behind the motion

- **WHEN** an impulse is blurred by a rightward path with Centered Blur off
- **THEN** pixels to the impulse's right change and pixels to its left do
  not; with Centered Blur on both sides change

#### Scenario: Strobe, taper, and Rear Sync Flash shape the trail

- **WHEN** an impulse is blurred with Strobe Strength 100 and four flashes,
  with Taper 100, or with Rear Sync Flash
- **THEN** the trail shows separated ghosts, fades toward its far end, or
  keeps the impulse itself brightest, respectively

#### Scenario: A vertical path turns the blur

- **WHEN** a vertical path is applied to the row picture
- **THEN** the picture changes

#### Scenario: Invalid paths are rejected

- **WHEN** a path has one point, a non-finite coordinate, or an end speed
  above the maximum, or the motion values are out of range
- **THEN** the kernel returns `InvalidParams` and the buffer is unchanged

### Requirement: Combined kernel with paths

`Filter::BlurGallery` SHALL gain `path: Vec<BlurPath>` and `motion:
PathMotion`. With paths, the kernel SHALL run the radius-field blur and the
Tilt-Shift smear first (when those effects are on), then the path gather
over their result, then the effects: Bokeh weighted by the radius field
only, Noise by the per-pixel larger of the radius and path weights. With no
paths the output SHALL be bit-identical to the kernel without them, and
paths alone SHALL match `Filter::PathBlur` with the same noise.

#### Scenario: Paths alone match their own kernel

- **WHEN** the combined kernel runs with only paths
- **THEN** the output equals `Filter::PathBlur`'s

#### Scenario: Paths add to a radius effect

- **WHEN** a Field pin and a path are combined
- **THEN** the output differs from each effect alone
