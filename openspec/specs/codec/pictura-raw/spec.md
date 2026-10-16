# pictura-raw Specification

## Purpose
The Pictura Raw smart filter: settings alias, Fltr codec, layer attachment, render, document op, and apply command.

## Requirements

### Requirement: Pictura Raw settings alias

The system SHALL expose `pictura_core::PicturaRawSettings` as an owning settings
struct composed of per-tab sub-structs: Basic (the 11 PV2012 controls temperature, tint,
exposure, contrast, highlights, shadows, whites, blacks, clarity, vibrance, saturation),
mode and white balance, tone curve (parametric regions, split points, and point curves),
detail (sharpening and luminance/chroma noise reduction), HSL color ranges and grayscale
mix, split toning, lens corrections, effects (grain, Dehaze, and post-crop vignette), and
camera calibration (camera profile, profile digest, the red/green/blue primary hue and
saturation, and the shadow tint). Each control SHALL be optional so an unset control is
distinguishable from a control set to its default. `pictura_core::CrsSettings` SHALL remain
the typed view of the `crs:` XMP property set and SHALL be usable as the Basic sub-struct.
The Pictura Raw settings type MUST NOT be a bare type alias of `CrsSettings`. The Camera
Calibration primaries and shadow tint MUST live in the camera-calibration sub-struct, and
the HSL sub-struct MUST NOT carry red/green/blue legacy hue or saturation controls.

#### Scenario: Alias resolves to the same type

- **WHEN** a `pictura_core::CrsSettings` is used as the Basic sub-struct of a
  `PicturaRawSettings` value
- **THEN** its controls are addressable without conversion

#### Scenario: Settings carry every modeled tab

- **WHEN** a `PicturaRawSettings` value sets controls in more than one tab
- **THEN** each tab's controls are addressable as distinct fields and the value is
  cloneable and comparable

#### Scenario: Unset control differs from a control set to default

- **WHEN** one settings value leaves a control unset and another sets that same control
  to its default value
- **THEN** the two settings values compare unequal

#### Scenario: Calibration controls live under camera calibration

- **WHEN** a `PicturaRawSettings` value sets a primary hue, a primary saturation, or the
  shadow tint
- **THEN** those controls are addressable on the camera-calibration sub-struct and the HSL
  sub-struct exposes only the eight-range hue, saturation, luminance, and grayscale mix

### Requirement: Pictura Raw Fltr codec

The system SHALL provide `pictura_codec::decode_pictura_raw_settings(&[u8]) ->
PicturaRawSettings` and `pictura_codec::encode_pictura_raw_fltr(&PicturaRawSettings) ->
DescValue`. Encoding SHALL write each modeled control to its on-disk `Fltr` key using that
key's observed value type and SHALL omit a key whose control is unset or empty, so
encoding default settings writes no controls. Decoding SHALL read each modeled key and
leave a control unset (`None`) for a missing, non-finite, unparseable, or unmodeled value.
The modeled key map SHALL be the PV2012 tab set:

- Basic: `Temp`, `Tint`, `Cr12`, `Hi12`, `Sh12`, `Wh12`, `Bk12`, `Cl12`, `Vibr`, `Strt`,
  as integers; `Ex12` as a double.
- Mode and white balance: `CMod` (text), `WBal` (enum), `Sett` (enum), `AWBV` and `PrVe`
  (integers), `CtoG` (boolean).
- Tone curve: `PC_S`, `PC_D`, `PC_L`, `PC_H`, `PC_1`, `PC_2`, `PC_3` as integers;
  `Crv `, `CrvR`, `CrvG`, `CrvB` as lists of integers holding flattened input/output
  point pairs.
- Detail: `Shrp`, `ShpD`, `ShpM`, `LNR `, `LNRD`, `LNRC`, `CNR `, `CNRD`, `CNRS` as
  integers; `ShpR` as a double.
- HSL color ranges: `HA_*`, `SA_*`, `LA_*` for the ranges `_R`, `_O`, `_Y`, `_G`, `_A`,
  `_B`, `_P`, `_M`, as integers.
- Grayscale mix: `GM_*` for the same eight ranges, as integers.
- Split toning: `STSH`, `STSS`, `STHH`, `STHS`, `STB `, as integers.
- Lens corrections: `LPEn`, `MDis`, `VigA`, `VigM`, `PerV`, `PerH`, `PerS`, `PerA`,
  `PerU`, `DfPA`, `DfGA`, `DPHL`, `DPPH`, `DPGL`, `DPGH` as integers; `PerR`, `PerX`,
  `PerY` as doubles.
- Effects: `GRNA`, `GRNS`, `GRNF`, `Dhze`, `PCVA`, `PCVM`, `PCVF`, `PCVR`, `PCVS`, `PCVH`,
  as integers.
