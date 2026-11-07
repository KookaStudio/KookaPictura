# Tasks

## 1. Tests first

- [x] 1.1 `tst_move_feedback::movedLayerStaysUnderTheLayersAbove` — a blue layer
  below a red layer drags under it; the red stays on top, the blue shows outside
  it, and the old place is the backdrop.
- [x] 1.2 `tst_move_feedback::antsRepaintClearsThePreviousFrame` — a repaint
  limited to the marching-ants rectangle clears the previous drag frame.

## 2. Above-layer image

- [x] 2.1 `view_preview::above_patch` composites the layers above the moving
  group over transparency over the presented rect at the presented level.
- [x] 2.2 Carry it through `PictureViewRust` (`move_above`,
  `move_above_origin`, `move_above_scale`, `move_above_presented`) and the
  bridge (`move_preview_above`, `move_preview_above_origin`,
  `move_preview_above_scale`).
- [x] 2.3 Invalidate the cached preview when the presented rect or level changed
  while an above-layer image exists.

## 3. Canvas

- [x] 3.1 `ImageView::beginMovePreview` takes the above-layer image and draws it
  clipped to the moving image, after the moving layer.
- [x] 3.2 `ImageView::antsUpdateRect` includes the moving image's current and
  last-painted rectangles.
- [x] 3.3 `MoveToolHandler::showPreview` passes the above-layer image.

## 4. Finish

- [x] 4.1 Run `tst_move_feedback`, `tst_linked_layers`, `tst_transform_controls`,
  `tst_large_document`.
- [x] 4.2 Run `scripts/verify-fast.sh`.
- [x] 4.3 Archive the change.
