# Design

## Context

`ImageView` owns the single brush outline (`brushOutlineActive_`) and paints it
outside the document clip. `ToolController` drives it from the hover path
(`updateBrushOutline`) and from zoom and brush-size changes
(`refreshBrushOutline`). Free Transform is modal: `handleMoved` returns before
the hover path while `transformSessionActive()`, so no mouse move clears the
ring, and Ctrl+T itself never touches it.

## Decisions

- Clear at the canvas entry point. `ImageView::beginTransformPreview` is where
  every transform session puts its first frame on the canvas, so it clears the
  outline there and the existing `update()` paints the cleared frame. That
  covers Free Transform, the constrained and projective modes, Warp, and the
  Move tool's controls-box gesture.
- Refuse re-showing at the one drawing choke point. `updateBrushOutline` clears
  instead of drawing while a session is active; the zoom and brush-size hooks
  reach it during a session, so no second guard is needed.
- The ring returns through the ordinary hover path after commit or cancel; the
  session stores no ring state.
