# Proposal

## Why

Adjustment and fill masks still inherit square image cards rather than the
document shape. Plain layer-row clicks can also retain a previously active mask
thumbnail instead of returning to the left content thumbnail.

## What Changes

- Give raster and vector mask cards the document aspect for every owner kind.
- Keep mask outlines, brackets, rendering and hit targets on those same cards.
- Make a plain layer-row click activate its left content thumbnail.
- Preserve the creation rule: a newly created adjustment is selected with its
  mask active; explicit mask clicks continue to target masks.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: universal document-aspect mask cards and row-click defaults.

## Impact

Layers delegate geometry/rendering, row event handling, and Qt regression tests.
No engine, codec, dependency or file-format changes. Follow-up to #1199.
