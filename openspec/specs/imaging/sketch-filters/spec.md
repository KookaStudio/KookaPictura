# sketch-filters Specification

## Purpose
The 14 Sketch filters, Bas Relief to Water Paper, with foreground and background inputs, texture options, and the shared contract.

## Requirements

### Requirement: Sketch filter application and error contract

The system SHALL implement the 14 CS6 Sketch filters as `Filter` variants and
apply them through the shared `pictura_filters::apply(filter, buf)` entry point.
`apply` SHALL transform every color sample of a planar 8-bit buffer whose channel
count is 3 (RGB) or 4 (RGBA) and return `Ok(())`, or return a `FilterError`
without partially applying; malformed buffers MUST error instead of panicking.

#### Scenario: Apply a Sketch filter to a color buffer

- **WHEN** `apply` receives a Sketch variant and a 3- or 4-channel planar buffer
- **THEN** the color planes are rewritten in place and `Ok(())` is returned

#### Scenario: Malformed buffers error

- **WHEN** the buffer is empty, has zero width or height, or an inconsistent length
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Alpha channel preservation

For 4-channel buffers, every Sketch filter SHALL leave channel 4 bit-identical.

#### Scenario: Every Sketch variant preserves alpha

- **WHEN** each of the 14 Sketch variants is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Parameter validation

The system SHALL validate each Sketch filter's parameters before writing any
sample and SHALL reject out-of-range or non-finite values with
`FilterError::InvalidParams` without panicking. The accepted ranges SHALL be:
Bas Relief `detail 1..=15`, `smoothness 1..=15`, `foreground`/`background` RGB;
Chalk & Charcoal
`charcoal_area 0..=20`, `chalk_area 0..=20`, `stroke_pressure 0..=5`; Charcoal
`thickness 1..=7`, `detail 0..=5`, `light_dark_balance 0..=100`; Chrome
`detail 0..=10`, `smoothness 0..=10`; Conté Crayon `foreground_level 1..=15`,
`background_level 1..=15`; Graphic Pen `stroke_length 1..=15`,
`light_dark_balance 0..=100`; Halftone Pattern `size 1..=12`, `contrast 0..=50`;
Note Paper `image_balance 0..=50`, `graininess 0..=20`, `relief 0..=25`;
Photocopy `detail 0..=24`, `darkness 1..=50`; Plaster `image_balance 0..=50`,
`smoothness 0..=15`; Reticulation `density 0..=50`, `black_level 0..=50`,
`white_level 0..=50`; Stamp `light_dark_balance 0..=50`, `smoothness 1..=50`;
Torn Edges `image_balance 0..=50`, `smoothness 1..=15`, `contrast 1..=25`;
Water Paper `fiber_length 3..=50`, `brightness 0..=100`, `contrast 0..=100`.

#### Scenario: Out-of-range parameters are rejected

- **WHEN** a Sketch filter is applied with a parameter outside its range
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: Boundary values are accepted

- **WHEN** each Sketch filter is applied at the minimum and maximum of its ranges
- **THEN** each returns `Ok(())` without a panic

### Requirement: Determinism and seeding

Every randomised Sketch filter SHALL take a `seed: u64` and SHALL be
deterministic: the same input, parameters, and seed SHALL produce bit-identical
output.

#### Scenario: Repeat applies match exactly

- **WHEN** a stochastic Sketch filter is applied twice with the same seed
- **THEN** the two outputs are bit-identical

#### Scenario: Different seeds differ

- **WHEN** a stochastic Sketch filter is applied with two different seeds
- **THEN** at least one output sample differs

### Requirement: Foreground and background colour inputs

The colour-dependent Sketch filters SHALL take their colours as explicit
`foreground` and `background` RGB parameters: Bas Relief, Chalk & Charcoal,
Charcoal, Conté Crayon, Graphic Pen, Plaster, Reticulation, Stamp, and Torn
Edges. Equal foreground and background SHALL NOT panic or divide by zero.

#### Scenario: Foreground and background colours are honoured

- **WHEN** a colour-dependent Sketch filter is applied with two different foreground/background pairs
- **THEN** the resulting output differs between the two runs

#### Scenario: Equal foreground and background are safe

- **WHEN** a colour-dependent Sketch filter is applied with `foreground` equal to `background`
- **THEN** `apply` returns `Ok(())` without panicking or dividing by zero

### Requirement: Texture options

