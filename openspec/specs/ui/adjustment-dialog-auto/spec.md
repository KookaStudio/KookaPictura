# ui/adjustment-dialog-auto Specification

## Purpose
TBD - created by archiving change adjustment-dialog-auto. Update Purpose after archive.

## Requirements

### Requirement: Shared histogram Auto bounds

The Levels, Curves, and Brightness/Contrast dialogs SHALL compute their Auto
bounds with one shared rule over the canvas histogram sampled when the dialog
opens: discard the first and last 0.1% of the population and return the
remaining black and white endpoints. The rule MUST return no bounds when the
histogram is empty or the endpoints are fewer than two levels apart, and an
Auto click with no bounds MUST leave the adjustment block unchanged and record
no history. Auto MUST only edit the dialog's scratch block; the single history
state is still recorded by OK.

#### Scenario: All three dialogs use the 0.1% clip [auto_shared_clip]

- **WHEN** Auto is clicked in the Levels, Curves, or Brightness/Contrast dialog
  on a ramp that occupies a known sub-range of the histogram
- **THEN** each dialog derives its edit from the same black and white endpoints,
  discarding 0.1% of the pixels at each end

#### Scenario: A flat image makes Auto a no-op [auto_flat_noop]

- **WHEN** Auto is clicked on a single-level image whose histogram spans fewer
  than two levels
- **THEN** the dialog's parameters are unchanged and no history state is added

### Requirement: Levels Auto

The Levels Auto button SHALL set the selected channel's input black and input
white to the shared bounds, resetting input black to 0 first so the engine's
black-below-white rule accepts the edit. The edit MUST preview once and MUST
move the Preset menu to `Custom`.

#### Scenario: Levels Auto stretches the selected channel [auto_levels_stretch]

- **WHEN** Auto is clicked on the Red channel of a ramp that occupies 40 through
  192
- **THEN** input black is about 40, input white is about 192, and the preset
  reads `Custom`

### Requirement: Curves Auto

The Curves Auto button SHALL replace the selected channel's curve with two
points that map the shared black endpoint to output 0 and the shared white
endpoint to output 255.

#### Scenario: Curves Auto writes a two-point stretch [auto_curves_stretch]

- **WHEN** Auto is clicked on a channel whose ramp occupies 40 through 192
- **THEN** the channel's curve is two points, `(40, 0)` and `(192, 255)`, within
  one level of the shared bounds

### Requirement: Brightness/Contrast Auto

The Brightness/Contrast Auto button SHALL be enabled and SHALL set brightness
and contrast from the shared bounds of the composite histogram. Brightness MUST
centre the occupied range through the engine's modern gamma curve, and contrast
MUST scale the S-curve slope to expand the remaining range, both clamped to the
engine's `-150..150` and `-50..100` ranges. The mapping is behavioural parity
only: Adobe's exact fit is closed.

#### Scenario: Brightness/Contrast Auto expands a low-contrast ramp [auto_bc_contrast]

- **WHEN** Auto is clicked on a neutral ramp that occupies 64 through 192
- **THEN** brightness stays near 0, contrast becomes positive, and the
  previewed range is wider than the original range

### Requirement: Auto button availability

The Levels, Curves, and Brightness/Contrast dialogs SHALL show an enabled
`Auto` button. Other stacked-slider adjustment dialogs, including Vibrance,
MUST NOT show an Auto button.

#### Scenario: Auto is present only on the three tonal dialogs [auto_button_present]

- **WHEN** the Levels, Curves, Brightness/Contrast, and Vibrance dialogs open
- **THEN** the first three expose an enabled `Auto` button and Vibrance exposes
  none
