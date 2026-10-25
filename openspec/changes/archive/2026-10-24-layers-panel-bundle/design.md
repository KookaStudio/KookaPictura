# Design: Layers panel interaction bundle

## Root causes and fixes

### #900 — canvas backdrop

`ToolController::updateCanvasBackdrop` (`cpp/tools.cpp:535-545`) marks the
canvas opaque whenever the bottom layer's kind is `background`, and
`ImageView::paintEvent` (`cpp/image_view.cpp:595-602`) fills the document rect
with the secondary colour instead of the checkerboard. The eye click uses the
region path (`regionBlitted`), which never calls `bindCanvas`, so the backdrop
is not re-evaluated anyway. Remove the special case: delete `canvasBackdrop_`,
`canvasBackdropOpaque_`, `ImageView::setCanvasBackdrop`, and
`ToolController::updateCanvasBackdrop`; the paint path always uses
`transparencyTile()`. A visible Background covers the checkerboard through its
opaque pixels, and a hidden Background reveals it. This is smaller than adding
a visibility check to a path that is not reached.

### #898 — anchored creation

`PictureView::add_adjustment` (`impl_filters.rs:44-56`) pushes onto
`doc.layers`; `add_solid_fill` and `add_gradient_fill`
(`impl_layers_rasterize.rs:56,77`) pass `""` to the render helpers, which means
"top". Both bypass the tree-aware insertion rule. The fix is one new public
render helper wrapping the existing `insert_node` and three call-site changes:

- `pictura_render::insert_layer(doc, selection_path, layer) -> String`.
- The bridge reads `rust.layer_target.anchor()` as `selection_path`; it never
  writes the target from C++.
- The created path is selected with `select_only_layer`.
- Fills also select the created path; today they leave the target alone.

### #897 — group masks

`add_layer_mask` sizes Reveal All / Hide All / From Transparency to
`layer.rect`. A group has a zero rect (`empty_group`, `create.rs:64-72`), so its
mask has a zero rectangle: `mask_thumbnail_image` returns `None`, the delegate's
`maskThumbRect` is empty, and `mask_document`/`write_mask_back` refuse a
zero-sized mask. Size those three kinds to the document rectangle when
`layer.is_group`; selection-driven masks keep the selection rectangle. A group
mask then has a thumbnail, a click target, and editable coverage.

### #894 — thumbnails

`layer_thumbnail` and `layers_snapshot` return an empty image for every layer
with `adjustment.is_some()` (`impl_layers.rs:333-343,376-381`). The delegate
draws a checkerboard and then the (null) image, so adjustments and fills look
like empty pixel layers.

- Adjustment rows get an `adjustment_kind` field on the Rust `LayerRow`, the C++
  `layers_api::LayerRow`, the model role, and `LayerRowNode`. The delegate
  paints `pictura::icon("adjustments." + kind)` on the row's card instead of the
  checkerboard. Shape rows keep their shape thumbnail and are excluded.
- Fill rows get a preview image from Rust:
  - Solid Fill: a flat RGBA fill.
  - Gradient Fill: the existing `gradient_rgba` evaluated at thumbnail extent
    (expose it `pub(crate)`), preserving kind/angle/scale/reverse.
  - Pattern Fill: a scaled nearest-neighbour sample of the document pattern
    tile. The snapshot decodes `pictura_codec::decode_patterns(doc)` once when
    the first pattern row is seen and passes the slice to the preview helper.
- Pixel, type, smart-object, and group behavior is unchanged.

### #895a — mask preferences and target

CS6 has two preferences: `Add Mask by Default` (Adjustments panel) for
adjustments, and `Use Default Masks on Fill Layers` (Layers Panel Options) for
fills. The code has one `use_default_masks` flag and consults it in both
`add_adjustment` and the fill creators. Split it:

- `PictureViewRust` gains `add_mask_by_default: bool` (default true), pushed
  from the session through `set_layers_panel_options` (extended) and persisted
  beside `layersUseDefaultMasksOnFill`.
