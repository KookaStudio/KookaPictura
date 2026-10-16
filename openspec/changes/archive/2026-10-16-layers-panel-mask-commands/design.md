# Design

## State

- **Row menu:** two new path queries on the mask bridge,
  `layer_mask_present_path` and `layer_mask_disabled_path`, drive four new
  `RowSpec::Enable` values.
- **Mask button:** reads the active layer's `layer_mask_present` and
  `vector_mask_present` in `syncControls`, which every panel refresh runs. The
  state therefore follows add, delete, undo and layer selection.
- **Overlay and display options:** these are view state on `PictureViewRust`,
  not document state:
  - `mask_overlay: Option<String>` (the layer path);
  - `mask_display: (argb, opacity)`;
  - `mask_overlay_revision`, bumped on every change.

  ponytail: the display options are per open document and are not written to
  the PSD; the spec leaves CS6's round-trip unstated.

## Mask menu

The mask-thumbnail menu itself landed in #675 (`ui/layers-panel-mask-menu`,
`layers_panel_menu.cpp`), including mask-to-selection through
`PictureView::mask_selection`. This change edits that menu rather than adding
a second one:
- `Select and Mask…` (CC-only per the docs) becomes Refine Mask…, opened by
  the panel; the panel's `selectAndMaskRequested` signal goes.
- Mask Options… opens `MaskDisplayOptionsDialog`.
- Apply is enabled only on a layer kind that renders from its own alpha (not
  `group` or `adjustment`, which covers fill layers, and not a smart object).
  `apply_layer_mask` refuses the same kinds before taking the mask, so the
  bridge cannot drop a mask that was hiding content.

The mask button and the two dialogs' launchers live in
`layers_panel_mask_menu.cpp`.

## Refine Mask

The refine bridge gains `refine_mask_available`, `refine_mask_preview` and
`refine_mask_apply`, all taking the layer path. The existing preview and apply
bodies are split into helpers that take the source selection and the target
layer:
- Refine Edge passes the active selection and the active layer.
- Refine Mask passes the mask's stored coverage and the clicked layer. It reads
  the coverage even when the mask is disabled (`selection_from_mask` would read
  a disabled mask as fully selected, which would overwrite it with white).

A preview borrows the selection, or samples the mask with `coverage_at`, at the
preview size; only the commit builds a document-sized coverage. A Layer Mask
output from Refine Mask replaces the mask of any layer that has one; Refine
Edge still adds one only to a pixel layer.

When a refine writes a Layer Mask output into a layer that already has a mask,
the existing mask's flags, density and feather are kept and only its
rectangle, default colour and data are replaced. Before this change Refine
Edge reset the flags too, which unlinked the mask.

`RefineEdgeDialog` takes an optional mask path. With one, it is titled
"Refine Mask", previews from the mask, and preselects Layer Mask output.

## Rubylith overlay

`mask_overlay_image(view, max_side)` renders the overlay as a premultiplied
image of the whole document, at most `max_side` px on its long side
(nearest-sample). Each pixel is the rubylith colour at
`opacity × (255 − coverage) / 255`, so hidden areas are tinted and revealed
areas are clear. `PicturaMainWindow::refresh` resyncs it when the overlay
revision or the canvas revision changes. `ImageView::setMaskOverlay` draws it
scaled over the document rectangle, the way the filter preview is drawn.

ponytail: the overlay is capped at 4096 px on its long side, so it is soft at
high zoom on very large documents. It is rebuilt on each `changed`, not per dab,
so a live mask stroke updates the overlay at the commit.

The overlay's layer is a positional path. Like mask solo it belongs to the
current layer: Alt+Shift-click selects the row, `syncMaskSolo` ends the
overlay when another layer becomes current, `clearSolo` ends it on every
structural edit (add, delete, group, move), and a document reset clears it, so
a stale path never lands on another layer's mask.

The overlay maps its columns to document x once per render rather than once
per pixel.
