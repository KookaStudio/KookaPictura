# Design — Distort transform parity

## Context

The Free Transform session already holds a live target quad for `Skew`,
`Distort`, and `Perspective`. The Distort dispatch currently sets the grabbed
corner to the pointer's absolute document position, does not clamp the quad,
leaves the reference point fixed, and accepts editable type. The projective
commit already solves the homography; it does not distinguish the two sides of
the horizon.

The reference is photocraft's `transform_tool::apply_mode_drag` Distort branch,
its `keep_convex`, its `carry_pivot`, its guide-only Distort snapping, and
`photocraft_algo::transform`'s `front_side` sampling rule. The local photocraft
checkout at HEAD is the source.

## Gesture math

`impl_transform/distort.rs`:

- `delta = pointer − press`.
- Move the grabbed corner by `delta` from the press-time quad
  (`start_quad`), so a press within the 6 px handle tolerance keeps its grab
  offset instead of jumping.
- Feed the result through the shared `keep_convex` (moved from `skew.rs` to
  `convex.rs`): interpolate from the start quad to the target and keep the
  furthest convex result. `store_quad` still refuses degenerate results.
- Carry the reference point through the homography that sends `start_quad` to
  the accepted quad, using `projective_coefficients`. The pivot at press is
  captured in a new `start_pivot` field; without it, carrying the already-moved
  pivot each move would compound. If the homography is singular, leave the
  pivot alone.

Skew keeps its current helpers and behavior; only the helper location changes.

## Type refusal

A Distort session on an editable type layer is refused at begin time. The same
predicate drives the menu's enabled provider through a new
`layer_can_transform_mode` bridge function, so the command greys out instead of
failing silently. A selection that lifts pixels is exempt: the session then
targets the floating raster, not the type layer.

Perspective stays unchanged; #748 is scoped to Distort.

## Guide snapping

Photocraft snaps only Distort corners, and only to guides. This app has no
working `View > Snap` / `Snap To > Guides` state, so the change snaps whenever
document guides are visible:

- The C++ `ToolController` records the grabbed corner and press point.
- On each move it computes the proposed corner (`start corner + delta`), finds
  the nearest visible vertical and horizontal guides within `8 / zoom` document
  pixels, and shifts the pointer passed to Rust by the snap delta. Rust remains
  the single owner of the gesture math.
- The snapped guides are drawn through the existing smart-guide overlay and
  cleared on release/cancel. Lines the convex clamp prevented the corner from
  reaching are dropped after the gesture, as photocraft does.

This is a deliberate deviation until `View > Snap` exists (`ponytail:` comment
at the seam).

## Horizon / front side

`QuadMap` gains the front side of the projective map: the sign shared by the
majority of the source rect's four forward homogeneous divisors (the centre's
side on a tie). Its inverse returns `None` when the destination point's divisor
has the opposite sign, so `resample_plane` writes those pixels as `0` instead of
sampling a mirrored copy wrapped round infinity. The same map serves channels,
masks, and native planes.

The live preview is one `QImage` under a `QTransform`, so it clips the
destination half-plane in document space before drawing. A new pure helper in
`image_view.{h,cpp}` builds the front-side clip from the local projective
matrix, the image origin, the source size, and the document bounds. It is tested
directly; a convex Distort gesture cannot reach a horizon crossing because
`keep_convex` stops first, so the engine rule is the defensive path for direct
quad transforms.

## What stays the same

- Perspective's gesture and enabled state.
- Skew's gestures and editable-type parallelogram.
- The commit path, history label, interpolation menu, and self-test checks.
- Numeric setters and the options-bar readouts.
