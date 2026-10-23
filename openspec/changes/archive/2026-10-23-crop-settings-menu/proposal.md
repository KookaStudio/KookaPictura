# Proposal

## Why

Issue #104: the Crop options bar's cog menu listed Use Classic Mode, Show
Cropped Area, Auto Center Preview, Enable Crop Shield, Color, Opacity and Auto
Adjust Opacity, but only Classic mode did anything. The shield was a hardcoded
`QColor(0, 0, 0, 150)`, `P` and `H` did nothing, and the menu looked flat and
loosely spaced next to CS6's.

## What Changes

- Show Cropped Area (`H`), Auto Center Preview, Enable Crop Shield, Color
  (Match Canvas or Custom, with a colour picker), Opacity (0-100 %, default
  75 %) and Auto Adjust Opacity drive the canvas.
- Classic mode disables Show Cropped Area and Auto Center Preview and shows
  them unchecked; leaving Classic checks both.
- `P` toggles Classic mode and `H` toggles Show Cropped Area while the Crop
  tool is active.
- The settings persist in the session store (`toolOptions.crop`).
- The menu is compact: tight rows, a shortcut column, and the shield controls
  in one indented panel.
- **BREAKING**: none.

## Capabilities

### New Capabilities
- `tools/crop-settings`: the Crop tool's Settings menu.

### Modified Capabilities
None.