Conté Crayon SHALL accept shared `TextureOptions`: a surface preset (Brick,
Burlap, Canvas, Sandstone), `scaling` in `50..=200`, `relief`, one of eight
`light_direction` values, and `invert`. The surface SHALL be generated
procedurally and lit by the chosen direction; the kernel MUST NOT materialize a
canvas-sized texture.

#### Scenario: Texture options change the result

- **WHEN** Conté Crayon is applied with two different surface presets or light directions
- **THEN** the two outputs differ

### Requirement: Bas Relief

The system SHALL implement `Filter::BasRelief { detail, smoothness, light_direction, foreground, background }` as a low relief: the image's luminance, softened by `smoothness`, is read as a height field lit from `light_direction`; a slope facing the light SHALL be driven toward the `background` colour and a slope facing away toward the `foreground` colour, by a gain that grows with `detail`, and a flat area, which catches no light, SHALL sit at the midpoint of the two colours.

#### Scenario: Flat areas sit at the midpoint

- **WHEN** Bas Relief is applied to an image with broad flat regions
- **THEN** the flat regions take the midpoint of the `foreground` and `background` colours within 2 levels

#### Scenario: Dark and light areas take foreground and background

- **WHEN** Bas Relief is applied across a strong step between a dark and a light region
- **THEN** the pixels at the step move away from the midpoint, the slope facing the light toward the `background` colour and the slope facing away toward the `foreground` colour

#### Scenario: Light direction moves the illumination

- **WHEN** Bas Relief is applied with two different `light_direction` values
- **THEN** the two outputs differ

### Requirement: Chalk & Charcoal

The system SHALL implement `Filter::ChalkCharcoal { charcoal_area, chalk_area, stroke_pressure, foreground, background, seed }` as coarse chalk over a solid midtone background with black diagonal charcoal shadows, where `charcoal_area` and `chalk_area` independently control the shadow and midtone coverage.

#### Scenario: Charcoal and chalk areas are independent

- **WHEN** Chalk & Charcoal is applied with only `charcoal_area` changed and with only `chalk_area` changed
- **THEN** each change alters its own region without cancelling the other

#### Scenario: Seed reproduces the result

- **WHEN** Chalk & Charcoal is applied twice with the same parameters and seed
- **THEN** the outputs are bit-identical

### Requirement: Charcoal

The system SHALL implement `Filter::Charcoal { thickness, detail, light_dark_balance, foreground, background, seed }` as a posterized, smudged charcoal sketch whose bold edges grow with `thickness` and whose tonal weighting is set by `light_dark_balance`.

#### Scenario: Non-empty effect with valid range

- **WHEN** Charcoal is applied to a textured test image
- **THEN** the output differs from the input and every sample stays in `0..=255`

#### Scenario: Thickness changes the result

- **WHEN** Charcoal is applied at `thickness` 1 and at 7
- **THEN** the two outputs differ

### Requirement: Chrome

The system SHALL implement `Filter::Chrome { detail, smoothness }` as a polished-chrome rendering where highlights read as high points and shadows as low points.

#### Scenario: Non-empty effect

- **WHEN** Chrome is applied to a textured test image
- **THEN** the output differs from the input

#### Scenario: Smoothness changes the result

- **WHEN** Chrome is applied at `smoothness` 0 and at 10
- **THEN** the two outputs differ

### Requirement: Conté Crayon

The system SHALL implement `Filter::ConteCrayon { foreground_level, background_level, texture, foreground, background, seed }` as a dense Conté crayon texture whose dark and light area emphasis is set by `foreground_level` and `background_level` over the shared `TextureOptions`.

#### Scenario: Foreground and background levels are independent

- **WHEN** Conté Crayon is applied with only `foreground_level` changed and with only `background_level` changed
- **THEN** each change alters its own tonal region

#### Scenario: Texture options change the result

- **WHEN** Conté Crayon is applied with two different `TextureOptions`
- **THEN** the two outputs differ

### Requirement: Graphic Pen

The system SHALL implement `Filter::GraphicPen { stroke_length, light_dark_balance, direction, foreground, background }` as fine linear ink strokes in the foreground colour over the background, with stroke direction one of `StrokeDirection { RightDiagonal, Horizontal, LeftDiagonal, Vertical }`.

#### Scenario: Direction changes the result

- **WHEN** Graphic Pen is applied with two different `direction` values
- **THEN** the two outputs differ

#### Scenario: Non-empty effect

