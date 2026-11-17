# Proposal

## Why

Engine callers currently choose among `Selection`, synthetic `LayerMask`,
`Plane<u8>`/`TiledMask`, and flat slices to express the same editing coverage.
Issue [#1346](https://github.com/KookaStudio/KookaPictura/issues/1346) introduces
one reusable core interface so selection-constrained operations do not depend
on invented layer-mask metadata or duplicate coordinate and storage rules.

## What Changes

- Add a core coverage view with signed document-space bounds, shared 8-bit
  plane storage, an explicit exterior value, and tile-safe reads.
- **BREAKING**: migrate engine selection inputs in render and paint to this
  view, including the clipboard and healing consumers confirmed in exploration.
- Project selections, alpha channels, Quick Mask coverage, and real raster-mask
  samples through the same interface. Preserve the distinction between no
  constraint and an explicitly empty selection.
- Remove `selection_to_mask`, `selection_mask_for`, and their production uses.
  Rebase coverage for mask-local edits without copying a canvas-sized selection.
- Construct a `LayerMask` only when attaching a real persistent mask. Keep
  density, feather, enabled state, flags, and native-depth interpretation with
  mask-aware operations.
- Preserve pixels, fractional blending, refusals, undo behavior, sparse tile
  fast paths, and zero-materialization guarantees. Add direct coverage tests
  and a feathered sparse-selection regression against flat storage.

## Capabilities

### New Capabilities

- `document/coverage-view`: the new public engine coverage interface, including
  its coordinate, exterior, sparse-storage, and consumer interoperability
  contract. This is a new callable core capability; existing user-visible edit
  requirements remain unchanged.

### Modified Capabilities

None. The existing selection, clipboard, healing, paint, layer-mask,
mask-refinement, and tiled-reader behavioral requirements remain in force.

## Impact

- `pictura-core`: coverage type, validation, streaming reads, and raw raster-mask
  projection; `TiledMask` may remain a storage alias inside algorithms.
- `pictura-select`: selection/channel projection without moving selection math
  into core or changing selection storage.
- `pictura-render`: filter/adjustment and Apply Image constraints, filter proxy
  coverage, mask creation, matting, Layer via Copy/Cut, selected-content moves,
  clipboard and selection-gated smart-filter paths. Real mask refinement still
  receives actual mask metadata.
- `pictura-paint`: fill/gradient/bucket, stroke clips, eraser, Patch and
  Content-Aware Move selection inputs and their app callers.
- `pictura-app`: adapter removal, owned preview/stroke/job snapshots, Quick
  Mask and alpha-channel producers, persistent-mask construction, and regression
  checks. No GUI commands or PSD format changes.
- No new dependencies, Adobe parity claims, golden-baseline changes, or
  `docs/` changes. Engine interface changes are migrated atomically within the
  workspace; no compatibility adapter accepting a synthetic `LayerMask` remains.
