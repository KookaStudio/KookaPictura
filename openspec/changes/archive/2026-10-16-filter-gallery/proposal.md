# Proposal

## Why

`Filter ▸ Filter Gallery…` is a disabled stub, yet CS6's canonical route to the
Artistic, Brush Strokes, Sketch, and Texture families (and Diffuse Glow, Glass,
Ocean Ripple, Glowing Edges) is the gallery: one dialog with a large preview,
the filters as thumbnails by category, the selected effect's options, and a
cumulative stack of effect layers (docs/06-filters/artistic-filters.md,
FILT-080).

## What Changes

- Add `FilterGalleryDialog`: preview pane with zoom, collapsible category
  headers over thumbnail grids (rendered by running each filter on a small
  sample of the picture), a hide-thumbnails toggle, OK / Cancel, a filter menu,
  the selected effect's options, and an effect-layer list with eye, drag to
  reorder, New effect layer, and Delete. Effects stack like layers (bottom row
  applied first). The stack previews in the dialog, OK commits it as one
  "Filter Gallery" history state, Cancel restores the layer. The gallery
  reopens on the session's last stack.
- Wire `Filter ▸ Filter Gallery…`, enabled exactly when a filter can run.
- Bridge (`filter_tools.rs`): stack preview restricted to the visible rect
  (expanded by the summed support of the stack), stack commit, and a thumbnail
  render of one filter on a small RGBA image.
- Extract the per-parameter controls of the filter dialog into
  `FilterParamControls`, shared by both dialogs (pure move).

## Capabilities

- `imaging/filter-gallery` (new): the gallery dialog and its stack semantics.

## Impact

- `pictura-app` only; no engine change, no new dependency.
- Out of scope: Alt-click to add on top, Cancel's Ctrl/Alt Default/Reset labels,
  the 8-bit gate (the app is 8-bit throughout today), Smart Object grouped
  "Filter Gallery" entries, and Diffuse Glow / Glass, which have no kernel yet
  and are left out of the Distort category until they do.
