# Tasks: tilt-shift-gallery

## 1. Engine kernel

- [x] 1.1 Split `variable_blur` into `pyramid_blur` + `apply_effects` with no
  byte change. Verify: the field and iris kernel digests are unchanged.
- [x] 1.2 Add `TiltPin`, `MAX_TILT_EXTENT`, and `blur/tilt.rs::tilt_shift`
  with the distortion smear. Verify: focus, fade, per-side extents,
  rotation, two-band, distortion, effects gating, no-op, validation, alpha,
  tiny-image, determinism tests.
- [x] 1.3 Add `Filter::TiltShift`, dispatch it, add it to `filters_alpha`
  and `preview_apron`; add the kernel digest and no-equivalent oracle rows.

## 2. App

- [x] 2.1 Map `tilt-shift` (`TILT_SHIFT_SLOTS = 24`), its arity, and the
  whole-layer preview row. Verify: `filter_map_tests`, `impl_filters` tests.
- [x] 2.2 Add `BlurGalleryMode` and the Tilt-Shift lines, dots, and drags to
  `BlurGalleryCanvas`; the Tilt-Shift row (Blur, Distortion, Symmetric
  Distortion) and 10-slot record to `BlurGalleryDialog`.
- [x] 2.3 Enable `Filter ▸ Blur Gallery ▸ Tilt-Shift…` and route it to the
  workspace.
- [x] 2.4 Add `filter_target_empty` and refuse empty Blur / Blur Gallery
  targets on the menu, Last Filter, and Last Filter Settings paths. Verify:
  the `selected_area_empty` unit test.
- [x] 2.5 Qt Test `tst_tilt_shift`: defaults/rows, slot round-trip, line and
  dot drags, distortion controls, preview band stays sharp, menu commit,
  every blur refuses an empty layer, a selection over transparency is
  empty. `tst_filter_menu` lists Tilt-Shift live with 24 slots. Verify:
  `ctest --test-dir build -R '^tst_' --output-on-failure`.

- [x] 2.6 Hover cursors on the lines (resize along the normal) and dots
  (rotate); a ≤ 360 px draft preview while dragging a pin, line, dot, or the
  Blur/Distortion slider. Verify: `hoverShowsTheDragCursors`,
  `aDragPreviewsADraftThenTheFullPicture`.

- [x] 2.7 Preview only the active layer over the rest of the document
  (`filter_proxy_base` + `filter_proxy_layer`), and blur the alpha plane in
  the gallery pyramid and smear; update the three gallery digests. Verify:
  `onlyTheActiveLayerBlurs`, the field/iris/tilt alpha unit tests.

- [x] 2.8 Combined effects: `Filter::BlurGallery` (max of the per-effect
  fields), the `blur-gallery` slots and command spec, per-effect pin lists
  in the dialog with selectable, checkable, expandable rows. Verify: the
  gallery kernel units, `blur_gallery_combines_each_effects_records`, the
  digest and oracle rows, `tst_blur_gallery_effects`.

## 3. Documentation

- [x] 3.1 Update `docs/06-filters/blur-gallery.md`,
  `docs/06-filters/blur-filters.md`, `docs/06-filters/filters-overview.md`,
  and `docs/dev/STATE.md`.

## 4. Integration verification

- [x] 4.1 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` and the Qt suite.
