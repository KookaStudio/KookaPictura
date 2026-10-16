# artistic-filters Specification

## Purpose
The CS6 Artistic filter family, from Colored Pencil to Watercolor, under a shared error, alpha, parameter, and seeding contract.

## Requirements

### Requirement: Artistic filter application and error contract

The system SHALL implement the 15 CS6 Artistic filters as `Filter` variants and
apply them through the shared `pictura_filters::apply(filter, buf)` entry point.
`apply` SHALL transform every color sample of a planar 8-bit buffer whose channel
count is 3 (RGB) or 4 (RGBA) and return `Ok(())`, or return a `FilterError`
without partially applying; malformed buffers MUST error instead of panicking.

#### Scenario: Apply an Artistic filter to a color buffer

- **WHEN** `apply` receives an Artistic variant and a 3- or 4-channel planar buffer
- **THEN** the color planes are rewritten in place and `Ok(())` is returned

#### Scenario: Malformed buffers error

- **WHEN** the buffer is empty, has zero width or height, or an inconsistent length
- **THEN** `apply` returns a `FilterError` and does not panic

### Requirement: Alpha channel preservation

For 4-channel buffers, every Artistic filter SHALL leave channel 4 bit-identical.

#### Scenario: Every Artistic variant preserves alpha

- **WHEN** each of the 15 Artistic variants is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

### Requirement: Parameter validation

The system SHALL validate each Artistic filter's parameters before writing any
sample and SHALL reject out-of-range or non-finite values with
`FilterError::InvalidParams` without panicking. The accepted ranges SHALL be:
Colored Pencil `pencil_width 1..=24`, `stroke_pressure 0..=15`,
`paper_brightness 0..=50`; Cutout `levels 2..=8`, `edge_simplicity 0..=10`,
`edge_fidelity 1..=3`; Dry Brush/Fresco `brush_size 0..=10`,
`brush_detail 0..=10`, `texture 1..=3`; Film Grain `grain 0..=20`,
`highlight_area 0..=20`, `intensity 0..=10`; Neon Glow
`glow_size -24..=24`, `glow_brightness 0..=50`; Paint Daubs
`brush_size 1..=50`, `sharpness 0..=40`; Palette Knife `stroke_size 1..=50`,
`stroke_detail 1..=3`, `softness 0..=10`; Plastic Wrap
`highlight_strength 0..=20`, `detail 1..=15`, `smoothness 1..=15`; Poster Edges
`edge_thickness 0..=10`, `edge_intensity 0..=10`, `posterization 0..=10`;
Rough Pastels `stroke_length 0..=40`, `stroke_detail 1..=20`;
Smudge Stick `stroke_length 0..=10`, `highlight_area 0..=20`,
`intensity 0..=10`; Sponge `brush_size 0..=10`, `definition 0..=25`,
`smoothness 1..=15`; Underpainting `brush_size 0..=40`,
`texture_coverage 0..=40`; Watercolor `brush_detail 1..=14`,
`shadow_intensity 0..=10`, `texture 1..=3`. Texture options SHALL require
`scaling 50..=200`. Every value inside these ranges SHALL reach the filter
unchanged, and each dialog's sliders SHALL run over the same ranges.

#### Scenario: Out-of-range parameters are rejected

- **WHEN** an Artistic filter is applied with a parameter outside its range
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: Boundary values are accepted

- **WHEN** each Artistic filter is applied at the minimum and maximum of its ranges
- **THEN** each returns `Ok(())` without a panic

#### Scenario: Dry Brush runs over CS6's slider range

- **WHEN** Dry Brush or Fresco is applied at `brush_size` 0 and `brush_detail` 0, and at `brush_size` 11
- **THEN** the first returns `Ok(())` and the second returns `FilterError::InvalidParams`

### Requirement: Determinism and seeding

Every Artistic filter that uses randomness SHALL take a `seed: u64` and SHALL be
deterministic: the same input, parameters, and seed SHALL produce bit-identical
output.

#### Scenario: Repeat applies match exactly

- **WHEN** a stochastic Artistic filter is applied twice with the same seed
- **THEN** the two outputs are bit-identical

#### Scenario: Different seeds differ

- **WHEN** a stochastic Artistic filter is applied with two different seeds
- **THEN** at least one output sample differs

