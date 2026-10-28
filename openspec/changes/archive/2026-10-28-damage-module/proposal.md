# Proposal

## Why

How far a layer's pixels reach is decided in several separate places, and every
place that cannot prove a bound falls back to a full composite. Deleting,
reordering, grouping, typing, dragging a mask-feather slider, or toggling a
smart filter rebuilds a 6000×4000 document for an edit that touched a small
layer — roughly 0.5–1.5 s on CPU. History capture compounds it: after every
commit it compares every unstamped plane in full even when the edit already
declared its rectangles, so a paint stroke pays an O(document) scan per commit.

## What Changes

- Add a damage module in `pictura-render` that answers which document pixels a
  layer can affect: content, effect reach, mask extent, vector-mask box, and
  the contiguous clipped run above it.
- A structural edit carries an optional bounded damage. Callers declare the
  paths that exist on each side: a path created by the edit contributes only on
  the after side, a removed path only on the before side, and an unexpectedly
  missing path on a side where it should resolve is unbounded. When bounded and
  allowed, the edit transaction clears positional links and region-composites
  only the damage instead of running a full rebuild.
- Bound the full-composite sites whose affected paths are known: stack reorder,
  delete, duplicate (including batch duplicate), group (including Group from
  Layers), ungroup, arrange, reverse; merge down, selected, and clipping;
  non-neutral insertion (including Layer via Copy/Cut); rasterize (including
  Rasterize All Layers), smart-object, type, shape, warp, and smart-filter
  conversion edits; `batch_changed` (layer and vector masks, styles, clipping,
  align/distribute, hide); mask-refine previews; and smart-filter toggles.
- Keep the full recomposite where the reach cannot be proven, where the
  compositor reads a global base, or where the regional path would be unsafe:
  flatten, Merge Visible, stamp visible/selected, canvas-sized fill creation,
  duplicate-to-document, document geometry, mode and HDR conversion, unmasked
  or non-255-density adjustments, a knockout base change, a soloed mask, a
  live gesture or uncommitted preview, and a damage area of at least three
  quarters of the canvas (the sum of the clipped disjoint rectangle areas, not
  their bounding box).
- Extend a bounded region edit with a history hint. Capture compares only tiles
  intersecting the hint. A paint commit hints the whole stroke, not only the
  region still pending at commit; a move commit hints its source/destination
  union; an empty rectangle list is no hint; and document channels are hinted
  through their canvas origin. The hint must cover every change since the last
  captured state; a non-covering hint is a caller contract violation.
- When a tracked plane's count, stride, height, or order changes, history keeps
  its full-segment fallback. Structural edits whose plane geometry is stable
  use the hint; delete, duplicate, group, ungroup, reorder, and merge may start
  a full history segment even when their composite is bounded.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/described-damage`: reorder and merge are no longer
  unconditionally unbounded; add the structural influence bound, its
  before/after one-sided paths, its clipped-run carry, its full-path
  exceptions, and the requirement that the bound covers the actual pixel diff.
- `document/edit-transaction`: a link-invalidating outcome may carry bounded
  damage and then uses the regional path; the transaction forwards the edit's
  changed rectangles to history capture, including move commits, and treats an
  empty rectangle list as no hint.
- `document/edit-history`: history capture may be restricted to declared tiles;
  hinted and unhinted captures must produce identical deltas when the hint
  covers every change, and a non-covering hint is a caller contract violation.

## Impact

- New: `crates/pictura-render/src/damage.rs`; touched: `layer_effects`,
  `mask_refine`, and the app's existing `refresh_regions` path in
  `impl_core.rs`.
- App transaction and call sites: `cxxqt_object/edit.rs`, `impl_core.rs`,
  `impl_layers*.rs`, `layer_arrange.rs`, `impl_layers_merge.rs`,
  `impl_layers_rasterize.rs`, `impl_layers_smart_object.rs`,
  `impl_layers_create.rs`, `impl_layers_select.rs`, `layers_smart_filters.rs`,
  `shapes.rs`, `type_tools*`, `impl_layers/mask_refine.rs`, `impl_paint.rs`.
- History: `crates/pictura-app/src/history.rs`, `history/planes.rs`,
  `document_session/session.rs` (`record_hinted`/`record_move_with`).
- No new dependencies. No C++ API change; every commit still emits at most one
  region blit.
- The rendered pixels do not change. The change also corrects existing
  under-bounds in adjustment reach (density, feather, vector masks) so a bounded
  refresh cannot leave stale pixels.
