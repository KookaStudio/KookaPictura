# Proposal: path-blur-gallery

## Why

`Filter ▸ Blur Gallery ▸ Path Blur…` is a disabled stub (issue #462). Path
Blur is a **post-CS6 (CC 2014)** Blur Gallery effect: motion blur that
follows one or more drawn paths. `docs/06-filters/blur-gallery.md` records
it as a CC non-goal; the owner has moved it into scope (as the Noise tab
was), and supplied CC screenshots of the workspace, its opening state, and
its controls. It is labelled non-parity: Adobe's algorithm is closed and
there is no oracle.

## What Changes

- `pictura-filters` gains `blur/path.rs`: `BlurPath { points, start_speed,
  end_speed }`, `PathMotion { speed, taper, centered, rear_sync,
  strobe_strength, strobe_flashes }`, `MAX_PATH_POINTS`, `MAX_PATH_SPEED`,
  `MAX_END_POINT_SPEED`, and `Filter::PathBlur { paths, motion, noise }`.
  Each path is a Catmull-Rom curve through its points; every pixel takes a
  motion vector blended from the paths by inverse-square distance (the
  nearest point's tangent times the speed interpolated between the path's
  two end-point speeds), and averages bilinear taps along the streamline of
  that field (centred, or trailing behind the pixel), with taper, strobe,
  and Rear Sync Flash weights. The Noise pass follows.
- `Filter::BlurGallery` gains `path` and `motion`: the radius-field effects
  blur first (pyramid, Tilt-Shift smear), then the path pass, then the
  effects. Bokeh stays gated by the radius-field weights; Noise takes the
  larger of those and the path weights. With no paths the combined kernel
  is byte-identical to today's.
- `pictura-app` maps `path-blur` (variable path records, the 6-slot motion
  section, the 13-slot tail; 27 canonical) and extends `blur-gallery` with
  the path section (51 canonical). `Filter ▸ Blur Gallery ▸ Path Blur…`
  opens `BlurGalleryDialog` in a new Path mode: the Path Blur row (Basic
  Blur / Rear Sync Flash, Speed, Taper, Centered Blur, End Point Speed, a
  disabled Edit Blur Shapes), a live Motion Effects tab (Strobe Strength,
  Strobe Flashes), and on-canvas paths with points and an end arrow.
- Verification: path engine units, kernel digests, a no-equivalent oracle
  row, mapping tests, `tst_filter_menu`, `tst_blur_gallery_*`, and a new
  `tst_path_blur` Qt suite.
- Documentation: `docs/06-filters/blur-gallery.md`,
  `docs/11-cross-cutting/open-questions.md`, `docs/dev/STATE.md`.

## Capabilities

**Modified:** `imaging/blur-gallery` (ADDED: the Path Blur workspace mode,
its slots, and the path section of the combined slots), `imaging/blur-filters`
(ADDED: the path blur kernel and its place in the combined kernel).

## Impact

- `crates/pictura-filters`: `blur/path.rs` (+ `blur/path/tests.rs`),
  `blur/gallery.rs` (+ tests), `blur.rs`, `filter/types.rs`,
  `filter/apply.rs`, `lib.rs`, `tests/kernel_digests.rs`,
  `tests/oracle/mapping.rs`.
- `crates/pictura-render`: `filter.rs` (preview apron) and its tests.
- `crates/pictura-app`: `filter_map.rs`, `filter_map/gallery.rs`,
  `filter_map/arities.rs`, `filter_map_tests.rs`, `filter_tools.rs`,
  `impl_filters.rs`, `cpp/blur_gallery_{canvas,dialog}.{h,cpp}`, new
  `cpp/blur_gallery_path.cpp` and `cpp/blur_gallery_canvas_path.cpp`,
  `cpp/filter_commands.cpp`, `cpp/command_tree.cpp`,
  `cpp/frame_menus_filter.cpp`, `CMakeLists.txt`, the Qt suites.
- No new dependency; CPU only.
- Ceilings: the motion field, the streamline gather (at most 64 taps), the
  taper / strobe / Rear Sync Flash weights, the Speed and End Point Speed
  ranges, and the path-drawing gestures are inferred; Edit Blur Shapes is
  not built (each end's blur shape follows the path tangent); Spin Blur
  stays a stub.
