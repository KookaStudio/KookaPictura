# Spec Delta

## ADDED Requirements

### Requirement: Pictura Raw destructive document op

The system SHALL provide `pictura_render::apply_pictura_raw_destructive(&mut Document,
&str, &PicturaRawSettings) -> bool`. It SHALL resolve a raster pixel layer (including
the Background) in an RGB or Grayscale document, filter the layer's own color channels
over its `rect`, and write the filtered result back into those channels while preserving
the layer's alpha, its channel set, its `rect`, and its Background identity, so a saved
document reflects the filtered pixels at the document's bit depth. It MUST create
no smart object and attach no filter, so no settings are retained. It MUST return `false`
without mutating for a missing path, a group, an adjustment layer, a smart object, a
layer with no color channels, a pixel-locked layer, a non-positive rect, or a document
whose mode is neither RGB nor Grayscale.

#### Scenario: Destructive apply filters the layer pixels without a smart object

- **WHEN** `apply_pictura_raw_destructive` runs on a plain raster pixel layer
- **THEN** its color channels change, its alpha and rect are unchanged, the layer is not
  a smart object, and the layer's Pictura Raw settings readback is empty

#### Scenario: Background keeps its Background identity

- **WHEN** `apply_pictura_raw_destructive` runs on the Background
- **THEN** its color channels change while it stays the Background (still flagged and
  named, no alpha channel added)

#### Scenario: Filtered pixels survive a save at the document bit depth

- **WHEN** `apply_pictura_raw_destructive` runs on a raster layer of a 16-bit document
  and the document is saved and read back
- **THEN** the re-read channel data reflects the filtered pixels, not the pre-edit
  native samples

#### Scenario: Ineligible targets are refused

- **WHEN** the target is a group, an adjustment layer, a smart object, a pixel-locked
  layer, a layer with no color channel, or a document whose mode is not RGB or Grayscale
- **THEN** `apply_pictura_raw_destructive` returns `false` and the document is unchanged

## MODIFIED Requirements

### Requirement: Pictura Raw apply command

The system SHALL provide a bridge command `apply_pictura_raw_filter(path, settings)`
taking the active layer path and the layer's complete Pictura Raw settings in the
serialized form defined by the settings readback. When `path` resolves to an embedded
smart-object layer it SHALL apply Pictura Raw to it, adding or refreshing the camera-raw
smart filter and retaining the settings. When `path` resolves to a raster pixel layer it
SHALL apply Pictura Raw destructively to the layer's own pixels, creating no smart object
and retaining no settings. It MUST NOT convert the layer to a smart object. On success it
SHALL recomposite and record exactly one `"Pictura Raw"` history state. It MUST return
false and record no history for a missing document or target, or when the engine refuses
the apply.

#### Scenario: Raster target converts then filters in one state

- **WHEN** `apply_pictura_raw_filter` runs on a plain raster pixel layer
- **THEN** its pixels change in place with no conversion to a smart object, no settings
  are retained, and exactly one `"Pictura Raw"` state is recorded

#### Scenario: Embedded smart object re-applies in one state

- **WHEN** `apply_pictura_raw_filter` runs on an embedded smart-object layer
- **THEN** the camera-raw smart filter is added or refreshed, the settings are retained,
  and exactly one `"Pictura Raw"` state is recorded

#### Scenario: Full settings persist across apply and readback

- **WHEN** settings set controls in more than one tab are applied to a smart object and
  then read back
- **THEN** the readback carries the same control values, including curve point lists

#### Scenario: Ineligible target records nothing

- **WHEN** the target is a group, a pixel-locked or type layer, a legacy placed layer,
  a non-RGB/Grayscale document, or there is no document
- **THEN** the command returns false and the history count is unchanged

### Requirement: Pictura Raw menu command

The system SHALL expose `Filter > Pictura Raw…` as an implemented command. The menu
label MUST read exactly `Pictura Raw…`. The command SHALL be enabled when a document is
open and the current layer is either a raster pixel layer (including the Background) that
is not pixel-locked and carries a color channel, in an RGB or Grayscale document at any
bit depth, or a replaceable embedded smart object. Enablement MUST NOT depend on whether
the layer can be converted to a smart object. It SHALL prefill the dialog from
`layer_pictura_raw_settings` and apply on OK.

#### Scenario: Command is enabled for an eligible target

- **WHEN** a document is open with a raster pixel layer, a Background, or a replaceable
  embedded smart-object current layer
- **THEN** the command is enabled

#### Scenario: Command is disabled for an ineligible layer

- **WHEN** the current layer is a group, an adjustment layer, or a pixel-locked raster layer
- **THEN** the command is disabled

#### Scenario: Command is disabled without a document

- **WHEN** no document is open
- **THEN** the command is disabled
