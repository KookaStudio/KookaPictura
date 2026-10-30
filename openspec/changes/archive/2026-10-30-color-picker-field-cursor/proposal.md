# Proposal

## Why

The Color Picker's colour field shows the platform crosshair (a "target" cross)
while the pointer is over it, so the pointer and the ring the field paints at
the current colour disagree. Photoshop's field pointer is that same ring.

## What Changes

- A new cursor asset, `cursor.colorPickerField`, draws the ring the field's
  marker draws (a white-haloed dark circle).
- `ColorPlane` — the dialog's field and the Color panel's inline field — uses
  it, falling back to the crosshair when the asset is missing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/color-picker`: the colour field's pointer is a circle.

## Impact

- `assets/cursors/cursor.colorPickerField.svg`, `assets/pictura.qrc`.
- `crates/pictura-app/cpp/color_picker_dialog.cpp`.
- One new `tst_color_picker` case.
- No new dependency, no `docs/` change, no `ROADMAP.md` item (UI polish within
  an already-shipped feature).
