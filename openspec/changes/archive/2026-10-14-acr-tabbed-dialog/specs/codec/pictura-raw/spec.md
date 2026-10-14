# Spec Delta

## MODIFIED Requirements

### Requirement: Pictura Raw apply command

The system SHALL provide a bridge command `apply_pictura_raw_filter(path,
settings)` taking the active layer path and the layer's complete Pictura Raw
settings in the serialized form defined by the settings readback. When `path`
resolves to a plain convertible raster pixel layer it SHALL first convert the
layer to an embedded smart object and then apply Pictura Raw; when it resolves
to an embedded smart-object layer it SHALL skip the conversion and apply
Pictura Raw. On success it SHALL recomposite and record exactly one
`"Pictura Raw"` history state. It MUST return false and record no history for a
missing document or target, or when the engine refuses the conversion or the
apply.

#### Scenario: Raster target converts then filters in one state

- **WHEN** `apply_pictura_raw_filter` runs on a plain raster pixel layer of an 8-bit document
- **THEN** the layer becomes an embedded smart object, its pixels change, and exactly one `"Pictura Raw"` state is recorded

#### Scenario: Embedded smart object re-applies in one state

- **WHEN** `apply_pictura_raw_filter` runs again on that smart-object layer
- **THEN** exactly one additional `"Pictura Raw"` state is recorded

#### Scenario: Full settings persist across apply and readback

- **WHEN** settings set controls in more than one tab are applied and then read back
- **THEN** the readback carries the same control values, including curve point lists

#### Scenario: Ineligible target records nothing

- **WHEN** the target is a group, a legacy placed layer, or there is no document
- **THEN** the command returns false and the history count is unchanged

### Requirement: Pictura Raw settings readback

The system SHALL provide a bridge read command `layer_pictura_raw_settings(path)`
that returns the complete stored settings of the layer's pictura-raw smart
filter in a single serialized form that can be handed back to the apply and
preview commands, or an empty string when the layer has no pictura-raw filter.
The serialized form MUST round-trip every modelled control, including curve
point lists, enum codes, and text profiles. It MUST be a pure read that mutates
no state.

#### Scenario: Stored settings are returned for prefill

- **WHEN** a pictura-raw filter is applied and its settings are read back
- **THEN** the returned string carries the applied values for every modelled tab

#### Scenario: A layer without a filter returns empty

- **WHEN** `layer_pictura_raw_settings` is called on a layer with no pictura-raw filter
- **THEN** it returns an empty string

### Requirement: Pictura Raw dialog

The system SHALL provide a modal `Pictura Raw` dialog with an icon tab strip
whose first tab is Basic and which exposes the Basic, Tone Curve, Detail,
HSL / Grayscale, Split Toning, Lens Corrections, Effects, Camera Calibration,
and Presets tabs. Selecting a tab SHALL show that tab's panel of controls. The
dialog SHALL show a live preview of the active layer rendered with the current
control values, and a histogram of the previewed image with shadow- and
highlight-clipping toggles and an RGB readout. The dialog SHALL have OK and
Cancel actions. Temperature and Tint SHALL use the approximate JPEG `-100…+100`
scale, Exposure SHALL use `-5…+5`, and other numeric controls SHALL use the
ranges in the Camera Raw Filter specification, treating unstated ranges as
inferred.

#### Scenario: Tab strip exposes every tab

- **WHEN** the dialog is opened
- **THEN** Basic is selected and every tab in the strip is selectable, each
  showing its own panel of controls

#### Scenario: Editing a control updates the preview

- **WHEN** the user changes a control in any tab
- **THEN** the preview and its histogram reflect the rendered result

#### Scenario: OK returns the edited values

- **WHEN** the dialog is accepted after editing a control
- **THEN** it reports the full settings for every tab, prefilled from the
  supplied initial settings

#### Scenario: Clipping toggles tint out-of-range pixels

- **WHEN** the shadow- or highlight-clipping toggle is enabled
- **THEN** pixels at or below black, or at or above white, are tinted in the preview

#### Scenario: Cancel reports no values

- **WHEN** the dialog is rejected
- **THEN** the caller applies nothing and no history state is recorded
