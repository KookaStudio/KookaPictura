## ADDED Requirements

### Requirement: Color Replacement engine

`pictura_paint::replace` SHALL apply a Color Replacement dab to a layer's
pixels: a pixel under the brush tip SHALL be replaced only when its largest
per-channel difference from the reference colour is within the tolerance
(0–255), fading linearly from 70 % of the tolerance to zero when anti-aliased.
The reference SHALL be the pixel under the dab centre (Continuous), the first
dab's centre pixel (Once), or the background colour (Background Swatch).
Contiguous and Find Edges SHALL replace only pixels connected to the dab centre
through matching pixels, and Find Edges SHALL NOT cross a normalised luminance
step above 0.35. The replacement SHALL blend the foreground by the W3C Hue,
Saturation, Color, or Luminosity formula, keep the pixel's alpha, never paint a
transparent pixel, and fully replace a pixel at most once per stroke.

#### Scenario: Color keeps the shading

- **WHEN** a Color-mode dab replaces a grey ramp with red
- **THEN** both ends turn red and the darker end stays darker

#### Scenario: Contiguous stops at a gap

- **WHEN** a dab straddles two matching stripes separated by a non-matching one
- **THEN** Contiguous leaves the far stripe unchanged and Discontiguous replaces it

#### Scenario: Once keeps the first sample

- **WHEN** a Once stroke starts on green and moves onto red
- **THEN** the green is replaced and the red is unchanged

### Requirement: Per-dab strokes

`pictura_paint::Stroke::begin_kind` SHALL start a stroke whose dabs either
accumulate paint coverage (`StrokeKind::Paint`, the Brush) or edit the
working layer in place at every dab (`StrokeKind::Replace`,
`StrokeKind::Mixer`). A per-dab stroke SHALL report each dab's dirty rectangle
for the live preview, SHALL leave the base document untouched until it
finishes, and SHALL commit its working document once.

#### Scenario: A replace stroke commits once

- **WHEN** a Replace stroke recolours a grey layer and finishes
- **THEN** the live document shows the recolouring, the outcome carries it, and the base document is unchanged

### Requirement: Color Replacement tool

The Color Replacement tool SHALL paint a live per-dab stroke on the active
pixel layer and record exactly one "Color Replacement Tool" history state per
stroke that changed pixels. Alt-click SHALL sample the foreground colour. The
options bar SHALL offer Size, Hardness, Mode (default Color), Sampling
(default Continuous), Limits (default Contiguous), Tolerance (0–100 %, default
30 %), and Anti-alias (default on).

#### Scenario: Repaint a field

- **WHEN** the `color_replacement_tool` self-test drags a green brush across a blue field beside a yellow one
- **THEN** one "Color Replacement Tool" state is recorded, the stroked blue turns green, and the yellow is unchanged
