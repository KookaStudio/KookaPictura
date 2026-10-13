# Spec Delta

## MODIFIED Requirements

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

### Requirement: Pictura Raw unmodeled key preservation

When a Pictura Raw filter is re-attached, the system SHALL preserve every `Fltr` key it
does not model, so keys written by another Camera Raw version survive a read-edit-write
cycle byte-for-byte. The unmodeled set SHALL include at least `Upri`, `GuUr`, `Rtch`,
`REye`, `LCs `, `AuCA`, and `TMMs`.

#### Scenario: Unmodeled keys survive a re-attach

- **WHEN** a filter carrying the unmodeled keys is decoded, re-attached, written, and
  read back
- **THEN** each unmodeled key is present with its original value
