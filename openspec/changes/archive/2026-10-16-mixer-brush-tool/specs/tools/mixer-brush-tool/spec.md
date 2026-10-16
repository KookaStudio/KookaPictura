## ADDED Requirements

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
