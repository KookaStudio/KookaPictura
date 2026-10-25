# Proposal: add-lens-correction

## Why

Issue #347 (part of the #167 remaining-filter audit). `Filter ▸ Lens
Correction…` is a disabled stub with no kernel and no photorust source. The
docs contract (`docs/06-filters/lens-correction.md`, `FILT-101`) describes
CS6's dialog: profile-driven Auto Correction plus manual geometric
distortion, chromatic aberration, vignette, and perspective/angle/scale
transform, with an alignment grid.

## What Changes

- `pictura-filters` adds `distort::lens_correction`: one inverse map per
  output pixel through Scale, an angle-and-perspective homography (a virtual
  camera tilt about the centre, re-centred, then rotated), Remove Distortion
  (Brown–Conrady `k1` on the radius over the half-diagonal), Auto Scale, and a
  profile's `k1 k2 k3`; per-channel radial scale for the three fringe
  sliders and a profile's chromatic aberration; a radial vignette gain over
  the profile's divided-out falloff; Edge Extension / Transparency / Black /
  White fill blended by coverage; premultiplied Catmull-Rom sampling.
  Lens profiles use a small in-tree text format; the bundled set holds one
  synthetic profile so the Auto path runs end to end. `Filter::LensCorrection`
  dispatches to it.
- `pictura-app` adds a `filter_tools/lens_correction.rs` bridge (one dialog
  session, EXIF, profile search, Previous Conversion) and the
  `lens-correction` kind with 16 slots. OK commits through the ordinary filter
  path (one history state, Last Filter, Last Filter Settings).
- The depth gate grows a second rule: Lens Correction takes 8- and 16-bit RGB
  and Grayscale (Adaptive Wide Angle and Vanishing Point stay 8-bit only).
- The C++ `LensCorrectionDialog` gives CS6's dialog: Remove Distortion (D),
  Straighten (A), Move Grid (M), Hand (H), Zoom (Z); the help line; the
  Auto Correction and Custom tabs; Preview, Show Grid, grid Color and Size;
  the Camera Model / Lens Model / Camera Settings lines. `Shift+Ctrl+R` opens
  it.
- Deferred (follow-up issues): a real profile set with sub-profiles, Prefer
  RAW and Search Online; Save / Load Settings, Set Lens Default and Alt-drag
  fringe isolation; Lens Correction as a Smart Filter.

## Capabilities

### New Capabilities

- `imaging/lens-correction`: the correction model, edge fill, profiles, the
  depth gate, the dialog, and the commit.

## Impact

- `pictura-filters` (`distort/lens_correction/*`, `liquify::catmull_rom`
  visibility, `Filter`, variants, props, apply, exports, test fixtures),
  `pictura-render` (moving-kernel fixture), `pictura-app`
  (`filter_tools/lens_correction.rs`, `depth_gate.rs`, `filter_map`
  descriptor and mapping, `build.rs`, `frame_menus_filter.cpp`,
  `filter_commands.h`, `lens_correction_*`, `CMakeLists.txt`),
  `pictura-script` (catalogue row and golden), `tst_lens_correction`,
  `tst_filter_menu`.
