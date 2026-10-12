# Tasks: transform-overlay-unclipped

## 1. Paint

- [x] 1.1 Extract the transform quad/handles/knob/pivot drawing from
  `paintEvent` into `ImageView::paintTransformOverlay` (a pure move).
- [x] 1.2 Call it after the document-clipped overlay scope, so the box paints
  past the document edge while the preview stays clipped.

## 2. Verification

- [x] 2.1 `tst_transform_overlay`: a controls-box drag scales a corner outside
  the canvas; the off-canvas corner's handle pixel equals the on-canvas
  corner's and is the white handle fill (fails with the document clip
  restored).
- [x] 2.2 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo nextest run --workspace`, `ctest --test-dir build -R
  '^tst_'`, `bash scripts/verify-fast.sh`, `openspec validate --all --strict`.
  The unified report's only failure is the pre-existing local ImageMagick
  fixture mismatch, which fails on master too.
