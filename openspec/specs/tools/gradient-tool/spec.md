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

### Requirement: Gradient Editor from the Gradient tool

The Gradient tool's options bar SHALL open the Gradient Editor from the main
part of the gradient sample while keeping the preset menu on the sample's arrow.
The editor SHALL open with the tool's current ramp. Accepting the editor SHALL
make the edited solid gradient — including its opacity stops and midpoints — the
ramp the next drag draws and SHALL leave no preset checked; cancelling SHALL keep
the ramp the tool had. Choosing a preset from the sample's menu SHALL replace
the edited ramp with that built-in gradient.

#### Scenario: The sample opens the editor [gt_editor_opens]

- **WHEN** the Gradient tool's gradient sample is clicked
- **THEN** the Gradient Editor opens with the tool's current colour stops

#### Scenario: An accepted gradient is drawn [gt_editor_applies]

- **WHEN** the editor is accepted after its start opacity stop is set to 0 % and a colour midpoint is moved off 50 %
- **THEN** the next drag draws the edited ramp: the start is transparent and the midpoint's even-mix point has moved

#### Scenario: Cancelling keeps the previous ramp [gt_editor_cancels]

- **WHEN** the editor is opened, edited, and cancelled
- **THEN** the next drag still draws the ramp the tool had before the editor opened

#### Scenario: A preset replaces the edited ramp [gt_preset_replaces]

- **WHEN** a built-in preset is chosen from the sample's menu after an edited gradient was accepted
- **THEN** the next drag draws that preset and the edited ramp is forgotten
