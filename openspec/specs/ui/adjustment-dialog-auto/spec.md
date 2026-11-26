# ui/adjustment-dialog-auto Specification

## Purpose
The shared histogram Auto behind the Levels, Curves, and Brightness/Contrast
Image > Adjustments dialogs.

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

The Levels, Curves, Brightness/Contrast, and Vibrance Image > Adjustments
dialogs SHALL show an enabled `Auto` button. Other stacked-slider adjustment
dialogs MUST NOT show an Auto button.

#### Scenario: Four direct dialogs expose Auto [auto_button_present]

- **WHEN** the Levels, Curves, Brightness/Contrast, and Vibrance dialogs open
- **THEN** each exposes an enabled `Auto` button

### Requirement: Vibrance Auto

Vibrance Auto SHALL estimate a positive boost from a bounded sample of the
opening active RGB raster layer, within the active selection if present.
Transparent, unselected, and effectively neutral samples SHALL be excluded.
Muted colors SHALL receive a suggested Vibrance value capped at +35.
A positive recommendation SHALL replace the current Vibrance value, even if
it is lower. Auto MUST leave Saturation unchanged, including at -100, and MUST
leave the scratch block unchanged when no positive recommendation is available. The estimate SHALL
be independent of earlier dialog previews. Auto SHALL use the dialog's
existing preview and acceptance lifecycle.

#### Scenario: Auto previews a muted raster [vibrance_auto_muted]

- **WHEN** Auto is clicked in the Vibrance dialog on a muted colored raster
  with Vibrance and Saturation initially at zero
- **THEN** Vibrance becomes +1 to +35, Saturation is preserved, and the
  preview increases chroma without recording history
- **AND** clicking Auto again yields the same block, and Cancel restores the
  original pixels and history

#### Scenario: Auto replaces a higher value and preserves desaturation [vibrance_auto_replace]

- **WHEN** Auto is clicked on a muted colored raster with Vibrance at +100 and
  Saturation at -100
- **THEN** Vibrance becomes the opening recommendation in the range +1 to +35,
  Saturation remains -100, and the preview remains gray

#### Scenario: Opening preview does not affect the recommendation [vibrance_auto_opening_preview]

- **WHEN** the dialog opens with a non-neutral Vibrance block and its opening
  preview changes the displayed colors before Auto is clicked
- **THEN** Auto uses the recommendation from the original opening raster,
  yielding the same value as it would without that preview

#### Scenario: Sampling weights alpha and selection coverage [vibrance_auto_weights]

- **WHEN** the sampled raster includes differently colored pixels with zero or
  fractional alpha and zero or fractional selection coverage
- **THEN** transparent and unselected samples contribute nothing, and the
  remaining samples contribute in proportion to alpha and selection coverage

#### Scenario: The opening selection controls the sample [vibrance_auto_selection]

- **WHEN** a muted region is selected in a layer that predominantly contains
  already vivid colors, and Auto is clicked from the default settings and accepted
- **THEN** Vibrance increases, the pixels outside the selection are preserved,
  and exactly one Vibrance history state is recorded

#### Scenario: No positive recommendation keeps the block [vibrance_auto_noop]

- **WHEN** Auto is clicked on a fully neutral or already vivid raster
- **THEN** the scratch block and history are unchanged
