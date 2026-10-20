# ui/brush-preset-picker Specification

## Purpose
The brush tip button and preset picker in the options bar.

## Requirements

### Requirement: Brush tip button

Every brush tool's options bar SHALL show a tip button, drawn by the engine
with the brush size under it, in place of separate Size and Hardness fields,
and a click on it SHALL open the Brush Preset picker below it. The button SHALL
follow size and tip changes made elsewhere.

#### Scenario: Opening the picker from the Eraser

- **WHEN** the `brush_preset_picker` self-test clicks the Eraser bar's tip button
- **THEN** the picker is shown with the full default brush set

### Requirement: Brush Preset picker

The picker SHALL offer Size (1–5000 px) and Hardness (0–100 %), each as a number
box and a slider, over the controller every paint tool reads. The Size slider's
first half SHALL cover 1–100 px evenly and its second half SHALL climb
geometrically from 100 to 5000 px. A preset SHALL set the whole tip: size,
hardness, roundness, angle, spacing, and dab dynamics.

#### Scenario: The slider midpoint is 100 px

- **WHEN** the Size slider is set to the middle of its travel
- **THEN** the brush size is 100 px and the tip button reads 100

#### Scenario: A spatter preset paints scattered

- **WHEN** Spatter 24 is chosen and the Brush clicks once
- **THEN** the controller holds size 24, count 7, scatter 150 %, and paint lands well outside the 24 px tip

### Requirement: Additional brush presets

The default brush set SHALL include `2B Pencil` (8 px), `Airbrush Soft Round
50% flow` (45 px), `Non-Photo Blue Pencil` (4 px), and `Paintbrush Oval 45
pixels Multiply` (45 px) after the existing CS6 set. `2B Pencil` and `Non-Photo
Blue Pencil` SHALL be anti-aliased round tips that set the pen-tip preview
model; `2B Pencil` SHALL use 25 % hardness at 25 % spacing, so its 100 % flow
paints a ~7 px stroke with soft shoulders rather than a flat top, and
`Non-Photo Blue Pencil` SHALL use 0 % hardness at 20 % spacing, so it paints a
~3.5 px line; both pencils SHALL paint with a fixed, deterministic paper grain.
The grain SHALL form 2–3 px clumps with fine per-pixel variation. `2B Pencil`
SHALL use a 50 % grain that leaves roughly 75 % mean core coverage; `Non-Photo
Blue Pencil` SHALL use a 55 % grain that leaves roughly 72 % mean core
coverage. `Airbrush Soft Round 50% flow` SHALL be a soft round tip with 25 %
spacing, so its 50 % flow builds a light, soft stroke instead of a solid core;
`Paintbrush Oval 45 pixels Multiply` SHALL be a hard (100 %) oval tip at 50 %
roundness, tilted with its major axis up and to the right, at 5 % spacing, and
SHALL paint with Wet Edges. Selecting one of these four presets SHALL set its
tip — size, hardness, roundness, angle, spacing, dab dynamics, grain, Wet
Edges, and the pen-tip preview model — and its default paint settings:
foreground colour, Mode, Opacity, and Flow. `Non-Photo Blue Pencil` SHALL use
foreground `#73d6ff`; the other three SHALL use `#47484a`. `2B Pencil` and
`Non-Photo Blue Pencil` SHALL use Normal mode at 100 % opacity and 100 % flow;
`Airbrush Soft Round 50% flow` SHALL use Normal mode at 100 % opacity and 50 %
flow; `Paintbrush Oval 45 pixels Multiply` SHALL use Multiply mode at 35 %
opacity and 50 % flow. Selecting an existing CS6 preset SHALL still set only
its tip, including turning Wet Edges off, and SHALL NOT change the foreground
colour, Mode, Opacity, or Flow. The exact Photoshop tip data is not available;
the settings are approximated from the owner's reference and marked non-parity.

#### Scenario: The four additional presets are listed

- **WHEN** the Brush Preset picker is opened
- **THEN** the grid lists `2B Pencil`, `Airbrush Soft Round 50% flow`,
  `Non-Photo Blue Pencil`, and `Paintbrush Oval 45 pixels Multiply` after the
  existing presets

#### Scenario: The additional presets apply their paint settings

- **WHEN** each of the four additional presets is chosen after setting a
  different foreground colour, Mode, Opacity, and Flow
- **THEN** the preset's tip and its default colour, Mode, Opacity, and Flow are
  applied; `Non-Photo Blue Pencil` paints `#73d6ff`, the others `#47484a`, the
  pencils report 25 % / 0 % hardness and 50 % / 55 % grain, and `Paintbrush
  Oval 45 pixels Multiply` uses Multiply at 35 % opacity and 50 % flow

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
  mean core coverage is about 75 %, adjacent grain pixels cluster at 2–3 px,
  and the same stroke renders the same every time

#### Scenario: The Non-Photo Blue Pencil paints a mottled stroke

- **WHEN** the Pencil tool paints a stroke with `Non-Photo Blue Pencil`
- **THEN** the stroke's alpha is broken up by the same fixed paper grain; its
  mean core coverage is about 72 %, adjacent grain pixels cluster at 2–3 px,
  and the same stroke renders the same every time

#### Scenario: The oval preset paints a wet-edged stroke

- **WHEN** `Paintbrush Oval 45 pixels Multiply` is chosen
- **THEN** the Brush reports 45 px, 100 % hardness, 50 % roundness, −50°
  angle, 5 % spacing, and Wet Edges on, and choosing `Soft Round 5` afterwards
  turns Wet Edges off

#### Scenario: The 2B Pencil paints the reference profile

- **WHEN** the Brush paints a straight stroke with `2B Pencil`
- **THEN** each cross-section covers 6–8 px, integrates to 3.2–4.4 px of full
  coverage, and peaks at 0.70–0.88 on average

#### Scenario: The Non-Photo Blue Pencil paints the reference profile

- **WHEN** the Brush paints a straight stroke with `Non-Photo Blue Pencil`
- **THEN** each cross-section covers 2.5–4.5 px, integrates to 1.5–2.1 px of
  full coverage, and peaks at 0.64–0.84 on average
