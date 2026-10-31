# Design: filter-gallery-wheel-zoom

## Context

`FilterGalleryDialog` builds its preview as a `QScrollArea`
(`galleryPreview`) whose widget is a `GalleryPreviewCanvas` sized to
`documentSize() * zoom_`. The canvas is centred in the viewport
(`Qt::AlignCenter`) and panned by the scrollbars. `setZoom` stores the factor
and calls `layoutPreview`, which resizes the canvas; nothing adjusts the
scrollbars, so their old pixel values address a different document point after
the resize. The wheel handler in `eventFilter` only reads
`angleDelta().y()` and discards `position()`.

The four sibling preview canvases (`LiquifyCanvas`, `AwaCanvas`, `VpCanvas`,
`LensCorrectionCanvas`) keep a floating-point `offset_` instead of scrollbars.
Each already has `zoomAbout(zoom, anchor)`, called by the wheel with
`event->position()` and by `setZoom` with the widget centre, so their behaviour
matches what this change adds to the gallery. They are audited, not modified.

## Goals / Non-Goals

**Goals:**
- A wheel step keeps the document point under the pointer under the pointer when
  zooming in.
- Zooming out keeps the document point at the viewport centre at the centre.
- The `−`/`+` buttons, the zoom shortcuts, the drop-down, and a typed percentage
  anchor at the viewport centre.
- The anchored point is preserved to within one screen pixel, or the scroll
  clamps to its range where the pane cannot scroll far enough.

**Non-Goals:**
- Changing the zoom ladder, the preview crop, or the committed filter stack.
- Changing the sibling preview canvases, which already anchor.
- Changing the wheel's modifier handling or the horizontal wheel.

## Decisions

### One anchored helper, with the document point read from the canvas

`documentPointAt(viewportPoint)` maps a viewport position to document space with
the canvas's current position and `zoom_`:

```
document = (viewportPoint - canvas->pos()) / zoom_
```

Reading `previewCanvas_->pos()` rather than only the scrollbar values also
accounts for `QScrollArea::AlignCenter`'s centring offset when the canvas is
smaller than the viewport. `setZoomAt(zoom, anchor)` captures the point, resizes
the canvas, then measures where that point landed and moves each scrollbar by the
residual:

```
drift = canvas->pos() + document * zoom_ - anchor
scrollbar += round(drift)
```

Moving the scrollbar by `drift` moves the canvas by `-drift`, so the point lands
at `anchor` regardless of how Qt places the widget. The alternative — computing
the new scroll value directly from the old scroll value — is off by the centring
offset when a small picture is zoomed from "fits the pane" to "overflows it".

`setZoom` delegates to `setZoomAt` with the viewport centre; `stepZoom` takes an
anchor so the buttons and shortcuts share the same path. The wheel passes
`event->position()` for zoom-in and the centre for zoom-out, mirroring
`ImageView::zoomAt`.

### Clamping is the documented edge behaviour

The scrollbars clamp to their range. When the requested anchor lies outside the
scrollable area — an edge, or a picture that still fits the pane — the point
cannot be kept and the view settles at the range end. The spec states this, and
the test pans to a position where the anchor is reachable.

### Rounding

Scrollbar values are integers, so the point is restored to within half a pixel
per axis; the spec allows one screen pixel. The canvas size is also rounded
(`qRound(doc * zoom)`), which the rest of the preview already treats as the
zoom's mapping (`scroll / zoom_`), so the test and the crop agree.

## Risks / Trade-offs

- **A stale widget position when the pane is not laid out yet** → `setZoom` is
  first called from `showEvent`, where the picture fits the pane and the scroll
  range is zero, so any drift clamps to zero; later calls happen on a visible,
  laid-out pane.
- **An asynchronous preview arriving mid-zoom** → `runPreview` calls
  `layoutPreview` at the same size; a same-size resize does not move the canvas
  or the scrollbars.
- **The wheel anchor outside the picture** → the document point is out of range
  and the scroll clamps; this is the same edge behaviour as an edge anchor.

## Migration Plan

One pull request: the dialog change, the two Qt cases, the spec delta, the test
plan, and the archive. No persisted state changes, so a revert is clean.

## Open Questions

None.
