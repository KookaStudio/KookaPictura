# Tasks

## 1. Tests first

- [x] 1.1 `tst_transform_controls::lowerLayerStaysUnderTheLayersAbove` — a blue
  layer below a red layer scales over it; the red stays on top and the blue
  shows outside it.
- [x] 1.2 `tst_transform_controls::topmostLayerBuildsNoAboveImage` — a transform
  of the topmost layer prepares no above-layer image.

## 2. Above-layer image for transforms

- [x] 2.1 `preview::transform_preview_above` builds the layers above the
  transform's hidden set over transparency at a requested level.
- [x] 2.2 `compute_transform_preview` stores the above image, origin, and scale
  in the shared `move_above*` fields, over the presented rectangle when the
  canvas has one, else the whole canvas.
- [x] 2.3 A topmost target leaves the above image empty.

## 3. Canvas

- [x] 3.1 `ImageView::beginTransformPreview` takes the above-layer image and
  draws it after the transformed layer, clipped to the transformed footprint.
- [x] 3.2 `ToolController::beginTransformImpl` passes the above image, origin,
  and scale from the shared bridge.

## 4. Finish

- [x] 4.1 Run `tst_transform_controls`, `tst_transform_warp`,
  `tst_transform_distort`, `tst_move_feedback`.
- [x] 4.2 Run `scripts/verify-fast.sh`.
- [x] 4.3 Archive the change.
