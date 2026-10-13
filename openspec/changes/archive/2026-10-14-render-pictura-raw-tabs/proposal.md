# Proposal

## Why

Pictura Raw (`filterID 2683`) renders only the 11 PV2012 Basic controls. A reference
Adobe file carries the full Camera Raw tab set in its `Fltr` descriptor, but the render
path drops everything except Basic, so Tone Curve, Detail, HSL/Grayscale, Split Toning,
Lens Corrections, Effects and Camera Calibration edits do not reach the pixels. The
settings model also mis-files the Camera Calibration controls: the red/green/blue primary
hue and saturation are stored as HSL legacy ranges, and the shadow tint is stored as a
"legacy blacks" field.

## What Changes

- Extend `pictura_adjust::render_pictura_raw` from the 11 Basic stages to every remaining
  tab, in tab order: Basic, Tone Curve, Detail, HSL/Grayscale, Split Toning, Lens
  Corrections, Effects, Camera Calibration. Every stage is an approximation of Adobe's
  closed algorithms, marked `ponytail:` inline, with one runnable check per stage.
- **BREAKING (settings type)**: move the Camera Calibration primaries
  (`RHue`/`RSat`/`GHue`/`GSat`/`BHue`/`BSat`) out of `Hsl` and the shadow tint (`BlkB`)
  out of `PicturaRawSettings.legacy_blacks`, into `CameraCalibration`; model `Dhze`
  (Dehaze) as an `Effects` control instead of a preserved key.
- Split the render module into one file per tab under `crates/pictura-adjust/src/pictura_raw/`
  to stay under the file-size cap, operating on shared `f32` planes so stages do not
  quantize through 8-bit between them.
- Correct the `docs/dev/camera-raw-cc-notes.md` key map and regenerate the synthetic
  `assets/test_with_smart_object02.psd` fixture for the corrected model.

The calibration keys were identified from a reference filter description produced with
the Camera Calibration primaries and shadow tint set off-default; the 111-key baseline
omits them because Adobe drops default-valued keys.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `codec/pictura-raw`: the settings-model requirement re-homes the Camera Calibration
  primaries and shadow tint and adds Dehaze; the Fltr-codec requirement moves `Dhze` into
  the modeled key map and regroups the calibration keys; the render requirement grows from
  the 11 Basic stages to every tab; the unmodeled-key preservation requirement drops
  `Dhze` from its set.
- `codec/psd-smart-filters`: `Dhze` moves out of the unmodeled set now that it is modeled,
  so the preservation requirement's example list changes.

## Impact

- **Code**: `crates/pictura-core/src/raw_settings.rs`; `crates/pictura-codec/src/pictura_raw/{codec.rs,tests.rs}`;
  `crates/pictura-codec/examples/make_smart_fixtures.rs`; `crates/pictura-adjust/src/pictura_raw.rs`
  becomes a module directory.
- **Assets**: regenerated `assets/test_with_smart_object02.psd` (synthetic; no bytes copied
  from a reference file).
- **Docs**: `docs/dev/camera-raw-cc-notes.md` §Fltr key map (carries `TASK-ALLOWS-DOCS`).
- **Out of scope**: the tabbed dialog, live preview, and histogram (#395), including the
  Lucide tab icons; Presets and Snapshots carry no render stage here.
