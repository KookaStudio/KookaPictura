## MODIFIED Requirements

### Requirement: Auto button availability

The Levels, Curves, Brightness/Contrast, and Vibrance Image > Adjustments
dialogs SHALL show an enabled `Auto` button. Other stacked-slider adjustment
dialogs MUST NOT show an Auto button.

#### Scenario: Four direct dialogs expose Auto [auto_button_present]

- **WHEN** the Levels, Curves, Brightness/Contrast, and Vibrance dialogs open
- **THEN** each exposes an enabled `Auto` button

## ADDED Requirements

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
