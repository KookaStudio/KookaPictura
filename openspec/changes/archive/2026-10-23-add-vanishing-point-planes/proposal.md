# Proposal: add-vanishing-point-planes

## Why

Issue #349 (part of the #167 remaining-filter audit). `Filter ▸ Vanishing
Point…` (Alt+Ctrl+V) is a disabled stub with no kernel and no photorust
source. The docs contract (`docs/06-filters/vanishing-point.md`, `FILT-102`)
describes a workspace of perspective planes that clone, paint, paste and
transform in perspective. This change ships the first half: the plane model,
the workspace, and Render Grids. The retouching tools follow in a second
change.

## What Changes

- `pictura-core` gains `homography::Homography`, the four-point projective
  solve that `pictura-render`'s perspective transforms and the app's
  transform preview each carried a private copy of. Both now use it.
- `pictura-filters` adds `distort::vanishing_point`:
  - A root plane is four image corners. They give two vanishing points, and
    with the principal point at the document centre, a focal length, or none
    when no camera sees the quad as a rectangle. The plane then lifts to a 3D
    rectangle, which fixes its metric and its grid.
  - Validity follows CS6's colours. Red means crossed, concave or
    degenerate. Yellow means no camera fits. Blue means valid.
  - Related planes hinge on a parent edge at an angle (90° when torn off).
    They follow their parent, keep the family's grid continuous across the
    hinge, and lock a parent's angle once it has children.
  - `VpEditor` adds the active plane, the drag gestures (corner, edge, move,
    tear-off, Alt-turn), Ctrl-click cycling, delete, and the dialog's own
    undo and redo.
  - `Filter::VanishingPoint` draws a kept edit's lines (Render Grids) black
    and antialiased.
- `pictura-app` adds the `filter_tools/vanishing_point.rs` bridge (one dialog
  session; a store of kept edits) and the `vanishing-point` kind, whose one
  slot names a kept edit by id. OK commits through the ordinary filter path:
  one history state, the selection clip, Fade, Last Filter, and Last Filter
  Settings. The planes stay on the document session, so the dialog reopens on
  them.
- The C++ `VanishingPointDialog` and `VpCanvas` provide:
  - the menu: Show Edges (Ctrl+H) and Render Grids To Photoshop
  - Grid Size and Angle, a help line, and OK / Cancel
  - Edit Plane (V), Create Plane (C), Hand (H) and Zoom (Z, or hold X)
  - the preview, with a zoom row.
- The 8-bit RGB / Grayscale gate Adaptive Wide Angle used is shared, and
  Vanishing Point takes it too.
- Deferred to follow-up issues:
  - Marquee, Stamp, Brush, Transform, Eyedropper and Paste (the second
    change on #349).
  - Saving plane data in PSD, TIFF and JPEG.
  - The Extended-only items stay out of scope: Measure, Render Measurements,
    and DXF/3DS export.

## Capabilities

### New Capabilities

- `imaging/vanishing-point`: the plane model and its validity, related
  planes, the editing gestures, Render Grids, the commit, and the workspace.

## Impact

- `pictura-core`: `homography.rs`.
- `pictura-render`: `layer_ops/transform.rs` uses the core solver; the
  moving-kernel fixture.
- `pictura-filters`:
  - `distort/vanishing_point/*`
  - `Filter`, the variant row, props, apply, and exports
  - the test fixtures.
- `pictura-app`:
  - `filter_tools/vanishing_point.rs` and the shared gate in `filter_tools.rs`
  - `document_session/session.rs`
  - the `filter_map` descriptor and mapping, and `build.rs`
  - `impl_transform/geometry.rs`
  - `frame_menus_filter.cpp` and `filter_commands.h`
  - `vp_dialog.*` and `vp_canvas.*`
  - `CMakeLists.txt`
- `tst_vanishing_point`.
- The `tool.createplane` icon.