### Requirement: Foreground and background colour inputs

The colour-dependent filters SHALL take their colours as explicit parameters:
Colored Pencil SHALL accept a background RGB, the paper that shows through its
smooth areas, and Neon Glow SHALL accept a glow RGB. Rough Pastels,
Underpainting, and Watercolor SHALL take no foreground or background colour,
and Colored Pencil no foreground colour. A background equal to the picture's
own tones SHALL NOT panic or divide by zero.

#### Scenario: Background colour shows through Colored Pencil

- **WHEN** Colored Pencil is applied to a flat region with two different background colours
- **THEN** the resulting flat-region colour differs between the two runs

### Requirement: Texture options

Rough Pastels and Underpainting SHALL accept shared texture options: a surface
preset (Brick, Burlap, Canvas, Sandstone), `scaling`, `relief`, one of eight
`light_direction` values, and `invert`. The surface SHALL be generated
procedurally and lit by the chosen direction; the kernel MUST NOT materialize a
canvas-sized texture.

#### Scenario: Texture options change the result

- **WHEN** Rough Pastels is applied with two different surface presets or light directions
- **THEN** the two outputs differ

### Requirement: Colored Pencil

The system SHALL implement `Filter::ColoredPencil { pencil_width, stroke_pressure, paper_brightness, background, seed }` as an edge-preserving colour reduction with crosshatch strokes and the background colour visible in smooth areas.

#### Scenario: Edges are retained

- **WHEN** Colored Pencil is applied to an image with a strong edge
- **THEN** the output preserves a detectable luminance change across that edge

### Requirement: Cutout

The system SHALL implement `Filter::Cutout { levels, edge_simplicity, edge_fidelity }` as a posterizing quantizer whose number of flat colour levels equals `levels` and whose contour smoothing increases with `edge_simplicity`.

#### Scenario: Raising levels adds colour bands

- **WHEN** Cutout is applied to a gradient at 2 levels and at 8 levels
- **THEN** the 8-level output contains strictly more distinct colour values

#### Scenario: Deterministic

- **WHEN** Cutout is applied twice with identical parameters
- **THEN** the outputs match bit-for-bit

### Requirement: Dry Brush

The system SHALL implement `Filter::DryBrush { brush_size, brush_detail, texture, seed }` as a colour-range reduction with a dry-looking directional stroke reconstruction.

#### Scenario: Produces a non-empty effect

- **WHEN** Dry Brush is applied to a textured test image
- **THEN** the output differs from the input and remains within the valid sample range

### Requirement: Film Grain

The system SHALL implement `Filter::FilmGrain { grain, highlight_area, intensity, seed }` as a tonal-zone noise field, stronger in shadows and midtones, with a smoother brightening laid over the tones above the `highlight_area` line, reproducible from its seed.

#### Scenario: Grain vanishes at zero

- **WHEN** Film Grain is applied with `grain` 0 and `highlight_area` 0
- **THEN** the output equals the input

#### Scenario: Grain is deterministic

- **WHEN** Film Grain is applied twice with the same seed and parameters
- **THEN** the outputs match bit-for-bit

### Requirement: Fresco

The system SHALL implement `Filter::Fresco { brush_size, brush_detail, texture, seed }` as coarse short daubs.

#### Scenario: Non-empty effect with valid range

- **WHEN** Fresco is applied to a textured test image
- **THEN** the output differs from the input and every sample stays in `0..=255`

### Requirement: Neon Glow

The system SHALL implement `Filter::NeonGlow { glow_size, glow_brightness, glow_color }` as a luminance-driven glow tinted by `glow_color`, where the glow extent grows with `glow_size` and a negative `glow_size` confines the glow to shadows.

#### Scenario: Glow colour appears in the output

- **WHEN** Neon Glow is applied with a strongly saturated glow colour
- **THEN** the output contains samples shifted toward that hue

#### Scenario: Glow extent grows

- **WHEN** Neon Glow is applied at a small and a large positive `glow_size`
- **THEN** the number of samples changed from the input is greater at the larger size

### Requirement: Paint Daubs

