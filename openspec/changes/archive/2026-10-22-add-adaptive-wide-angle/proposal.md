# Proposal: add-adaptive-wide-angle

## Why

Issue #346 (part of the #167 remaining-filter audit). `Filter ▸ Adaptive Wide
Angle…` is a disabled stub with no kernel and no photorust source. The docs
contract (`docs/06-filters/adaptive-wide-angle.md`, `FILT-094`) describes a
full-canvas workspace that corrects wide-angle and panorama curvature from a
lens model plus user-drawn line and polygon constraints.

## What Changes

- `pictura-filters` adds `distort::adaptive_wide_angle`: a lens model
  (Fisheye = equidistant, Perspective = rectilinear, Full Spherical =
  equirectangular; focal length in pixels from Focal Length × Crop Factor
  over the 43.27 mm full-frame diagonal), a stereographic base projection
  (unit magnification at the centre, fisheye clipped at 90°), and a
  constraint solver after Carroll, Agrawala and Agarwala's content-preserving
  projections (SIGGRAPH 2009): a coarse mesh solved by least squares with a
  similarity (conformal) energy, great-circle line constraints that must come
  out straight, Horizontal / Vertical / fixed-angle orientations, and polygon
  constraints whose inside follows a perspective (gnomonic) projection. The
  result renders through a fine triangle mesh. `Filter::AdaptiveWideAngle`
  dispatches to it; blank areas are transparent, or white on a layer with no
  transparency.
- `pictura-app` adds a `filter_tools/adaptive_wide_angle.rs` bridge (one
  dialog session; a store of kept constraint sets) and the `adaptive-wide-angle`
  kind: Correction, Scale, Focal Length, Crop Factor, the Move tool's offset,
  and the kept constraint set's id. OK commits through the ordinary filter
  path (one history state, Last Filter, Last Filter Settings).
- The C++ `AdaptiveWideAngleDialog` / `AwaCanvas` give CS6's workspace:
  Constraint (C), Polygon Constraint (Y), Move (M), Hand (H) and Zoom (Z);
  Correction, Scale, Focal Length, Crop Factor and As Shot; the Detail loupe;
  Preview, Show Constraints and Show Mesh; the camera and lens status lines.
- The menu row is enabled for 8-bit RGB and Grayscale documents only.
- Deferred (follow-up issues): Auto and the lens-profile database, Panorama
  (needs Photomerge metadata), Smart Filter re-edit, constraint rotation
  handles, and Save/Load Constraints.

## Capabilities

### New Capabilities

- `imaging/adaptive-wide-angle`: the projection model, the constraint solver
  and its guarantees, the commit, and the workspace.

## Impact

- `pictura-filters` (`distort/adaptive_wide_angle/*`, `Filter`, variants,
  props, apply, exports, test fixtures), `pictura-render` (moving-kernel
  fixture), `pictura-app` (`filter_tools/adaptive_wide_angle.rs`,
  `filter_map` descriptor and mapping, `build.rs`, `frame_menus_filter.cpp`,
  `filter_commands.h`, `awa_dialog.*`, `awa_canvas.*`, `CMakeLists.txt`),
  `tst_adaptive_wide_angle`.
