# Proposal

## Why

The Ruler cannot measure an included angle, straighten an image, or change the ruler origin, leaving the workflows in #535 incomplete.

## What Changes

- Extend the transient measuring line with a second protractor arm, interaction, overlay and readouts.
- Add the Ruler Straighten action using the document rotation path and its existing undo and interpolation behavior.
- Allow the ruler intersection to set the ruler origin, with unsnapped placement. Snapping the origin to guides, grid and slices is deferred with Snap / Snap To (#305).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/ruler-tool`: protractor gestures, measurements and Straighten.
- `ui/rulers-and-guides`: movable ruler origin and origin-relative ruler coordinates.

## Impact

Ruler model and bridge, canvas input and overlay, options and Info readouts, document rotation command, ruler widgets and Qt and Rust tests. No new dependencies.