The system SHALL implement `Filter::PaintDaubs { brush_size, sharpness, brush_type, seed }` with `brush_type` in Simple, Light Rough, Dark Rough, Wide Sharp, Wide Blurry, Sparkle, and daubs that widen with `brush_size`. The `seed` SHALL re-roll the brush types that carry randomness (Sparkle). Sparkle SHALL draw thin lines of light along the boundaries between bands of brightness, with a glow round them, and SHALL NOT draw dark lines.

#### Scenario: Brush types differ

- **WHEN** Paint Daubs is applied with two different brush types
- **THEN** the outputs differ

#### Scenario: Size scales the strokes

- **WHEN** Paint Daubs is applied to a ramp at `brush_size` 1 and 50
- **THEN** the 50 output keeps fewer distinct tones than the 1 output

#### Scenario: Sparkle draws lines of light

- **WHEN** Paint Daubs is applied to a slow ramp with the Simple and the Sparkle brush
- **THEN** the Sparkle output has pixels well above the Simple output and none noticeably below it

### Requirement: Palette Knife

The system SHALL implement `Filter::PaletteKnife { stroke_size, stroke_detail, softness }` as Kuwahara smoothing with a reach of a third of `stroke_size`, then half of each channel rounded onto a palette of `2 + 3 * stroke_detail` levels, then a Gaussian blur of `softness / 2` pixels. The filter is deterministic and carries no seed.

#### Scenario: Non-empty effect

- **WHEN** Palette Knife is applied to a textured test image
- **THEN** the output differs from the input

#### Scenario: Flattens a surface and stops at a boundary

- **WHEN** Palette Knife is applied to two noisy fields that meet at a hard edge
- **THEN** each field comes back flatter and the edge stays hard

#### Scenario: Stroke Detail is the palette

- **WHEN** Palette Knife is applied to a flat 150 grey at `stroke_detail` 1 and 3
- **THEN** the outputs are 139 and 152

### Requirement: Plastic Wrap

The system SHALL implement `Filter::PlasticWrap { highlight_strength, detail, smoothness }` as a smoothing pass with highlight edges whose strength grows with `highlight_strength`.

#### Scenario: Highlights strengthen

- **WHEN** Plastic Wrap is applied at highlight strength 0 and 20
- **THEN** the output at 20 has greater maximum luminance than at 0

### Requirement: Poster Edges

The system SHALL implement `Filter::PosterEdges { edge_thickness, edge_intensity, posterization }`, producing posterized flat areas bounded by dark lines whose thickness grows with `edge_thickness`.

#### Scenario: Flat areas and dark lines

- **WHEN** Poster Edges is applied to a gradient with an edge
- **THEN** the output has fewer distinct interior levels than the input and contains samples darker than the input near the edge

### Requirement: Rough Pastels

The system SHALL implement `Filter::RoughPastels { stroke_length, stroke_detail, texture: TextureOptions, seed }` as textured chalk strokes over the paper surface.

#### Scenario: Texture affects the output

- **WHEN** Rough Pastels is applied with two different `TextureOptions`
- **THEN** the outputs differ

### Requirement: Smudge Stick

The system SHALL implement `Filter::SmudgeStick { stroke_length, highlight_area, intensity, seed }` as short diagonal smearing strokes.

#### Scenario: Non-empty effect

- **WHEN** Smudge Stick is applied to a textured test image
- **THEN** the output differs from the input

### Requirement: Sponge

The system SHALL implement `Filter::Sponge { brush_size, definition, smoothness, seed }` as contrasting sponge daubs.

#### Scenario: Non-empty effect

- **WHEN** Sponge is applied to a textured test image
- **THEN** the output differs from the input

### Requirement: Underpainting

The system SHALL implement `Filter::Underpainting { brush_size, texture_coverage, texture: TextureOptions, seed }` as an underpainting over the texture surface with the image painted on top.

#### Scenario: Texture coverage changes the result

- **WHEN** Underpainting is applied at texture coverage 0 and 40
- **THEN** the two outputs differ

### Requirement: Watercolor

The system SHALL implement `Filter::Watercolor { brush_detail, shadow_intensity, texture, seed }` as a soft colour-simplifying watercolour effect that saturates colour at significant tonal edges.

#### Scenario: Non-empty effect

- **WHEN** Watercolor is applied to a textured test image
- **THEN** the output differs from the input