- **WHEN** Graphic Pen is applied to a textured test image
- **THEN** the output differs from the input

### Requirement: Halftone Pattern

The system SHALL implement `Filter::HalftonePattern { size, contrast, pattern }` as a halftone screen whose cell shape is one of `HalftoneType { Dot, Line, Circle }`, whose cell size grows with `size`, and whose threshold is set by `contrast`.

#### Scenario: Patterns are distinguishable

- **WHEN** Halftone Pattern is applied with `pattern` Dot, Line, and Circle
- **THEN** the three outputs are pairwise distinct

#### Scenario: Size changes the cell

- **WHEN** Halftone Pattern is applied at `size` 1 and at 12
- **THEN** the 12 output has larger same-tone cells than the 1 output

### Requirement: Note Paper

The system SHALL implement `Filter::NotePaper { image_balance, graininess, relief, seed }` as an emboss plus grain surface, where `relief` changes apparent depth and `graininess` changes surface noise.

#### Scenario: Relief and grain act independently

- **WHEN** Note Paper is applied with only `relief` changed and with only `graininess` changed
- **THEN** each change alters the output while the other control is held constant

#### Scenario: Seed reproduces the result

- **WHEN** Note Paper is applied twice with the same parameters and seed
- **THEN** the outputs are bit-identical

### Requirement: Photocopy

The system SHALL implement `Filter::Photocopy { detail, darkness }` as a high-contrast photocopy in which large dark areas are reduced to edge-only marks and midtones collapse toward black or white.

#### Scenario: Large dark areas become edge-only

- **WHEN** Photocopy is applied to an image containing a large dark region
- **THEN** the output marks only the boundary of that region and leaves its interior near white

#### Scenario: Darkness changes the result

- **WHEN** Photocopy is applied at `darkness` 1 and at 50
- **THEN** the two outputs differ

### Requirement: Plaster

The system SHALL implement `Filter::Plaster { image_balance, smoothness, light_direction, foreground, background }` as a moulded-plaster relief colourised with the foreground and background colours, where dark areas are raised and light areas recessed.

#### Scenario: Light direction moves the illumination

- **WHEN** Plaster is applied with two different `light_direction` values
- **THEN** the two outputs differ

#### Scenario: Non-empty effect

- **WHEN** Plaster is applied to a textured test image
- **THEN** the output differs from the input

### Requirement: Reticulation

The system SHALL implement `Filter::Reticulation { density, black_level, white_level, foreground, background, seed }` as clumped shadows and lightly grained highlights, where `density` sets the clump density and `black_level` and `white_level` set the shadow and highlight levels.

#### Scenario: Density changes the clumping

- **WHEN** Reticulation is applied at `density` 0 and at 50
- **THEN** the two outputs differ

#### Scenario: Seed reproduces the result

- **WHEN** Reticulation is applied twice with the same parameters and seed
- **THEN** the outputs are bit-identical

### Requirement: Stamp

The system SHALL implement `Filter::Stamp { light_dark_balance, smoothness, foreground, background }` as a rubber/wood-stamp simplification of the image rendered in the foreground and background colours.

#### Scenario: Non-empty effect

- **WHEN** Stamp is applied to a textured test image
- **THEN** the output differs from the input

#### Scenario: Light/dark balance changes the result

- **WHEN** Stamp is applied at `light_dark_balance` 0 and at 50
- **THEN** the two outputs differ

### Requirement: Torn Edges

The system SHALL implement `Filter::TornEdges { image_balance, smoothness, contrast, foreground, background }` as a ragged torn-paper reconstruction colourised with the foreground and background colours.

#### Scenario: Non-empty effect

- **WHEN** Torn Edges is applied to a high-contrast test image
- **THEN** the output differs from the input

#### Scenario: Contrast changes the result

- **WHEN** Torn Edges is applied at `contrast` 1 and at 25
- **THEN** the two outputs differ

### Requirement: Water Paper

The system SHALL implement `Filter::WaterPaper { fiber_length, brightness, contrast, seed }` as blotchy daubs on fibrous damp paper where `fiber_length` changes the fibre scale and `brightness` and `contrast` shift the wash.

#### Scenario: Non-empty effect

- **WHEN** Water Paper is applied to a textured test image
- **THEN** the output differs from the input

#### Scenario: Seed reproduces the result

- **WHEN** Water Paper is applied twice with the same parameters and seed
- **THEN** the outputs are bit-identical
