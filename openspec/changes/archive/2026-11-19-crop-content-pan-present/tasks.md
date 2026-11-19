# Tasks

## 1. Tests

- [x] 1.1 Add `cropPanPresentsThroughALevel` to `crates/pictura-app/cpp/tests/tst_present_cache.cpp`: resize/show a standalone `ImageView`, a fake `LevelProvider` recording each crop request and each `reportView` rectangle, a 4000×3000 document, `setCropContentOffset(100, 100)`, renders at zoom 1.0 and 0.4 plus one with `setPresentLevelCropForTest(false)`; assert every request fits the viewport in document pixels plus the sampling margin (one level pixel per edge plus rounding), no request covers the whole level 0, the reported rectangle moves back by the offset, and a 520×120 document at 100 % still reports its whole rectangle. Verify with `python3 scripts/check-test-plan.py`.
- [x] 1.2 Add `cropPanMatchesAViewPan` to the same suite: paint a non-uniform red dab, render at zoom 1.0 and 0.4, render with `setCropContentOffset(10, 10)`, assert the rebuild count stays flat when the offset moves, reset the offset and `panBy(+10·zoom, +10·zoom)`, and assert pixel equality with the content-offset frame and with the direct level-0 draw (`setPresentLevelCropForTest(false)`). Verify with `python3 scripts/check-test-plan.py`.
- [x] 1.3 Add `cropPanKeepsTheSingleChannel` to the same suite: 520×120 document, 100 px red dab at the centre, zoom 1.0; assert the widget centre is pure red and a point past the dab radius is white, then `setChannelMask(0x2)` with `setCropContentOffset(10, 10)` and assert that same point turns black, so the channel moved with the content. Verify with `python3 scripts/check-test-plan.py`.
- [x] 1.4 Register the three slots in the test class declaration and confirm `scripts/check-test-plan.py` maps all three scenarios to existing Qt slots.

## 2. Present path

- [x] 2.1 Factor the unclipped `viewportDocumentRect()` out of `visibleDocumentRect()` (`image_view_document.cpp`), add `presentVisibleRect()` in `image_view_present.cpp` as that rect shifted by `-cropContentOffset_` and then clipped to the document, and use it from `presentLevelRect` and the two paint fallbacks; the public `visibleDocumentRect()` keeps its clip. Verify `rg -n "presentVisibleRect" crates/pictura-app/cpp` shows only `image_view.h`, `image_view_present.cpp`, and `image_view.cpp`, and that no dialog/filter file changed.
- [x] 2.2 In `ImageView::paintEvent`, delete the `image()` content-offset branch and fold `cropContentOffset_ * zoom_` into the existing single-channel, crop, and fallback draw targets through one `docOrigin` local; verify `rg -n "image\\(\\)" crates/pictura-app/cpp/image_view.cpp` shows no call left in the paint path.
- [x] 2.3 Confirm `image_view.cpp` stays under its 1200-line cap with `bash scripts/check-file-size.sh`.

## 3. Documentation

- [x] 3.1 Remove the crop content pan from the remaining `ponytail:` debt list in `docs/dev/STATE.md`; commit with `TASK-ALLOWS-DOCS` and verify `bash scripts/guard.sh` passes.

## 4. Verification

- [x] 4.1 Build and run the suite: `cmake --build build --target tst_present_cache` then `QT_QPA_PLATFORM=offscreen python3 scripts/cpu-budget.py exec -- ctest --test-dir build -R '^tst_present_cache$' --output-on-failure`. 30/31 pass; the only failure is `frameBoundedPaintMatchesCpu:gpu`, confirmed pre-existing because it also fails with the present-path change reverted (this machine's software Vulkan).
- [ ] 4.2 Run the full gate under the CPU budget (`python3 scripts/cpu-budget.py exec -- bash scripts/verify-fast.sh`); locally the targeted equivalent passed: `openspec validate --all --strict`, `guard.sh`, `check-file-size.sh`, `check-test-plan.py`, the crop and transform-distort suites, and `./build/pictura --headless --self-test` (467 passed, 0 failed).
- [x] 4.3 Record the profile note for the PR: `cropPanMatchesAViewPan` prints `pictura profile: crop_pan 4000x3000 full_document_crop_ms=218.12 pan_frame_ms=110.33` on this machine (a second run: 467.52 / 162.85). The full-document crop is the exact per-frame work the old branch did, so it stands in for the "before" number; note the software-rendered environment in the PR.
