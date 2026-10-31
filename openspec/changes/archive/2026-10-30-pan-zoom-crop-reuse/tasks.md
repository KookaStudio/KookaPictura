# Tasks: pan-zoom-crop-reuse

## 1. Test plan and tests first

- [x] 1.1 Write `test-plan.md` mapping every scenario in
  `specs/ui/document-canvas/spec.md` to a coverage cell.
- [x] 1.2 Add the deterministic Qt case
  `tst_present_cache::panAndZoomReuseTheLevel0Crop` (failed before the fix).
- [x] 1.3 Add the print-only pan-then-zoom timing case
  `tst_present_cache::panThenZoomFrameTimes` (profile evidence).

## 2. Present-cache reuse

- [x] 2.1 Record the level-space rectangle (`sourceRect`) in `PresentCache` and
  keep it coherent in `presentCrop`, `dropPresentCache`, and `damageRegion`.
- [x] 2.2 Add the reuse-and-patch path to `ImageView::presentCrop`: when the
  frame, revision, and level match and the new level rectangle overlaps the
  cached one, copy the overlap and crop only the exposed band(s); fall back to a
  whole crop on any failure.
- [x] 2.3 Prove byte identity against a cache-disabled draw and a fresh crop at
  several pan and zoom steps.

## 3. Profile

- [x] 3.1 Extend `scroll_zoom_pan_profile` in
  `crates/pictura-app/src/cxxqt_object/tests_profiles/pan_zoom.rs` to time a
  level-0 viewport crop against the exposed pan band.

## 4. Verification

- [x] 4.1 Build and run `tst_present_cache` (20), the present self-test check 76,
  `tst_canvas_pan`, `tst_large_document`, `tst_damage_bounds`, `tst_zoom_tool`,
  and `cargo nextest run -p pictura_app` (573).
- [x] 4.2 `cargo fmt`, `cargo clippy`, `scripts/guard.sh`,
  `openspec validate --all --strict`, `check-test-plan.py`, and
  `check-file-size.sh` all pass. (`scripts/verify-fast.sh` stops at the
  pre-existing app self-test failure at check 283, which also fails on an
  untouched master build.)
- [x] 4.3 Headless pan-then-zoom measurement at 4000² and 16000²: the level-0
  zoom frame is ~2× cheaper (16000²: ~16 ms full crop → ~8.7 ms reused) and the
  Rust profile shows the full level-0 crop is 6.8 ms vs 1.1 ms for the exposed
  band.

## 5. Close out

- [x] 5.1 Archive the change (`openspec archive pan-zoom-crop-reuse`).
- [x] 5.2 Update `docs/dev/STATE.md` with the measured before/after.
