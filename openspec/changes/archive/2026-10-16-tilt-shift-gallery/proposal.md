# Proposal: tilt-shift-gallery

## Why

`Filter ▸ Blur Gallery ▸ Tilt-Shift…` is a disabled stub (issue #342).
CS6's Tilt-Shift (`docs/06-filters/blur-gallery.md`, `FILT-092`) fakes a
miniature photo: a sharp band, a fade, and blur beyond it, with a Distortion
slider and Symmetric Distortion. Field Blur (#340) and Iris Blur (#341)
landed the workspace, the variable-blur pyramid, and the effects passes;
Tilt-Shift reuses them. The owner also asked that every blur refuse an empty
target with an error, as Photoshop does ("Could not complete the Tilt-Shift
command because the initial bounding rectangle is empty.").

## What Changes

- `pictura-filters` gains `TiltPin { x, y, angle, focus_above, focus_below,
  fade_above, fade_below, distortion, symmetric, blur }`, `MAX_TILT_EXTENT`,
  and `Filter::TiltShift { pins, bokeh, noise }` in `blur/tilt.rs`. Each pin
  is a band through its centre: 0 between the focus lines, a smoothstep
  across the fade, 1 beyond it; pins multiply falloffs and blend amounts by
  inverse-square distance (as Iris). Distortion smears the blurred zone
  below the band (both zones when symmetric) radially (positive) or
  tangentially (negative) around the pin. `field.rs` splits `variable_blur`
  into `pyramid_blur` + `apply_effects` (no byte change) so the smear runs
  between them.
- `pictura-app` maps `tilt-shift` (`count`, 10 slots per pin, the 13-slot
  tail; 24 canonical) and opens `BlurGalleryDialog` in Tilt-Shift mode: the
  Tilt-Shift row with Blur, Distortion, and Symmetric Distortion; the
  selected pin draws its solid focus lines, dashed fade lines, and two
  rotation dots. The canvas's `iris` flag becomes `BlurGalleryMode`.
- The Blur Tools rows become selectable, checkable, and expandable, and the
  effects switched on combine: `Filter::BlurGallery { field, iris, tilt,
  bokeh, noise }` blurs each pixel by the largest of the per-effect radius
  fields, and OK commits the single effect's own kind or the new
  `blur-gallery` kind for several (owner's request).
- Every `Filter ▸ Blur` and `Filter ▸ Blur Gallery` command (menu, Last
  Filter, Last Filter Settings) refuses an active layer with no opaque pixel
  inside the selection (or at all) with a critical message box and no
  history state. A new `filter_target_empty` bridge answers the question.
- Verification: tilt engine units, a kernel digest, a no-equivalent oracle
  row, mapping and empty-target Rust tests, the whole-layer preview row,
  `tst_filter_menu`, and a new `tst_tilt_shift` Qt suite.
- Documentation: `docs/06-filters/blur-gallery.md`,
  `docs/06-filters/blur-filters.md`, `docs/06-filters/filters-overview.md`,
  `docs/dev/STATE.md`.

## Capabilities

**New:** `imaging/empty-blur-target`.

**Modified:** `imaging/blur-gallery` (ADDED: the Tilt-Shift workspace mode
and its slots, combined effects and their slots), `imaging/blur-filters`
(ADDED: the tilt-shift and combined kernels).

## Impact

- `crates/pictura-filters`: `blur/tilt.rs` (+ `blur/tilt/tests.rs`),
  `blur/field.rs`, `blur.rs`, `filter/types.rs`, `filter/apply.rs`,
  `lib.rs`, `tests/kernel_digests.rs`, `tests/oracle/mapping.rs`.
- `crates/pictura-render`: `filter.rs` (preview apron).
- `crates/pictura-app`: `filter_map.rs`, `filter_map/gallery.rs`,
  `filter_map/arities.rs`, `filter_map_tests.rs`, `filter_tools.rs`,
  `impl_filters.rs`, `cpp/blur_gallery_{canvas,dialog}.{h,cpp}`,
  `cpp/filter_commands.cpp`, `cpp/command_tree.cpp`,
  `cpp/frame_menus_filter.cpp`, `cpp/tests/tst_tilt_shift.cpp`,
  `cpp/tests/tst_blur_gallery_dialog.cpp`, `cpp/tests/tst_filter_menu.cpp`,
  `cpp/tests/CMakeLists.txt`.
- No new dependency; CPU only.
- Ceilings: the band falloff, combine, and distortion smear are inferred;
  one gallery effect per workspace; no `M` mask view; the distortion is a
  line-sampled smear, not Adobe's shader.
