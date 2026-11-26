# Proposal

## Why

Issue #564 calls for one-click Vibrance and reusable settings. The
Image > Adjustments dialog has two manual sliders, and the shared Preset
menu has no way to save a named Vibrance setting.

## What Changes

- Add Auto to the direct Vibrance dialog. Use the opening active raster
  layer and selection to recommend a boost, keeping the Saturation value
  intact.
- Enable Vibrance in the existing Properties Save and Load Preset
  commands. Both Preset menus can apply saved Vibrance and Saturation
  values.
- Document Auto as an approximation and saved Vibrance presets as a
  Pictura extension. Adobe's CS6 guide describes the manual Vibrance
  sliders and omits Vibrance from its documented preset list.

## Capabilities

### Modified Capabilities

- `ui/adjustment-dialog-auto` gains a color-aware Auto for Vibrance.
- `imaging/adjustment-presets` gains Vibrance as a saveable adjustment.

## Impact

The Image Adjustments bridge and stacked dialog gain the Auto recommendation.
The existing render-side preset store adds Vibrance to its save list. Qt tests
cover the dialog and the shared Properties preset actions.
