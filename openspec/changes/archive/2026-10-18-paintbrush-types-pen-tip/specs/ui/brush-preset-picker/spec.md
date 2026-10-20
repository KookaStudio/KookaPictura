## ADDED Requirements

### Requirement: Additional brush presets

The default brush set SHALL include `2B Pencil` (10 px), `Airbrush Soft Round
50% flow` (45 px), `Non-Photo Blue Pencil` (9 px), and `Paintbrush Oval 45
pixels Multiply` (45 px) after the existing CS6 set. `2B Pencil` and
`Non-Photo Blue Pencil` SHALL be hard-edged, anti-aliased tips that set the
pen-tip preview model; `2B Pencil` SHALL paint with a fixed paper grain.
`Airbrush Soft Round 50% flow` SHALL be a soft round tip; `Paintbrush Oval 45
pixels Multiply` SHALL be a flattened oval tip. Selecting a preset SHALL set
only the tip — size, hardness, roundness, angle, spacing, dab dynamics, grain,
and the pen-tip preview model — and SHALL NOT change Mode, Opacity, Flow, or
the foreground colour. The exact Photoshop tip data is not available; the
settings are approximated from the owner's screenshots and marked non-parity.

#### Scenario: The four additional presets are listed

- **WHEN** the Brush Preset picker is opened
- **THEN** the grid lists `2B Pencil`, `Airbrush Soft Round 50% flow`,
  `Non-Photo Blue Pencil`, and `Paintbrush Oval 45 pixels Multiply` after the
  existing presets

#### Scenario: A preset sets the tip only

- **WHEN** `Paintbrush Oval 45 pixels Multiply` is chosen after setting Mode,
  Opacity, Flow, and the foreground colour to non-defaults
- **THEN** the tip becomes the oval preset's values and Mode, Opacity, Flow,
  and the foreground colour are unchanged

#### Scenario: The pencil presets set the pen-tip model

- **WHEN** `2B Pencil` or `Non-Photo Blue Pencil` is chosen
- **THEN** the pen-tip preview model is set; choosing another preset clears it

#### Scenario: The 2B Pencil paints a grainy stroke

- **WHEN** the Pencil tool paints a stroke with `2B Pencil`
- **THEN** the stroke's alpha is broken up by a fixed per-pixel grain and the
  same stroke renders the same every time