- Camera calibration: `CamP`, `CP_D`, as text; `RHue`, `RSat`, `GHue`, `GSat`, `BHue`,
  `BSat`, and `BlkB`, as integers.

A key whose name ends in a space (`Crv `, `LNR `, `CNR `, `STB `) is a four-byte key
whose final byte is a space and MUST be written and read with that space.

#### Scenario: Set fields round-trip

- **WHEN** settings with controls set in every modeled tab are encoded and decoded
- **THEN** the decoded settings equal the input, including curve point lists, enum
  codes, text controls, Dehaze, and the camera-calibration primaries and shadow tint

#### Scenario: Default settings emit no controls

- **WHEN** default settings are encoded
- **THEN** the resulting `Fltr` object contains no controls

#### Scenario: Decode is tolerant

- **WHEN** the buffer omits a key, carries a value of the wrong type for a numeric key,
  or is not a descriptor
- **THEN** the affected controls are unset and no panic occurs

### Requirement: Attaching the filter to a layer

The system SHALL provide `pictura_codec::attach_pictura_raw_filter(&mut Layer,
&PicturaRawSettings)`. When the layer already carries a pictura-raw smart filter it SHALL
update that filter's `Fltr` in place. When a preserved `SoLd`/`SoLE` has no
pictura-raw entry it SHALL insert one while preserving every other descriptor key.
When the layer has an authored embedded smart object with no preserved block it
SHALL record the filter so the writer authors it into the `SoLd`. It MUST return
an error without mutating a layer that has no smart object.

#### Scenario: Settings survive save and read

- **WHEN** the filter is attached and the document is written and read back
- **THEN** the layer's pictura-raw smart filter carries the same settings

### Requirement: Pictura Raw render

The system SHALL provide `pictura_adjust::render_pictura_raw(&PixelBuffer,
&PicturaRawSettings) -> Result<PixelBuffer, AdjustError>` over 8-bit 3- or 4-channel
buffers, working in `f32` and preserving the channel count and alpha. The pass SHALL apply
the tabs in this order: Basic (white balance, exposure, contrast,
highlights/shadows/whites/blacks, clarity, vibrance, saturation), Tone Curve, Detail,
HSL/Grayscale, Split Toning, Lens Corrections, Effects, Camera Calibration. Each stage
SHALL be an approximation of the reference behavior, documented as behavioral parity only,
and SHALL leave the pixels unchanged when its controls are unset or at their default.
Default settings MUST be a byte-identical no-op and the result MUST be deterministic.

#### Scenario: Default settings are a no-op

- **WHEN** `render_pictura_raw` is called with all-`None` settings
- **THEN** the output is byte-identical to the input

#### Scenario: Exposure is monotonic

- **WHEN** a mid-grey pixel is rendered with positive exposure
- **THEN** it is brighter than the input, and a negative exposure is darker

#### Scenario: Every tab changes the render

- **WHEN** a settings value sets a non-default control in Tone Curve, Detail,
  HSL/Grayscale, Split Toning, Lens Corrections, Effects, or Camera Calibration
- **THEN** the rendered output differs from the same render with that control unset

#### Scenario: Malformed buffers are refused

- **WHEN** the buffer has fewer than 3 channels or a length that does not match its dimensions
- **THEN** `render_pictura_raw` returns an `AdjustError` and does not panic

### Requirement: Pictura Raw document op

The system SHALL provide `pictura_render::apply_pictura_raw(&mut Document, &str,
&PicturaRawSettings) -> bool`. It SHALL resolve a smart-object layer with an embedded,
decodable payload, render its source, run the Pictura Raw pipeline, write the result into
the layer's color channels while preserving alpha and the layer rect, and attach
the settings. It MUST return `false` without mutating for a missing path, a
group, an adjustment layer, a non-smart-object, a non-embedded object, or an
empty or undecodable payload.

#### Scenario: Apply bakes the proxy and records settings

- **WHEN** `apply_pictura_raw` runs on a converted smart-object layer
- **THEN** its color channels change, its alpha is unchanged, and the filter settings round-trip through save and read

#### Scenario: Ineligible targets are refused

- **WHEN** the target is a plain pixel layer or a missing path
- **THEN** `apply_pictura_raw` returns `false` and the document is unchanged

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

### Requirement: Pictura Raw unmodeled key preservation

When a Pictura Raw filter is re-attached, the system SHALL preserve every `Fltr` key it
does not model, so keys written by another Camera Raw version survive a read-edit-write
cycle byte-for-byte. The unmodeled set SHALL include at least `Upri`, `GuUr`, `Rtch`,
`REye`, `LCs `, `AuCA`, and `TMMs`.

#### Scenario: Unmodeled keys survive a re-attach

- **WHEN** a filter carrying the unmodeled keys is decoded, re-attached, written, and
  read back
- **THEN** each unmodeled key is present with its original value

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
