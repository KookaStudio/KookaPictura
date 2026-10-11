# Design

## Context

The tool lives in `tool_perspectivecrop.cpp` (currently drag-draws a rectangle
in `Gesture::New`, no snapping, no rotate, crosshair/move cursor only), its
overlay in `image_view_overlays.cpp` (`perspectiveQuad_.size() == 4`, shield over
`viewRect()`), and its commit is `perspective_crop_commit` in `crop_group.rs`
over `pictura_render::perspective_crop` (output size from
`perspective_crop_size`). The tool has no options-bar page (it falls through
`OptionsBar::buildPageBody` to the generic icon-only page). The Crop options page
(`options_bar_crop.cpp`) already provides the widgets to reuse.

## Goals / Non-Goals

**Goals:** four-click staging, rotate-outside, corner snapping, an options bar
with a manual output size, an inch grid, a canvas-only shield shown when
complete.

**Non-Goals:** a draggable rotate pivot (later, with the crop pivot); changing
the homography, refusals, or the layer-warp behaviour.

## Decisions

- **Staging state machine.** `Quad` accumulation holds 1–3 points; the fourth
  closes it. `setPerspectiveCropQuad` already takes a `QPolygonF`, so partial
  points are stored the same way; the overlay draws points for size 1–3 and the
  full chrome for 4. No new geometry type.
- **Rotate reuse.** The rotate band reuses `crop_grip.h`'s
  `cropRotateZone`/`cropRotateZoneAngle`; rotating maps the quad through a
  rotation about its centroid (the quad is re-stored, not the content).
- **Snapping reuse.** A shared snap helper snaps a point to the canvas
  corners/edges and the composite bounding box within 6 screen px, matching the
  crop's threshold; corners use it on placement and drag.
- **Output size seam.** Add `perspective_crop_sized(doc, quad, w, h)` in
  `pictura-render` (0 or negative = auto, delegating to `perspective_crop_size`)
  and a bridge `perspective_crop_commit_sized(view, quad, w, h)`; `W`/`H` blank
  passes 0. `perspective_crop` becomes a thin wrapper passing 0.
- **Options page.** Build `buildPerspectiveCropPage` in `options_bar_crop.cpp`
  reusing `NumericField`, the swap `QToolButton`, `document_ppi`, and the
  Clear/menu helpers; `showCropGroup` in the options bar maps
  `ToolId::PerspectiveCrop` to it. Cancel/Apply visibility is driven by the
  tool's "quad complete" state via the existing `toolSessionChanged` signal.
- **Grid inches.** Spacing = `round(ppi)` document px (resolution, defaulting to
  72 as `document_ppi` does); lines every `spacing` px along the quad edges,
  interpolated and clipped to the quad.
- **Shield scope.** Replace `shield.addRect(viewRect())` with the canvas image
  rect used by the rectangular crop, and gate the whole perspective overlay on
  `size() == 4`.

## Risks / Trade-offs

- [Spec scenario "Enter commits … drags a box" is rewritten] → the self-test
  switches to four clicks; update `selftest_crop_group.cpp`.
- [Manual W/H changes the output aspect] → the free-form homography already
  maps the quad onto the target rect, so a non-matching aspect is a legitimate
  stretch, as in CS6.
- [Rotating the quad can make it near-degenerate] → the existing degenerate
  refusal keeps the quad staged.
- [A new invokable needs a bridge declaration] → add it to `crop_group.rs`'s
  `extern "Rust"` block, not just the impl.

## Migration Plan

No migration; single app, no persisted format.

## Open Questions

None.
