# Proposal

## Why

With only the Highlights saturation set (Shadows saturation 0), the preview
looked unchanged. The render did blend toward the highlight tint, but the
additive offset was too weak and the hue blend started from the pixel's HSL hue,
which is meaningless for a near-grey highlight, so the effect was invisible on
the kind of bright neutral areas the tab is meant to tint. Camera Raw tints the
highlights strongly in this case.

## What Changes

- Blend the pixel toward the tint colour at the pixel's own lightness, instead
  of adding a small signed offset.
- Keep the luminance scaling so the tint does not change the tonal value.
- Add a render test for a highlight-only tint and a dialog test that the preview
  changes when only the Highlights sliders move.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/pictura-raw`: the Split Toning requirement gains the rule that a
  non-zero saturation on one side tints that side even when the other side is
  zero.
