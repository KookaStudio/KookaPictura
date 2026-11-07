# Proposal

## Why

Opening a portrait PSD can leave a checkerboard strip beside an opaque mask
thumbnail because a rounded preview is fitted a second time into a taller card.
A separate white focus border also remains after thumbnail selection changes.

## What Changes

- Fill document-aspect mask cards without exposing rounding gaps.
- Draw the normal black outline around raster mask cards.
- Use the existing selection-aware white corner brackets as the mask's only
  white target indicator, including after raster-thumbnail and layer changes.
- Preserve square-card fitting and mask/vector modifier-click behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: opaque mask-card presentation and bracket-only selection.

## Impact

The Layers delegate and Qt regression suite. No engine, codec, dependency, or
file-format changes. Follow-up to #1199.
