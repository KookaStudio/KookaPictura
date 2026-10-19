# Proposal

## Why

With a layer mask as the edit target and a selection active, the Move tool runs
the pixel-content move: it cuts the layer's pixels and folds the mask into alpha
during the merge, so the mask is lost and the image moves. Photoshop CS6 instead
moves the selected mask coverage, fills the vacated area with the mask background
colour, and leaves the layer's pixels and the mask intact.

The app's mask colour pair also starts inverted relative to CS6 (black foreground
/ white background), so a correct background fill would use the wrong default.

## What Changes

- Route the Move tool's selection drag and the selection tools' Ctrl content move
  to the mask's coverage when a raster layer mask is the edit target.
- Translate the selected mask coverage by `(dx, dy)` inside the mask rectangle,
  blend the vacated coverage toward the mask background's luma, and composite
  the moved coverage at the destination. The mask stays attached; the layer's
  pixels are untouched.
- Alt-drag copies the selected coverage within the mask instead of clearing the
  source; it inserts no layer, changes no active layer, and shows no pixel-clone
  preview.
- Record exactly one `Move Selection` history state; keep the selection at the
  drag offset.
- Align the mask's default colour pair with CS6: white foreground / black
  background, so the default vacated area is black. Reset (`D` and the toolbox
  reset icon) follows the active target's defaults.
- Ensure the selection is translated to the release offset before committing a
  content move, so a release without a final pointer move still leaves the
  selection on the moved coverage.
- Move the selection-move bridge declarations out of the capped
  `cxxqt_object.rs` so the added background argument keeps the file under the
  hard size limit.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/selection-content-move`: adds mask-target content move semantics
  (coverage translation, background fill, copy mode, single history state) and
  scopes the existing pixel Alt-copy and Alt-preview requirements to non-mask
  targets.
- `compositing/layer-masks`: the initial mask colour pair becomes white foreground
  / black background, and resetting colours follows the active target.

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/mask_edit.rs` and its exports:
  new mask-coverage move engine.
- `crates/pictura-app/src/cxxqt_object/impl_selection.rs` and a new selection
  bridge module: route the content move, pass the mask background luma, and keep
  `cxxqt_object.rs` under its size cap.
- `crates/pictura-app/cpp/tools_selection_move.cpp` and `tools.cpp`: supply the
  current background colour and translate the selection before commit.
- `crates/pictura-app/cpp/frame.h`, `frame_build.cpp`, `toolbox.cpp`,
  `panels/color_panel.h`: target-aware colour defaults and reset.
- `crates/pictura-app/cpp/tests/tst_mask_edit.cpp`: mask colour-pair, reset, and
  move coverage tests.
