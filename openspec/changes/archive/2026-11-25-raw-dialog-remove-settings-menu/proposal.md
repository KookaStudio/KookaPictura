# Proposal

## Why

The far-right `fx` settings button added with the Presets tab does not match
Camera Raw: its list items are not the reference menu, and the dialog already
has the correct `fx`-icon tab — the Effects tab. The user wants only that
Effects tab kept, with its Effects controls, and the far-right settings button
and every action it owns removed.

## What Changes

- Remove the `acrSettingsButton` at the right of the tab strip and its popup
  settings list.
- Remove the settings actions behind it: Image Settings, Camera Raw Defaults,
  Previous Conversion, Clear Imported Settings, and `crs:` XMP Load/Save, along
  with the session-conversion state and the two bridge functions that only
  served them.
- Keep the Effects tab (the `fx`-icon tab) and its Grain and Post Crop
  Vignetting controls.
- Move the slider labels a few more pixels right inside each panel.

The raw open's own settings save on OK/Done is a separate path and is unchanged.
The histogram, photo information, picker, status bar, and size grip are
unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/pictura-raw`: the settings-actions and settings-files requirements are
  removed, and the effects-tab requirement replaces the effects-icon-and-
  settings-button requirement. The panel-inset requirement stays, with a larger
  inset.
