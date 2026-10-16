# Proposal: iris-blur-gallery

## Why

`Filter ▸ Blur Gallery ▸ Iris Blur…` is a disabled stub (issue #341). CS6's
Iris Blur (`docs/06-filters/blur-gallery.md`, `FILT-092`) simulates a
shallow depth of field: each pin carries an ellipse with a **sharp** core, a
**fade** band, and a **blurred** exterior, reshaped on canvas. Field Blur
(#340) landed the shared Blur Gallery workspace, the variable-blur pyramid,
and the Blur Effects / Noise passes; Iris Blur reuses all three.

## What Changes

- `pictura-filters` gains `IrisPin { x, y, radius_x, radius_y, angle,
  squareness, feather, blur }`, `iris_exponent`, `MAX_IRIS_RADIUS`, and
  `Filter::IrisBlur { pins, bokeh, noise }` in `blur/iris.rs`. Each pin is a
  rotated superellipse (squareness 0 = ellipse, 100 = near-square); the
  per-pin falloff is 0 inside the feather core, a smoothstep across the fade
  band, and 1 outside the outline. Pins multiply their falloffs (every pin
  keeps its own sharp island) and the exterior amount is the
  inverse-square-distance blend of the pin amounts. Field Blur's pyramid and
  effects passes move behind a shared `variable_blur(radius_at)` with no
  byte change (the Field Blur digest is unchanged).
- `pictura-app` maps the `iris-blur` kind (`count`, then 8 slots per pin,
  then the shared 13-slot effects tail; 22 canonical) in a new
  `filter_map/gallery.rs` that also takes over the `field-blur` mapping, and
  routes `Filter ▸ Blur Gallery ▸ Iris Blur…` to `BlurGalleryDialog` in Iris
  mode: the Iris Blur row active with its Blur amount, the other rows
  disabled, and the selected pin's outline drawn with its four radius
  handles (resize and rotate), four feather dots, and the roundness diamond.
  New pins copy the selected pin's outline and amount.
- Verification: engine unit tests (sharp core, fade ramp, rotation /
  squareness / feather falloff, two islands, effects gating, no-op,
  validation, alpha, determinism), a kernel digest row, a no-equivalent
  oracle row, bridge mapping tests, the whole-layer preview row, and Qt Test
  cases in `tst_blur_gallery_dialog` and `tst_filter_menu`.
- Documentation: `docs/06-filters/blur-gallery.md` and `docs/dev/STATE.md`.

## Capabilities

**New:** none — the Iris requirements join `imaging/blur-gallery`.

**Modified:** `imaging/blur-gallery` (ADDED: the Iris Blur workspace mode and
its slots), `imaging/blur-filters` (ADDED: the iris kernel).

## Impact

- `crates/pictura-filters`: `blur/iris.rs` (+ `blur/iris/tests.rs`),
  `blur/field.rs` (shared `variable_blur`), `blur.rs`, `filter/types.rs`,
  `filter/apply.rs`, `lib.rs`, `tests/kernel_digests.rs`,
  `tests/oracle/mapping.rs`.
- `crates/pictura-render`: `filter.rs` (opacity allowlist, apron, alpha pass).
- `crates/pictura-app`: `filter_map.rs`, `filter_map/gallery.rs`,
  `filter_map/arities.rs`, `filter_map_tests.rs`, `filter_tools.rs`,
  `impl_filters.rs`, `cpp/blur_gallery_{canvas,dialog}.{h,cpp}`,
  `cpp/filter_commands.cpp`, `cpp/command_tree.cpp`,
  `cpp/frame_menus_filter.cpp`, `cpp/tests/tst_blur_gallery_dialog.cpp`,
  `cpp/tests/tst_filter_menu.cpp`.
- No new dependency; CPU only.
- Ceilings: the falloff curve, squareness map, and multi-pin combine are
  inferred; one gallery effect per workspace (Field and Iris do not combine);
  Alt-drag of a single feather dot is not shipped; no `M` mask view.
