# tools/gradient-tool Specification

## Purpose
The Gradient tool: drag a gradient fill across a layer or selection.

## Requirements

### Requirement: Gradient tool

A Gradient drag SHALL draw the chosen gradient over the active pixel layer
along the axis from the press to the release — Linear, Radial, Angle,
Reflected, or Diamond — extending the ramp's end colours past both ends, inside
the selection when there is one, at Opacity in the chosen Mode, recording
exactly one "Gradient" history state. Reverse SHALL swap the ramp's ends;
Transparency off SHALL ignore the ramp's own alpha; Transparency on SHALL make
the ramp's alpha the alpha of the pixels it draws, in the Normal mode, so that
the ramp's transparent end is transparent; Shift SHALL snap the axis to 45°
steps. A click without a drag SHALL draw nothing, a transparency-locked
layer SHALL keep every pixel's alpha, and a layer whose pixels are locked SHALL
be refused.

#### Scenario: Drawing a linear gradient

- **WHEN** the `tst_fill_tools` gradient test drags across an opened image with black foreground and white background
- **THEN** the axis is shown during the drag, one "Gradient" state is recorded, the start is black and the end white, and undo restores the image

#### Scenario: A click draws nothing

- **WHEN** the canvas is clicked without a drag
- **THEN** no state is recorded

#### Scenario: The selection confines the gradient

- **WHEN** a Radial gradient is dragged inside a selection of the left half
- **THEN** the centre is black and the right half is unchanged

#### Scenario: The ramp's alpha reaches the pixels [gt_ramp_alpha]

- **WHEN** a gradient whose far stop is transparent is dragged across a layer
  in the Normal mode with Transparency on
- **THEN** the pixels at the transparent end hold zero alpha and the pixels at
  the opaque end hold full alpha

#### Scenario: Transparency off ignores the ramp's alpha [gt_ramp_alpha_off]

- **WHEN** the same gradient is dragged with Transparency off
- **THEN** every drawn pixel holds full alpha
