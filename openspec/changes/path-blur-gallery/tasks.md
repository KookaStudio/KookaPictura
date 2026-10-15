# Tasks: path-blur-gallery

## 1. Engine kernel

- [x] 1.1 Add `blur/path.rs`: `BlurPath`, `PathMotion`, the limits, the
  motion field, and the streamline gather. Verify: row/stripe, centred and
  trailing, strobe, taper, rear sync, vertical, no-op, validation, alpha,
  determinism units.
- [x] 1.2 Add `Filter::PathBlur`, dispatch it, add it to `filters_alpha` and
  `preview_apron`; kernel digest and no-equivalent oracle rows.
- [x] 1.3 Add `path` and `motion` to `Filter::BlurGallery`. Verify: the
  existing gallery digest is unchanged; paths alone match `PathBlur`.

## 2. App

- [x] 2.1 Map `path-blur` (27) and the extended `blur-gallery` (51), their
  arities, and the whole-layer preview row. Verify: `filter_map_tests`.
- [x] 2.2 `BlurGalleryMode::Path`: canvas paths, points, arrow, gestures
  (`blur_gallery_canvas_path.cpp`); the Path Blur row, Motion Effects tab,
  and path slots (`blur_gallery_path.cpp`).
- [x] 2.3 Enable `Filter ▸ Blur Gallery ▸ Path Blur…` and route it to the
  workspace.
- [x] 2.4 Qt Test `tst_path_blur`; update `tst_filter_menu`,
  `tst_blur_gallery_dialog`, `tst_blur_gallery_effects`, `tst_tilt_shift`
  for the live Path Blur row. Verify:
  `ctest --test-dir build -R '^tst_' --output-on-failure`.

## 3. Documentation

- [x] 3.1 Update `docs/06-filters/blur-gallery.md`,
  `docs/11-cross-cutting/open-questions.md`, and `docs/dev/STATE.md`
  (`blur-filters.md` and `filters-overview.md` state only CS6 facts and
  stay as they are).

## 4. Integration verification

- [x] 4.1 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` and the Qt suite.
