# Proposal: mask-solo-edit

## Why

CS6 lets you view a layer mask on its own: `Alt`/`Option`-clicking the mask
thumbnail in the Layers panel shows the mask's grayscale channel on the canvas,
and clicking again restores the composite (`docs/05-layers/layer-masks.md`,
"View the mask alone"; `docs/02-ui-ux/keyboard-shortcuts.md`, "Toggle between
layer mask/composite image"). The long-form contract already describes this, but
the app never implements it: the mask-thumbnail handler in
`crates/pictura-app/cpp/panels/layers_panel_events.cpp` branches only on `Shift`
(toggle enabled) and plain click (set the mask edit target), so `Alt` falls
through to the plain-click path and the canvas never changes. Issue #436.

The same thumbnail's **right-click menu** — Disable/Enable, Delete, Apply, and
the three selection combine commands (`docs/05-layers/layer-masks.md:87-96`) — is
also missing: right-clicking a mask thumbnail currently opens the generic row
menu, which carries only Add/Delete/Enable/Disable. And `Delete`/`Backspace`
with the mask thumbnail active does not delete the mask. Both gaps are part of
the same mask-thumbnail contract and land together.

## What Changes

### Alt-click mask solo (issue #436)

- `Alt`-clicking a raster layer mask thumbnail SHALL show the mask alone: the
  canvas renders the mask's grayscale plane (value replicated across RGB,
  opaque) instead of the composite.
- The state SHALL be a transient, view-only display mode: it SHALL record no
  history state and SHALL NOT mutate, serialize, or save the document.
- A second `Alt`-click on the same thumbnail, a click on any layer or mask
  thumbnail, removing the mask or its layer, or switching documents SHALL
  restore the composite.
- The `Alt`-click SHALL also make the mask the view's mask edit target (and
  select its row, so the target routes strokes), keeping the focus border on the
  mask thumbnail — matching the existing activation contract, so the mask can be
  painted while viewed.
- `Alt`+`Shift`-click SHALL be consumed without toggling anything: it is
  reserved for the (unimplemented) rubylith overlay, so it MUST NOT fall through
  to the mask-enabled toggle or to row selection.
- The canvas SHALL present the mask plane through the existing view-pyramid
  level-crop path, so zoom and pan behave as they do on the composite and a
  large document never scales a full-resolution image per paint. Edits to the
  mask made while it is soloed SHALL refresh the displayed plane.
- The mask display SHALL be a canvas-only source: every other display consumer
  (Channels, Histogram, Navigator, filter and adjustment previews, thumbnails)
  SHALL keep reading the composite.

### Mask-thumbnail context menu

- Right-clicking a raster layer mask thumbnail SHALL open a mask context menu,
  not the generic row menu, and SHALL make that mask the edit target.
- The menu SHALL offer, in CS6 order: Enable Layer Mask / Disable Layer Mask
  (label per the mask's current state), Delete Layer Mask, Apply Layer Mask, Add
  Mask To Selection, Subtract Mask From Selection, Intersect Mask With
  Selection, and the disabled placeholders `Select and Mask…` (a CC feature) and
  `Mask Options…`; separators partition the groups exactly as CS6 shows them.
- Apply Layer Mask SHALL fold the mask's coverage into the layer's pixel alpha
  and remove the mask, and SHALL be disabled for a Smart Object layer.
- Delete Layer Mask SHALL remove the mask without applying it.
- The three selection commands SHALL load the mask's coverage (white = selected)
  into the pixel selection, combined with `Add`, `Subtract`, or `Intersect`.
- Each document-changing menu action SHALL be one undoable step.

### Delete key

- `Delete`/`Backspace` SHALL delete the active mask when the mask is the edit
  target, without applying it and without a confirmation dialog, ahead of the
  pixel-selection clear and the layer-delete branches.

## Capabilities

### New Capabilities

- `ui/mask-solo-view`: the Alt-click mask thumbnail gesture and the canvas mask
  display mode it drives, including entry, exit, the view-only guarantee,
  pyramid-backed presentation, live refresh, and canvas-only scoping.
- `ui/layers-panel-mask-menu`: the raster mask-thumbnail right-click menu, its
  item set and order, the Apply/Delete/selection semantics, the disabled
  placeholders, and the Smart-Object refusal for Apply.

### Modified Capabilities

- `ui/layers-panel`: the "Layer mask row indicators" requirement gains the
  `Alt`-click clause (mask solo view) and the `Alt`+`Shift` reservation, so the
  enumeration of mask-thumbnail click gestures stays exhaustive.
- `ui/delete-key`: the delete requirement gains the mask-edit-target branch that
  deletes the active mask ahead of the selection/layer branches.
- `compositing/layer-masks`: a new requirement that a layer mask's coverage can
  be loaded as a pixel selection (white = selected), supplying the selection
  commands.

## Impact

- `crates/pictura-select/` — `selection_from_mask` over a `LayerMask`.
- `crates/pictura-app/src/cxxqt_object/` — `state.rs` (the view-only solo field
  and its mask display cache), a new `impl_core/display.rs` (the canvas display
  source; the composite display accessors move there so `impl_core.rs` stays
  under its size cap), `impl_core.rs` (hooks in the present/pyramid paths,
  `reset_edit_state`), `impl_paint.rs` (a region hook), `impl_selection.rs`
  (the mask-to-selection bridge), `impl_layers/layer_masks.rs` (the mask bridge:
  `mask_solo_*`, `layer_mask_apply_path`, the apply/delete reads).
- `crates/pictura-app/cpp/` — `frame_canvas.cpp` (the provider crop switches to
  the canvas display source), `panels/layers_panel_events.cpp` (the guarded
  gesture branch and the mask-thumbnail right-click route),
  `panels/layers_panel_menu.cpp` (the mask menu), `panels/layers_panel_actions.cpp`
  (`clearSolo` clears the Rust state; `deleteActiveMask`), `frame_menus_edit.cpp`
  (the Delete-key mask branch), `panels/layers_panel.h` + a new
  `panels/layers_panel_test_solo.cpp` (the test seams; kept out of the capped
  `layers_panel_test.cpp`).
- `CMakeLists.txt` (add the new `.cpp`) and `crates/pictura-app/cpp/tests/tst_mask_edit.cpp`
  (the Qt Test). No `build.rs` change: the bridge methods live in the existing
  listed `impl_layers/layer_masks.rs`.
- `docs/` — document the mask-thumbnail context menu, the delete-on-Delete-key
  behavior, and the load-as-selection row (`TASK-ALLOWS-DOCS`).
- No new dependency.
