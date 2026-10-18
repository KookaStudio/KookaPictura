# Proposal: spin-blur-gallery

## Why

`Filter ▸ Blur Gallery ▸ Spin Blur…` is a disabled stub (issue #464). Spin
Blur is a **post-CS6 (CC 2014)** Blur Gallery effect: a rotational motion
blur inside one or more ellipses, measured as a Blur Angle.
`docs/06-filters/blur-gallery.md` records it as a CC non-goal; the owner has
moved it into scope (as Path Blur and the Noise tab were) and supplied CC
screenshots of the workspace, its controls, and the Motion Effects tab. It
is labelled non-parity: Adobe's algorithm is closed and there is no oracle.

## What Changes

- `pictura-filters` gains `blur/spin.rs`: `SpinPin { x, y, radius_x,
  radius_y, angle, feather, blur_angle, pivot_x, pivot_y }`, `SpinStrobe {
  strength, flashes, duration }`, `MAX_SPIN_ANGLE`, and `Filter::SpinBlur {
  spins, strobe, noise }`. Each pixel inside a spin's ellipse averages
  bilinear taps along the circular arc around the spin's rotation centre,
  the arc's angle the Blur Angle times the ellipse's feathered mask; strobe
  flashes of a set angular duration mix in. The Noise pass follows.
- `Filter::BlurGallery` gains `spin` and `spin_strobe`: the spin pass runs
  after the path pass; Noise takes the largest of the radius, path, and
  spin weights. With no spins the combined kernel is byte-identical to
  today's.
- `pictura-app` maps `spin-blur` (26 canonical) and extends `blur-gallery`
  with the spin section and a Strobe Flash Duration slot (62 canonical).
  `Filter ▸ Blur Gallery ▸ Spin Blur…` opens `BlurGalleryDialog` in a new
  Spin mode: the Spin Blur row (Blur Angle), a third Motion Effects slider
  (Strobe Flash Duration), and on-canvas spins (ellipse, radius handles,
  feather dots, an angle ring, Alt-drag to move the rotation centre).
- Verification: spin engine units, kernel digests, a no-equivalent oracle
  row, mapping tests, the existing `tst_blur_gallery_*` / `tst_tilt_shift` /
  `tst_path_blur` / `tst_filter_menu` suites updated for the live row, and a
  new `tst_spin_blur` Qt suite.
- Documentation: `docs/06-filters/blur-gallery.md`,
  `docs/11-cross-cutting/open-questions.md`, `docs/dev/STATE.md`,
  `docs/dev/doc-code-parity-gaps.md`.

## Capabilities

**Modified:** `imaging/blur-gallery` (ADDED: the Spin Blur workspace mode,
its slots, and the spin section of the combined slots), `imaging/blur-filters`
(ADDED: the spin blur kernel and its place in the combined kernel).

## Impact

- `crates/pictura-filters`: `blur/spin.rs` (+ `blur/spin/tests.rs`),
  `blur/gallery.rs` (+ tests), `blur.rs`, `filter/types.rs`,
  `filter/apply.rs`, `lib.rs`, `tests/kernel_digests.rs`,
  `tests/oracle/mapping.rs`.
- `crates/pictura-render`: `filter.rs` (preview apron) and its tests.
- `crates/pictura-app`: `filter_map.rs`, `filter_map/gallery.rs`,
  `filter_map/arities.rs`, `filter_map_tests.rs`, `filter_tools.rs`,
  `impl_filters.rs`, `cpp/blur_gallery_{canvas,dialog}.{h,cpp}`,
  `cpp/blur_gallery_path.cpp`, new `cpp/blur_gallery_spin.cpp`,
  `cpp/filter_commands.cpp`, `cpp/command_tree.cpp`,
  `cpp/frame_menus_filter.cpp`, `CMakeLists.txt`, the Qt suites.
- No new dependency; CPU only.
- Ceilings: the mask, the arc gather (at most 128 taps), the strobe model,
  the shared strobe values, the ranges, and the linear sliders (CC's Blur
  Angle and Flash Duration sliders are non-linear) are inferred.
