## MODIFIED Requirements

### Requirement: Additional brush presets

The default brush set SHALL include `2B Pencil` (10 px), `Airbrush Soft Round
50% flow` (45 px), `Non-Photo Blue Pencil` (9 px), and `Paintbrush Oval 45
pixels Multiply` (45 px) after the existing CS6 set. `2B Pencil` and
`Non-Photo Blue Pencil` SHALL be hard-edged, anti-aliased tips that set the
pen-tip preview model; both pencils SHALL paint with a fixed, deterministic
paper grain. `2B Pencil` SHALL use an 80 % grain that leaves roughly 60 % mean
coverage and forms 2–3 px clumps with fine per-pixel variation;
`Non-Photo Blue Pencil` SHALL use a 30 % grain. `Airbrush Soft Round 50% flow`
SHALL be a soft round tip with 25 % spacing, so its 50 % flow builds a light,
soft stroke instead of a solid core; `Paintbrush Oval 45 pixels Multiply` SHALL
be a flattened oval tip. Selecting one of these four presets SHALL set its tip
— size, hardness, roundness, angle, spacing, dab dynamics, grain, and the
pen-tip preview model — and its default paint settings: foreground colour,
Mode, Opacity, and Flow. `Non-Photo Blue Pencil` SHALL use foreground
`#73d6ff`; the other three SHALL use `#47484a`. `2B Pencil` and `Non-Photo
Blue Pencil` SHALL use Normal mode at 100 % opacity and 100 % flow; `Airbrush
Soft Round 50% flow` SHALL use Normal mode at 100 % opacity and 50 % flow;
`Paintbrush Oval 45 pixels Multiply` SHALL use Multiply mode at 35 % opacity
and 50 % flow. Selecting an existing CS6 preset SHALL still set only its tip
and SHALL NOT change the foreground colour, Mode, Opacity, or Flow. The exact
Photoshop tip data is not available; the settings are approximated from the
owner's screenshots and marked non-parity.

#### Scenario: The four additional presets are listed

- **WHEN** the Brush Preset picker is opened
- **THEN** the grid lists `2B Pencil`, `Airbrush Soft Round 50% flow`,
  `Non-Photo Blue Pencil`, and `Paintbrush Oval 45 pixels Multiply` after the
  existing presets

#### Scenario: The additional presets apply their paint settings

- **WHEN** each of the four additional presets is chosen after setting a
  different foreground colour, Mode, Opacity, and Flow
- **THEN** the preset's tip and its default colour, Mode, Opacity, and Flow are
  applied; `Non-Photo Blue Pencil` paints `#73d6ff`, the others `#47484a`, and
  `Paintbrush Oval 45 pixels Multiply` uses Multiply at 35 % opacity and 50 %
  flow

#### Scenario: The airbrush paints a light, soft stroke

- **WHEN** the Brush paints a stroke with `Airbrush Soft Round 50% flow`
- **THEN** the stroke's core stays below full opacity and is lighter than the
  same stroke at 10 % spacing

#### Scenario: A preset sets the tip only

- **WHEN** an existing CS6 preset is chosen after setting a non-default
  foreground colour, Mode, Opacity, and Flow
- **THEN** the tip changes and the foreground colour, Mode, Opacity, and Flow
  are unchanged

#### Scenario: The pencil presets set the pen-tip model

- **WHEN** `2B Pencil` or `Non-Photo Blue Pencil` is chosen
- **THEN** the pen-tip preview model is set; choosing another preset clears it

#### Scenario: The 2B Pencil paints a grainy stroke

- **WHEN** the Pencil tool paints a stroke with `2B Pencil`
- **THEN** the stroke's alpha is broken up by a fixed, clumpy paper grain; its
  mean coverage is about 60 %, adjacent grain pixels cluster at 2–3 px, and
  the same stroke renders the same every time