- The Adjustments panel menu row becomes a checkable action
  (`addMaskByDefault`) routed through the Layers panel, which owns the view.
  The menu's `aboutToShow` hook sets the checked state.
- A new `default_layer_mask(doc, selection)` helper returns the selection mask
  when one exists, else a white document-sized `LayerMask` (rect = document
  bounds).
- `add_adjustment` uses `add_mask_by_default`; fills use `use_default_masks`.
- After creation, an adjustment with a mask sets `mask_edit_target` to its
  path. Fills leave the layer thumbnail active.

### #720 — row menu

Replace the flat `kRowSpecs` plus the `commonEnd`/`exportBegin` slicing with a
`block` field on each `RowSpec`. `populateRowMenu` walks the table and emits one
separator between non-empty blocks. The row table (non-Background kinds) is:

| Block | Rows | Kinds |
|---|---|---|
| head | Rename | all non-Background |
| 1 | Blending Options…, Edit Adjustment… | style-capable; adjustment |
| 2 | Duplicate, Delete, Group from Layers…, New Layer, New Group, Ungroup, Move Up/Down | all |
| 3 | Quick Export as PNG, Export As… | pixel, smart |
| 4 | Convert to Smart Object | pixel, Background |
| 5 | Rasterize Type / Shape / Layer, Rasterize Layer Style (disabled) | per kind |
| 6 | Add/Delete/Enable/Disable Layer Mask, vector-mask rows, Create/Release Clipping Mask | pixel, group (masks); pixel (clipping) |
| 7 | Link/Unlink Layers, Select Linked Layers | all |
| 8 | Copy/Paste/Clear Layer Style | styled-capable |
| 9 | Copy/Paste Shape Attributes, Rasterize Shape | shape |
| 10 | Release from Isolation (only while isolation is active) | all |
| 11 | Merge Down / Merge Layers, Merge Visible, Merge Clipping Mask, Flatten Image | pixel, group |
| 12 | Color Label submenu | all non-Background |

The Background is a separate brief table: `Layer from Background…`,
`Duplicate Layer…`, `Delete Layer` (disabled), `Quick Export as PNG`,
`Export As…`, `Convert to Smart Object`. It has no Rename, Blending Options,
color label, merge, or flatten.

Most actions already exist in `performPanelMenuAction`; `performRowAction`
delegates to it for `groupFromLayers`, `link`, `selectLinked`, `mergeDown`,
`mergeVisible`, `mergeClippingMask`, and `flatten`. New bridge work:

- `vector_mask_add_path` (the add bridge is active-layer-only today).
- `rasterizeType` becomes enabled through `rasterize_type(path)`.
- `Rasterize Layer` dispatches to `rasterize_type`, `rasterize_shape`,
  `rasterize_smart_object`, or `rasterize_fill_content` by kind.
- `Link Layers` toggles to `Unlink Layers` when the row is linked.
- `Merge Down` becomes `Merge Layers` for a multi-selection; `canMergeSelection`
  drives enablement.

`showContextMenu` selects the right-clicked row when it is outside the current
selection, so row actions act on the clicked row.

## Test-file split

`tst_command_tree.cpp` (1396 lines) and `tst_layers_panel.cpp` (1399 lines) are
at the 1400-line test cap. Move the row-menu assertions into a new
`tst_layers_row_menu.cpp` and the new #895/#898 cases into
`tst_layers_panel_bundle.cpp`; register both in `cpp/tests/CMakeLists.txt`.
No allowlist entry is added.

## Risks

- `layers_panel.cpp` is 1101 lines; keep the new preference plumbing in
  `layers_panel_actions.cpp`/`panel_group_menu.cpp` where possible.
- The row-menu table is behaviorally dense; the per-kind Qt tests are the
  contract.
- Group mask sizing is documented as document-sized (inferred from CS6; the
  mask outside its rectangle is reveal-all, so the visual result is the same).
