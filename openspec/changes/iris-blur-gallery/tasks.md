# Tasks: iris-blur-gallery

## 1. Engine kernel

- [x] 1.1 Factor `variable_blur(radius_at)` out of `field_blur` without a
  byte change. Verify: the `field blur` kernel digest is unchanged.
- [x] 1.2 Add `IrisPin`, `iris_exponent`, `MAX_IRIS_RADIUS`, and
  `blur/iris.rs::iris_blur`. Verify: core, fade, falloff, two-island,
  effects-gating, no-op, validation, alpha, tiny-image, determinism tests.
- [x] 1.3 Add `Filter::IrisBlur`, dispatch it, add it to
  `filter_preserves_opacity`, `preview_apron`, and the alpha pass's dropped
  effects; add the kernel digest and no-equivalent oracle rows.

## 2. App

- [x] 2.1 Move the gallery slot parsing to `filter_map/gallery.rs`, add
  `iris-blur` (`IRIS_BLUR_SLOTS = 22`) and its arity, and the whole-layer
  preview row. Verify: `filter_map_tests`, `impl_filters` tests.
- [x] 2.2 Add the Iris mode to `BlurGalleryCanvas` (outline, radius handles,
  feather dots, roundness diamond) and `BlurGalleryDialog` (active row,
  8-slot records, default outline, preview scaling).
- [x] 2.3 Enable `Filter ▸ Blur Gallery ▸ Iris Blur…`, add the `iris-blur`
  command spec (sharing the effects tail helper with Field Blur), and route
  it to the workspace.
- [x] 2.4 Qt Test: defaults/rows, slot round-trip, handle drags, preview core
  stays sharp, menu commit; `tst_filter_menu` lists Iris Blur live with 22
  slots. Verify: `ctest --test-dir build -R '^tst_' --output-on-failure`.

## 3. Documentation

- [x] 3.1 Update `docs/06-filters/blur-gallery.md` and `docs/dev/STATE.md`.

## 4. Integration verification

- [x] 4.1 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` and the Qt suite.
