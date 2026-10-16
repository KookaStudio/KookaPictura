# brush-stroke-filters Specification

## Purpose
The Brush Stroke filter family, from Accented Edges through Sumi-e, under the shared filter contract.

## Requirements

### Requirement: Brush Stroke filter application and error contract

The system SHALL implement the 8 CS6 Brush Stroke filters as `Filter` variants and
apply them through the shared `pictura_filters::apply(filter, buf)` entry point.
`apply` SHALL transform every color sample of a planar 8-bit buffer whose channel
count is 3 (RGB) or 4 (RGBA) and return `Ok(())`, or return a `FilterError`
without partially applying; malformed buffers MUST error instead of panicking.

#### Scenario: Apply a Brush Stroke filter to a color buffer

- **WHEN** `apply` receives a Brush Stroke variant and a 3- or 4-channel planar buffer
- **THEN** the color planes are rewritten in place and `Ok(())` is returned

#### Scenario: Malformed buffers error

- **WHEN** the buffer is empty, has zero width or height, or an inconsistent length
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Alpha channel preservation

For 4-channel buffers, every Brush Stroke filter SHALL leave channel 4 bit-identical.

#### Scenario: Every Brush Stroke variant preserves alpha

- **WHEN** each of the 8 Brush Stroke variants is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Parameter validation

The system SHALL validate each Brush Stroke filter's parameters before writing any
sample and SHALL reject out-of-range or non-finite values with
`FilterError::InvalidParams` without panicking. The accepted ranges SHALL be:
Accented Edges `edge_width 1..=14`, `edge_brightness 0..=50`, `smoothness 1..=15`;
Angled Strokes `direction_balance 0..=100`, `stroke_length 3..=50`,
`sharpness 0..=10`; Crosshatch `stroke_length 3..=50`, `sharpness 0..=20`,
`strength 1..=3`; Dark Strokes `balance 0..=10`, `black_intensity 0..=10`,
`white_intensity 0..=10`; Ink Outlines `stroke_length 1..=50`,
`dark_intensity 0..=50`, `light_intensity 0..=50`; Spatter `spray_radius 0..=25`,
`smoothness 1..=15`; Sprayed Strokes `stroke_length 0..=20`,
`spray_radius 0..=25`; Sumi-e `stroke_width 3..=15`, `stroke_pressure 0..=15`,
`contrast 0..=40`.

#### Scenario: Out-of-range parameters are rejected

- **WHEN** a Brush Stroke filter is applied with a parameter outside its range
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: Boundary values are accepted

- **WHEN** each Brush Stroke filter is applied at the minimum and maximum of its ranges
- **THEN** each returns `Ok(())` without a panic

### Requirement: Determinism and seeding

Every randomised Brush Stroke filter SHALL take a `seed: u64` and SHALL be
deterministic: the same input, parameters, and seed SHALL produce bit-identical
output.

#### Scenario: Repeat applies match exactly

- **WHEN** a stochastic Brush Stroke filter is applied twice with the same seed
- **THEN** the two outputs are bit-identical

#### Scenario: Different seeds differ

- **WHEN** a stochastic Brush Stroke filter is applied with two different seeds
- **THEN** at least one output sample differs

### Requirement: Accented Edges

The system SHALL implement `Filter::AccentedEdges { edge_width, edge_brightness, smoothness }` as edge accentuation: edges are traced into lines from the smoothed brightness and laid over the image dark when `edge_brightness` is below 25 and light (chalk) when it is above, harder the further from 25; at 25 the edges SHALL NOT be accented.

#### Scenario: Edge brightness polarity

- **WHEN** Accented Edges is applied to an edged image at `edge_brightness` 0 and at 50
- **THEN** the 0 output is darker near the edge than the input and the 50 output is lighter near the edge than the input

#### Scenario: Edge width retains a detectable edge

- **WHEN** Accented Edges is applied at `edge_width` 1 and at 14
- **THEN** both outputs preserve a detectable luminance change across the source edge

#### Scenario: Edge brightness 25 is neutral

- **WHEN** Accented Edges is applied to an edged image at `edge_brightness` 0, 25, and 50
- **THEN** at the edge the 25 output is lighter than the 0 output and darker than the 50 output

