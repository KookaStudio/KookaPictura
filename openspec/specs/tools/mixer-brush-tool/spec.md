# tools/mixer-brush-tool Specification

## Purpose
The Mixer Brush engine: blend paint from a brush's reservoir along a stroke.

## Requirements

### Requirement: Mixer Brush engine

`pictura_paint::mixer` SHALL apply a Mixer Brush dab to a layer's pixels. The
pickup SHALL be the tip-weighted, alpha-premultiplied average colour under the
dab. On a dry canvas (Wet 0) the dab SHALL deposit the reservoir and the load
SHALL not run down; on a wet canvas the deposit SHALL mix reservoir and pickup
with a canvas share of `Mix + (1 − Mix) × (1 − remaining load)`, the reservoir
SHALL absorb the pickup at a rate proportional to Wet × Mix, and the load SHALL
decrease per distance painted. A dry brush with no load SHALL deposit nothing.
Under a transparency lock the dab SHALL keep every pixel's alpha and leave
transparent pixels untouched.

#### Scenario: Dry paints the reservoir

- **WHEN** a Wet 0 dab with a red reservoir lands on white
- **THEN** the centre pixel is the reservoir red

#### Scenario: A full mix is a pure smear

- **WHEN** a Wet 100 / Mix 100 brush loaded with red strokes over white
- **THEN** the stroke leaves white

#### Scenario: A wet brush drags colour

- **WHEN** a wet brush strokes from a blue band into white
- **THEN** blue appears past the band's edge, thinned toward white

### Requirement: Mixer Brush tool

The Mixer Brush SHALL paint a live per-dab stroke on the active pixel layer
and record exactly one "Mixer Brush Tool" history state per stroke that
changed pixels. The brush's paint SHALL outlive the stroke: after a stroke it
SHALL be emptied when Clean after stroke is on, else set to the foreground when
Load after stroke is on, else keep what the stroke left. Alt-click SHALL load
the brush from the image, choosing a foreground colour SHALL load it, and the
options bar's Load Brush / Clean Brush SHALL load or empty it. The options bar
SHALL offer the load swatch, the after-stroke toggles, a Wet/Load/Mix preset
menu that follows the fields (Custom when none matches), Wet, Load, Mix, and
Flow (0–100 %), with Load and Mix disabled while Wet is 0.

#### Scenario: A wet stroke smears and carries paint

- **WHEN** the `mixer_brush_tool` self-test strokes a Wet/Load/Mix 100 brush loaded with white from a black band into white
- **THEN** one "Mixer Brush Tool" state is recorded, grey appears past the band, and the brush's paint is no longer pure white

#### Scenario: Clean after stroke empties the brush

- **WHEN** a stroke ends with Clean after stroke on
- **THEN** the brush's paint is transparent

#### Scenario: A preset sets the fields

- **WHEN** the preset menu picks "Wet, Heavy Mix"
- **THEN** Wet, Load, and Mix become 50, 50, and 60 and Mix is enabled

### Requirement: Mixer Brush engine applies Mode and Opacity

The Mixer Brush engine SHALL composite its deposit through the stroke's paint mode and SHALL cap it by the stroke's Opacity, using the same blend and coverage rules as the Brush.

#### Scenario: Multiply mixes through the backdrop [mx_mode_multiply]

- **WHEN** a Mixer dab with Mode Multiply deposits a colour over an opaque base
- **THEN** the pixel becomes the channel-wise product of the deposit and the base, matching the shared Multiply paint mode

#### Scenario: Opacity caps the deposit [mx_opacity_cap]

- **WHEN** a Mixer dab runs at Opacity 50 % over an opaque base
- **THEN** a covered pixel moves half way from its base colour to the full-opacity result

### Requirement: Mixer Brush bar matches CC

The Mixer Brush options bar SHALL NOT show a Mode or Opacity control, and a Mixer Brush stroke SHALL run in Normal mode at 100 % Opacity regardless of the Brush's settings. The bar SHALL show an enabled Sample All Layers checkbox that writes the Mixer options.

#### Scenario: The Mixer bar matches CC [mx_bar_cc]

- **WHEN** the Mixer Brush tool is active
- **THEN** the options bar shows no Mode or Opacity control and shows an enabled Sample All Layers checkbox that writes the Mixer options

### Requirement: Mixer Brush Sample All Layers

With Sample All Layers on, the Mixer Brush pickup SHALL average the colour under the dab from the visible composite, snapshotted when the stroke begins, instead of the active layer. The deposit SHALL still be written to the active layer. With it off the pickup SHALL read the active layer.

#### Scenario: The composite supplies the pickup [mx_sample_all]

- **WHEN** a wet dab lands on a white layer while the composite under it is blue and Sample All Layers is on
- **THEN** the deposit and the brush's paint take on blue, whereas with it off the dab leaves the white layer white

### Requirement: Mixer Brush mixing is an approximation

Adobe's Mixer Brush mixing and dry-out math is closed. The engine SHALL be documented as behavioural parity only, naming its limits: a single flat load colour, a tip-weighted mean pickup, and photorust's rates and preset values. The Dry, Moist and Very Wet settings SHALL order by how much canvas colour a stroke carries.

#### Scenario: Presets order by wetness [mx_preset_order]

- **WHEN** the same red brush strokes from a blue band into white at Dry, Moist and Very Wet settings
- **THEN** Dry lays down pure red, Moist carries blue into the stroke, and Very Wet leaves the stroke with no red tint from the brush
