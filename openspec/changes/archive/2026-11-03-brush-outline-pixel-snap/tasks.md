# Tasks

## 1. Regression tests

- [x] 1.1 Add `crates/pictura-app/cpp/tests/tst_brush_outline.cpp` with the `paint_fixture::Fixture` harness and slots `ringTracksBrushSize`, `ringDiameterMatchesZoom`, `ringEdgeSnapsToScaledPixelBoundary`, `ringHaloIsOneDevicePixel`, `subPixelBrushKeepsAOnePixelRing`, `ringDrawsPastTheDocumentEdge`, and `noRingForOtherTools`; declare them as `private slots` in the class in `tst_brush_outline.cpp`, define them in the same file, and add `QTEST_MAIN` plus `#include "tst_brush_outline.moc"`.
- [x] 1.2 Register `tst_brush_outline` in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify the name is in `PICTURA_QT_TESTS`.
- [x] 1.3 Make `ringDiameterMatchesZoom` loop over zooms 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4 with `D=8` (all products are even) and assert the outer dark-edge distance `hi - lo` on the centre row equals `round(D × zoom)` and is symmetric about the snapped centre. Add a separate rounding case `D=10` at zoom 1.25 (`12.5 -> 13`) asserting `abs((hi - lo) - 13) <= 1` with no symmetry claim. Compute the snapped centre from `offset()` and `zoom()`; the suite runs with the view unrotated.
- [x] 1.4 Make `ringEdgeSnapsToScaledPixelBoundary` assert each cardinal stroke is one device pixel thick and `hi - lo` matches `round(D × zoom)`; use `D=8` so the even rounded diameters allow exact assertions, and note that a 1-px cosmetic stroke makes the span `hi - lo + 1` equal `round(D × zoom) + 1`, which is why the test measures `hi - lo`.
- [x] 1.5 Update the ring sample points in `tst_canvas_pan::panHidesTheBrushRing` and `tst_transform_overlay::freeTransformClearsTheBrushRing` to use the snapped centre (or a small scan); verify both still reference `brushOutlineScreenDiameterForTest`, and confirm `tst_eraser_tools` still passes because the Eraser Block's size requirement is unchanged and only its centre snaps.

## 2. Implementation

- [x] 2.1 Add a private `brushOutlineDeviceDiameter()` helper returning `std::max(1.0, std::round(brushOutlineDiameter_ * zoom_))`; use it from both `paintEvent` and `brushOutlineScreenDiameterForTest()` in `crates/pictura-app/cpp/image_view.h`, and update the accessor's comment. Keep the helper in `image_view.cpp`; `image_view.h` needs no `<cmath>`. Verify no other helper still claims the unrounded value.
- [x] 2.2 Rewrite the ring block in `crates/pictura-app/cpp/image_view.cpp` (`paintEvent`) as an explicit block: `painter.save(); painter.setTransform(viewRotation()); painter.setClipping(false); … painter.restore();`. The ring block is not inside an existing save/restore, so the restore is required or the overlays that follow draw in the wrong transform. Snap the centre to whole screen pixels, draw at `brushOutlineDeviceDiameter()`, and use `halo = 1.0 / devicePixelRatioF()` for the circle and square; verify the block no longer reads `1.0 / zoom_` for the halo.

## 3. Verification

- [x] 3.1 Build once with `cmake --build build --parallel "$(nproc | awk '{print int($1*0.8)}')"` and run `ctest --test-dir build -R '^tst_brush_outline|^tst_eraser_tools|^tst_canvas_pan|^tst_transform_overlay' --output-on-failure`; verify every selected suite passes.
- [x] 3.2 Run `bash scripts/verify-fast.sh`; verify formatting, clippy, the test report, file-size/guard checks, the test-plan check, and `openspec validate --all --strict` pass.
- [x] 3.3 Run `openspec archive brush-outline-pixel-snap` in the shipping pull request and verify the delta merged into `openspec/specs/tools/brush-tools/spec.md`.
