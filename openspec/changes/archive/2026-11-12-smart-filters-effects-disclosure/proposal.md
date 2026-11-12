# Proposal

## Why

Pictura renders the synthetic Smart Filters and Effects rows with the same
left-side group chevron used by layer groups. Photoshop puts the
list toggle on the owning layer row's right edge, nests the list inside the
layer's cell, and paints a faded eye when a row is hidden. The current layout
misreads the hierarchy, gives a layer with both lists two separate toggles, and
makes the fx control look like a second group disclosure.

## What Changes

- A layer exposes exactly one right-edge list toggle for its synthetic lists.
  Opening it shows every group the layer owns (Effects, Smart Filters, or both);
  closing it hides them all. It points down when closed and up when open.
- Closing the list hides the whole synthetic group, header included.
- The synthetic group header no longer carries a disclosure, and the synthetic
  rows no longer draw the `fx` thumbnail.
- The synthetic list nests inside the layer's cell: every nested row's eye
  starts at the owning layer's image-thumbnail right edge, each child row sits
  one indent step deeper, the eye-to-name spacing is halved, and a hidden row
  paints the eye-off glyph at 50% opacity while keeping the eye slot clickable.
- Clicking a synthetic row outside its eye selects and highlights the owning
  layer, not the synthetic row.
- The fixed left eye cell is a flat fill one shade lighter than the row with the
  eye centred in its gutter; it keeps its right border through the layer + list
  block, and the right cell spans the same block. The nested rows and the
  Effects/Smart Filters groups draw no horizontal rule between them; the block's
  bottom rule closes the block after its last row.
- The layer-row lock badge is smaller, and the fx badge and clipping-mask glyph
  shrink with it so the right-edge run stays clear.
- A group (folder) row keeps its left nested-layers chevron; the list toggle is
  independent of it.
- Smart-filter rows keep their drag-to-reorder behavior inside the nested list;
  the group header remains non-draggable.
- **BREAKING**: none; the document model and bridge APIs are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: the layer-row visibility gutter and eye anchoring, the
  layer-row badges/delegate thumbnail treatment, the Smart Filters tree rows,
  the layer effects disclosure rows, and the effects row interactions all change
  to the nested-cell layout described above.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_internal.h` — delegate geometry,
  painting, and hit rectangles.
- `crates/pictura-app/cpp/panels/layers_panel_events.cpp` and
  `layers_panel_menu.cpp` — eye/toggle hit testing.
- `crates/pictura-app/cpp/panels/layers_panel.cpp` and
  `layers_panel_effects.cpp` — per-layer list-expansion state and synthetic-row
  inclusion.
- Qt tests: `tst_layers_effects`, `tst_layers_smart_filters`,
  `tst_layers_panel`; test hooks in `layers_panel_test.cpp` and
  `layers_panel_test_extra.cpp`.
- No new dependencies, no document-format change, no bridge API change.