### Requirement: Angled Strokes

The system SHALL implement `Filter::AngledStrokes { direction_balance, stroke_length, sharpness }` as diagonal strokes whose two opposing directions are weighted by `direction_balance`.

#### Scenario: Direction balance changes the result

- **WHEN** Angled Strokes is applied at `direction_balance` 0 and at 100
- **THEN** the two outputs differ

#### Scenario: Stroke length changes the result

- **WHEN** Angled Strokes is applied at `stroke_length` 3 and at 50
- **THEN** the two outputs differ

### Requirement: Crosshatch

The system SHALL implement `Filter::Crosshatch { stroke_length, sharpness, strength }` as a detail-preserving pencil-hatching overlay whose number of hatching passes equals `strength`.

#### Scenario: Strength increases hatching passes

- **WHEN** Crosshatch is applied at `strength` 1 and at 3
- **THEN** the 3 output exhibits more hatching passes than the 1 output

#### Scenario: Preserves detail

- **WHEN** Crosshatch is applied to an image with a strong edge
- **THEN** the output preserves a detectable luminance change across that edge

### Requirement: Dark Strokes

The system SHALL implement `Filter::DarkStrokes { balance, black_intensity, white_intensity }` as short dark strokes in shadows and long light strokes in highlights, where raising `balance` increases the proportion of dark strokes.

#### Scenario: Balance raises the dark proportion

- **WHEN** Dark Strokes is applied at `balance` 0 and at 10
- **THEN** the 10 output contains a greater proportion of dark strokes

#### Scenario: Intensities act independently

- **WHEN** Dark Strokes is applied with `black_intensity` raised alone and with `white_intensity` raised alone
- **THEN** each change deepens its own dark or light region without cancelling the other

### Requirement: Ink Outlines

The system SHALL implement `Filter::InkOutlines { stroke_length, dark_intensity, light_intensity }` as fine narrow pen-and-ink lines over the original detail.

#### Scenario: Line detail is retained

- **WHEN** Ink Outlines is applied to an image with fine lines
- **THEN** the output preserves a detectable luminance change across those lines

#### Scenario: Intensities strengthen their components

- **WHEN** Ink Outlines is applied at `dark_intensity` 0 and 50 and at `light_intensity` 0 and 50
- **THEN** each intensity strengthens its respective dark or light line component

### Requirement: Spatter

The system SHALL implement `Filter::Spatter { spray_radius, smoothness, seed }` as a seeded airbrush scatter whose spots spread over a radius that grows with `spray_radius` and merge as `smoothness` rises.

#### Scenario: Larger spray radius scatters wider

- **WHEN** Spatter is applied at `spray_radius` 0 and at 25
- **THEN** the number of samples changed from the input is greater at 25

#### Scenario: Seed reproduces the result

- **WHEN** Spatter is applied twice with the same parameters and seed
- **THEN** the outputs are bit-identical

### Requirement: Sprayed Strokes

The system SHALL implement `Filter::SprayedStrokes { stroke_length, spray_radius, direction, seed }` as seeded angled strokes whose direction is one of `StrokeDirection { RightDiagonal, Horizontal, LeftDiagonal, Vertical }`.

#### Scenario: Direction changes the result

- **WHEN** Sprayed Strokes is applied with two different `direction` values
- **THEN** the two outputs differ

#### Scenario: Larger spray radius scatters wider

- **WHEN** Sprayed Strokes is applied at `spray_radius` 0 and at 25
- **THEN** the number of samples changed from the input is greater at 25

### Requirement: Sumi-e

The system SHALL implement `Filter::SumiE { stroke_width, stroke_pressure, contrast }` as Japanese-ink strokes with soft blurred edges and rich blacks, where `stroke_width` sets the brush width.

#### Scenario: Non-empty effect with valid range

- **WHEN** Sumi-e is applied to a textured test image
- **THEN** the output differs from the input and every sample stays in `0..=255`

#### Scenario: Stroke width changes the result

- **WHEN** Sumi-e is applied at `stroke_width` 3 and at 15
- **THEN** the two outputs differ
