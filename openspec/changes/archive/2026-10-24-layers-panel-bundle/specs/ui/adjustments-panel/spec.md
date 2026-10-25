# Spec Delta — ui/adjustments-panel

## MODIFIED Requirements

### Requirement: Panel menu preserved

The Adjustments panel menu SHALL keep its existing rows (the sixteen adjustment
kinds in CS6 order, Add Mask by Default, Clip to Layer) and SHALL remain a
working alternative create path. The menu rows use the existing hyphenated row
texts (`Brightness-Contrast`, `Hue-Saturation`); the grid labels are the CS6
display names and need not match the row texts verbatim. **Add Mask by Default
SHALL be an enabled checkable preference** whose state reflects the session and
whose toggle governs whether a newly created adjustment layer receives a mask.
`Clip to Layer` SHALL remain a checkable row.

#### Scenario: Panel menu still lists the adjustment kinds

- **WHEN** the Adjustments panel menu is opened
- **THEN** it lists Brightness-Contrast, Levels, Curves, Exposure, Vibrance, Hue-Saturation, Color Balance, Black & White, Photo Filter, Channel Mixer, Color Lookup, Invert, Posterize, Threshold, Gradient Map, Selective Color, Add Mask by Default, and Clip to Layer, in that order

#### Scenario: Add Mask by Default is a working toggle [adj_add_mask_default_toggle]

- **WHEN** the user toggles Add Mask by Default in the Adjustments panel menu
- **THEN** the row's check state follows the preference and the next created adjustment layer honors it

## ADDED Requirements

### Requirement: Adjustment mask defaults

The system SHALL provide an `Add Mask by Default` preference, defaulting to
enabled and persisted in the session store. When it is enabled, a newly created
adjustment layer SHALL receive a layer mask: the active selection's mask when a
selection exists, otherwise a white document-sized mask. When it is disabled,
the new adjustment layer SHALL have no mask. After a successful creation with a
mask, the new layer's mask SHALL become the view's mask edit target, with mask
solo mode off, so painting and filling edit the mask immediately; without a
mask, the layer thumbnail SHALL be the active target.

#### Scenario: A selection becomes the adjustment mask [adj_mask_from_selection]

- **WHEN** Add Mask by Default is enabled, a selection is active, and an adjustment layer is created
- **THEN** the layer carries a mask built from the selection and that mask is the active target

#### Scenario: No selection creates a white adjustment mask [adj_mask_white]

- **WHEN** Add Mask by Default is enabled, no selection is active, and an adjustment layer is created
- **THEN** the layer carries a white document-sized mask and that mask is the active target, not mask solo mode

#### Scenario: Disabling the preference creates no mask [adj_mask_off]

- **WHEN** Add Mask by Default is disabled and an adjustment layer is created
- **THEN** the layer has no mask and its layer thumbnail is the active target
