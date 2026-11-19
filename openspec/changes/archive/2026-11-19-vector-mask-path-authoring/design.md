# Design: vector-mask-path-authoring

## Context

The path bridge (`cxxqt_object/paths/imp.rs`) edits one target path. Without a
layer target, the target is the Paths panel's active row (`ActivePath::Work` or
`ActivePath::Saved(i)`). With `path_on_layer` on (Path Selection, Direct
Selection, and shape mode) and a shape layer selected, `layer_target` returns
the layer instead. `path()` then decodes the layer's `vmsk`/`vsms` block into a
`VectorPath`, and `edit_target_with` re-encodes the edited path through
`set_layer_shape_paths` and recomposites only the damaged region.

The decoding and encoding work on any layer's vector-mask block, but
`layer_shape_paths` and `layer_target` gate on `is_shape_layer`.

## Decisions

**A fourth active row, `ActivePath::VectorMask`.** It means "the selected
layer's vector mask". `layer_target` returns the selected layer when:

- the active row is `VectorMask`, the selected layer is not a shape layer, and
  it has a vector mask (whatever `path_on_layer` is set to, so the Pen draws
  into the mask), or
- `path_on_layer` is on and the selected layer is a shape layer (unchanged).

A saved path or the Work Path selected over a masked pixel layer stays the
target. The mask is edited only while its row is active, as in CS6, where the
mask path shows only when it is selected. When the selected layer has no vector
mask, `VectorMask` reads as no active path, and the Pen then starts a new Work
Path as it does with nothing selected.

The bridge index for the row is `-4`; `-3` is already the panel's shape row and
`-2` means none.

**Generalise the outline accessor, not the shape predicate.**
`vector_mask_paths(layer, w, h)` decodes the effective block of any layer, and
`layer_shape_paths` keeps its shape gate by calling it. The gesture backup and
the path read use the new accessor.

**Keep the Pen's drawing session across decodes.** `VectorPath::editing` is
in-memory state, and the mask path is rebuilt from bytes on every call. The view
stores `(layer path, editing subpath)` after each layer-target edit and restores
it, through a new `VectorPath::set_editing_subpath`, when it rebuilds the same
layer's path. Changing the active row or the layer clears it.

**Preserve the flags word on rewrite.** `set_layer_shape_paths` copies bytes
`4..8` of the existing effective block into the new block. Before, an edit to a
Hide All mask lost its invert bit.

**Current Path takes subpaths.** `VectorMaskKind::Path(&[Subpath])` replaces
`CurrentPath`. The engine no longer chooses the Work Path; the app passes the
active row's subpaths. Shape rows, the mask row, an empty Work Path, and an
empty saved path all leave the command disabled.

**Thumbnail.** Each pixel is grey (`0x80`) where the mask hides and white where
it reveals. Each flattened subpath is then drawn as a 1 px black polyline,
scaled to the thumbnail (DDA lines). A disabled mask or one with no fill stays
all white with the outline. `ponytail:` the grey level and the line weight are
approximated by eye; no CS6 oracle exists.

## Risks

- Every Pen move on the mask re-encodes the block and refreshes the layer's
  damage, as Direct Selection already does on shapes. The live drag defer
  (#1185) applies to selection drags only.
- 8.24 fixed-point quantisation moves points by less than 1e-6 of the canvas
  extent per round trip. That is below what the overlay can display.
