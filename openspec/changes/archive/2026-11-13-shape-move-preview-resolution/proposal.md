# Shape Move preview at the view's level

## Why

Issue #1190: on a 17000 × 17000 document, pressing the Move tool on a shape
layer still renders the shape and its backdrop at full resolution. A shape over
half the canvas costs two region composites of its reach (about 65 MP), taking
1.6–1.9 s, or 3.7 s when the shape carries a stroke. The large-layer view
preview from #1148 already follows the canvas's presented level, but it refuses
shape layers because their moving image is not a pixel plane to reduce.

## What Changes

- **A shape previews from the view.** The Move view preview accepts one shape
  layer whose appearance is its solid fill, vector mask, and layer effects. Its
  content is the shape's reach (outline box, effects, and clipped run), and the
  moving image is a reduced composite of the shape layer alone.
- **Effect sizes follow the level.** A shape's layer-effect sizes are scaled by
  the preview level before the reduced composite, so a stroke keeps its document
  width instead of rendering at full size on a coarser image.
- **The backdrop check ignores the hidden layer.** Above level 0 a hidden layer
  draws nothing, so it no longer disqualifies an otherwise faithful reduction.
- **The drag follows the shape's reach.** A drag that carries the shape's reach
  past the moving image's region rebuilds the image around the shifted view,
  using the reach rather than the canvas-sized layer rect.
- **The release stays off the GUI thread.** A shape move whose before/after
  reach union reaches 16 MP already takes the deferred Move composite path
  (#1290); a regression test now covers the shape case explicitly.

## Capabilities

- **MODIFIED** `tools/move-press-responsiveness`
- **MODIFIED** `tools/move-commit-responsiveness`

## Impact

- `pictura-render`: `filter_proxy.rs` — a hidden layer no longer disqualifies a
  faithful reduction.
- `pictura-app` Rust: `impl_transform/session/view_preview.rs` — shape content
  and moving region; `impl_transform/session/preview.rs` — effect scaling in the
  reduced moving image; `impl_transform/session/view_preview_tests.rs`.
- `pictura-app` C++ tests: `tst_move_feedback.cpp`, `tst_large_document.cpp`.
