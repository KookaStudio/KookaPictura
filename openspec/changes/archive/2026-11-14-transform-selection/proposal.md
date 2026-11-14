# Proposal

## Why

`Select > Transform Selection` is a disabled menu row (#526, parity register
`TOOL-2`): a user can reshape a selection only by moving it, and cannot scale,
rotate, skew, distort, or warp the selection border without moving pixels. CS6
treats the border as a first-class transform target — the shared transform box
resamples the coverage mask and leaves the image untouched
(`docs/08-selection/transform-selection.md`, `SEL-011`).

The engine already has the session, gestures, overlay, options bar, exact
transform commands, and a tiled resampler. What is missing is a **selection
target** for that machine, a mask commit, a live border preview, and the Warp
branch for a colourless mask.

## What Changes

- Enable `Select > Transform Selection` for a document with a non-empty
  selection, and start the existing transform box over the selection bounds.
- Add a `Selection` target to the transform session: the same
  Free/Scale/Rotate/Skew/Distort/Perspective gestures, reference point,
  numeric X/Y/W/H/angle/skew fields, interpolation menu, and exact
  rotate/flip commands operate on the border.
- Commit by resampling the coverage mask once through the session map and
  replacing the selection, recording one `"Transform Selection"` history
  state; cancel and identity commits record nothing and leave the mask
  byte-identical.
- Show the marching ants on the transformed border while the session is live,
  and re-contour the committed mask at the 50 % coverage line.
- Support Warp on the selection border: the existing mesh UI and presets
  drive a warped coverage plane; the box hugs the warped bounds when Warp
  ends; the mask is warped plus framed on commit.
- `Edit > Transform > Again` repeats the last committed selection-border
  transform on the current selection.
- Keep the command distinct from `Edit > Free Transform` (`Ctrl+T`), which
  continues to transform the selected **pixels**.
- Lock the Select menu while a border transform is pending, as the modal
  layer transform locks Layer/Image/Filter.

Out of scope: the right-click canvas context-menu entry (community-reported,
unconfirmed against CS6) and the Move tool's `Show Transform Controls` path
for a selection border.

## Capabilities

### New Capabilities

- `tools/transform-selection`: the selection-border transform session, mask
  resampling commit, live border preview, Warp, and history/undo contract.

### Modified Capabilities

- `tools/select-menu`: `Transform Selection` becomes an implemented, enabled
  command instead of a documented deferred row.
- `ui/transform-options-bar`: the page is keyed by any transform session, not
  only Free Transform, and seeds its fields from the selection bounds for a
  border transform.
- `tools/transform-commands`: `Transform Again` repeats the last border
  transform when the selection was the last target, and `Edit > Transform >
  Warp` enters Warp on an open session instead of replacing it.

## Impact

- `pictura-render`: new `transform_selection` and `warp_selection` ops in
  `document_ops/layer_ops`, reusing the tiled resampler, homography solver,
  and warp rasterizer.
- `pictura-app` Rust: a transform target discriminant, a selection-transform
  begin/commit/cancel path, a preview-contour bridge, and Warp preview
  guards.
- `pictura-app` C++: Select menu handler and enablement, transform lock,
  overlay-only canvas start, ant preview, Warp/Again target handling.
- `pictura-script`: catalogue `implemented` flag, regenerated command ids and
  golden tables.
- `docs/dev/doc-code-parity-gaps.md` (`TOOL-2`) and `ROADMAP.md`.
- No new dependencies. The coverage model and history already store the
  selection mask.
