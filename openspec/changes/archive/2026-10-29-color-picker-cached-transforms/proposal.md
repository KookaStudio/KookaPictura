# Proposal

## Why

Dragging in the Color Picker dialog (opened from Layer > New Fill Layer > Solid
Color, Edit > Fill's Color..., and every other `ColorPickerDialog::getColor`
caller) lags the cursor (#1037). The Color Fill paths pass no live-preview
callback, so the document is not recomposited. The cost is in the picker
itself: every drag sample runs `ColorPicker::syncControls`, whose
`updateAlerts` calls the Rust `picker_in_gamut`. Each call clones the working
CMYK ICC bytes, re-parses the profile, builds an sRGB→CMYK and a CMYK→sRGB
lcms2 transform, and converts one pixel. `picker_cmyk`, `picker_rgb` and
`picker_closest_rgb` repeat the same per-call build.

## What Changes

- Add `SpaceTransform` to `pictura-color`: a reusable 8-bit transform between
  two profiles of any supported colour space (grey, RGB, CMYK), built once and
  applied many times. `convert_space` is rebuilt on it, so the format mapping
  has one home.
- The picker colour services hold one sRGB↔working-CMYK transform pair. Color
  Settings rebuilds the pair when it installs a working CMYK profile; every other
  call reuses it.
- No behaviour change. Every picker service returns the same value it returned
  before, and the alerts, readouts and CMYK edits are unchanged.

Out of scope: the CMYK readout fields convert through the bundled Coated
profile (`CmykConverter::working`), not the working CMYK profile. Fixing that
mismatch is a behaviour change, and a separate issue tracks it.

## Capabilities

### New Capabilities

- `ui/color-picker-drag`: the picker's colour services reuse their transforms
  across calls, so each drag sample costs a pixel conversion and no profile
  parse or transform build.

### Modified Capabilities

None. `ui/color-picker` behaviour (alerts, readouts, CMYK edits) is unchanged.

## Impact

- `crates/pictura-color/src/lib.rs`: `SpaceTransform`; `convert_space` uses it.
- `crates/pictura-app/src/cxxqt_object/color_management.rs`: the cached
  transform pair replaces the stored ICC bytes; the four picker services use it.
- `crates/pictura-app/cpp/tests/tst_color_picker.cpp`: a drag check.
- No new dependency, no `docs/` change.
