# Tasks: spin-blur-gallery

## 1. Engine kernel

- [x] 1.1 Add `blur/spin.rs`: `SpinPin`, `SpinStrobe`, `MAX_SPIN_ANGLE`, the
  mask, and the arc gather. Verify: ellipse, circles/spokes, pivot, strobe,
  validation, alpha, no-op, determinism units.
- [x] 1.2 Add `Filter::SpinBlur`, dispatch it, add it to `filters_alpha` and
  `preview_apron`; kernel digest and no-equivalent oracle rows.
- [x] 1.3 Add `spin` and `spin_strobe` to `Filter::BlurGallery`. Verify: the
  existing gallery digests are unchanged; spins alone match `SpinBlur`.

## 2. App

- [x] 2.1 Map `spin-blur` (26) and the extended `blur-gallery` (62), their
  arities, and the whole-layer preview row. Verify: `filter_map_tests`.
- [x] 2.2 `BlurGalleryMode::Spin`: canvas ellipse, handles, marker at the
  pivot, angle ring, Alt-drag; the Spin Blur row, the Flash Duration
  slider, and the spin slots (`blur_gallery_spin.cpp`).
- [x] 2.3 Enable `Filter ▸ Blur Gallery ▸ Spin Blur…` and route it to the
  workspace.
- [x] 2.4 Qt Test `tst_spin_blur`; update `tst_filter_menu`,
  `tst_blur_gallery_dialog`, `tst_blur_gallery_effects`, `tst_tilt_shift`,
  `tst_path_blur` for the live Spin Blur row and the 62-slot list.

## 3. Documentation

- [x] 3.1 Update `docs/06-filters/blur-gallery.md`,
  `docs/11-cross-cutting/open-questions.md`, `docs/dev/STATE.md`, and
  `docs/dev/doc-code-parity-gaps.md`.

## 4. Integration verification

- [x] 4.1 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` and the Qt suite.
