## ADDED Requirements

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
