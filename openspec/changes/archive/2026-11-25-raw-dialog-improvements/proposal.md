# Proposal

## Why

Issue #1500 reports three defects in the Pictura Raw dialog. The icon tab strip
draws a dotted base line through the buttons (it reads as white dots). The
controls column sits directly against the preview, so Exposure, Contrast, and
the rest start a few pixels from the image. The Presets (fx) tab is a static
list: Image Settings, Camera Raw Defaults, Previous Conversion, Clear Imported
Settings, Load Settings…, and Save Settings… are labels that do nothing. The
issue body also notes that the mouse wheel does not zoom the preview.

`ROADMAP.md` records the Presets tab as a stub with no persistence (#633); the
preset library itself stays out of scope here. The dialog is the shared front
end for `Filter > Pictura Raw…` and the camera-raw open, so the fixes land in
`crates/pictura-app/cpp/acr/` and the dialog rather than in the raw engine.

## What Changes

- Give the tab strip an even icon row with no base line, and space the controls
  column away from the preview.
- Make the Presets (fx) list actionable: Image Settings restores the settings the
  dialog opened with, Camera Raw Defaults clears every control, Previous
  Conversion applies the last accepted settings of the session, Clear Imported
  Settings reverts a loaded file to the image settings, and Load/Save Settings
  read and write `crs:` XMP. Custom Settings, Preset Settings, and Apply Preset
  stay present but disabled until #633.
- Step the preview zoom with the mouse wheel along the same ladder as the − and
  + buttons.
- Expose two free bridge functions that convert between the `crs:` XMP packet
  and the bridge settings text, so the C++ dialog does not parse XMP itself.

No render, settings-codec, or smart-filter behavior changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/pictura-raw`: the capability gains settings actions, XMP load/save,
  wheel zoom, and an even tab strip. The requirements are additive; the existing
  tab, preview, histogram, and OK/Cancel contracts are unchanged.
