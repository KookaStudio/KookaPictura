# Design: transform-overlay-unclipped

## Context

`ImageView::paintEvent` draws the composite/preview under
`painter.setClipRect(docClip)`, then opens a second document-clipped scope for
the selection overlays and the transform quad (`image_view.cpp:707`). The
transform quad therefore disappears at the document edge even though the layer
may extend far past it. The preview content itself must stay clipped: Photoshop
clips layer pixels to the canvas.

## Goals / Non-Goals

**Goals:**
- The full box, handles, rotate knob, and reference point paint past the
  document edge, in the modal session and the Move tool's controls box.
- The rasterised preview stays clipped to the canvas.

**Non-Goals:**
- Re-clamping the view or growing the scroll range for off-canvas layers.
- Crop/perspective/slice overlay clipping, which is separate chrome.

## Decisions

- **Extract the overlay into `paintTransformOverlay(QPainter&)` and call it
  after the document-clipped scope closes.** The painter is still in image
  space (rotation + pan + zoom), and the widget itself is the only remaining
  clip, so the box paints anywhere on the canvas widget. Keeping the block
  inline and merely wrapping it in a save/restore would also work, but the
  extraction keeps `paintEvent` readable and names the unclipped scope. The
  moved code is a pure move (same drawing order, same painter state).
- **Preview stays under `docClip`.** No change to the composite/preview scopes:
  only the overlay's clip is dropped, matching "only the rasterised result is
  clipped to the canvas".
- **Test the painted pixels, not the clip stack.** `tst_transform_overlay`
  scales a layer so one corner lands outside the document, then grabs the canvas
  and compares the off-canvas corner handle's centre pixel with the on-canvas
  corner's: both must be the white handle fill. Restoring the clip makes the
  off-canvas pixel the workspace colour, so the test has teeth.

## Risks / Trade-offs

- [The overlay can now paint over the workspace beyond the document] → Intended:
  the canvas widget is the workspace; CS6 shows the box across it.
- [Handles drawn past the widget are clipped by Qt] → Unavoidable and harmless;
  the box is as visible as the widget allows.

## Migration Plan

None: paint-only.

## Open Questions

None.
