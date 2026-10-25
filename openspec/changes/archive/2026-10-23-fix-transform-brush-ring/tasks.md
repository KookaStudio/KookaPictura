# Tasks

## 1. The regression test

- [x] 1.1 Add `freeTransformClearsTheBrushRing` to
  `crates/pictura-app/cpp/tests/tst_transform_overlay.cpp`: with Brush active
  and the ring drawn, open Free Transform and assert the outline state is
  cleared and the ring's pixel is repainted; zoom during the session and assert
  it stays hidden; cancel, hover, and assert it returns.

## 2. The fix

- [x] 2.1 Clear the outline in `ImageView::beginTransformPreview` before the
  first transform frame.
- [x] 2.2 Clear instead of drawing in `ToolController::updateBrushOutline` while
  `transformSessionActive()`.

## 3. Verification

- [x] 3.1 `ctest --test-dir build -R '^tst_transform_overlay' --output-on-failure`
  passes.
- [x] 3.2 `bash scripts/verify-fast.sh` and
  `openspec validate --all --strict` pass, apart from the pre-existing
  `pictura-render` ImageMagick oracle check (`ramp_Divide.rgba`), which fails
  identically on unmodified `master`.
- [x] 3.3 Archive the change with `openspec archive fix-transform-brush-ring -y`,
  so the pull request leaves no open change.
