# Design — Distort preview warp

## Context

The Free Transform preview draws a cached base composite and the moving layer
under a live transform. For a projective mode the moving layer image was drawn
with `QPainter::setTransform` using a projective `QTransform`. Qt's raster
engine splits such an image into triangles; at some angles one triangle is
dropped, which is the black-triangle preview reported against #748. The
committed engine already samples the full quad.

## Warp

`image_view_transform_warp.cpp` adds `warpTransformPreview`:

- `coefficients` is the source-local-to-destination-local map in `QTransform`
  constructor order; `sourceRect` is the source bounds in that local space.
- The forward map bounds the destination from the source corners; the visible
  document region (the widget rect mapped back to image coordinates) is
  intersected with it.
- The inverse `QTransform` maps each destination pixel centre to a source point.
  A point on the far side of the horizon (opposite the source majority's
  forward divisor) is skipped, as is a point outside the source rect or image.
- Bilinear sampling on a premultiplied `ARGB32` image keeps the preview's alpha.
- The result is an image plus its local destination origin. The paint path
  draws it at that origin after the usual pan/zoom translate; no projective
  `QTransform` is involved.

The Qt projective draw remains as a fallback when the warp returns no image
(e.g., no visible region or no source rect).

## What stays the same

- The cached base/layer preview, opacity handling, overlays, and commit path.
- The reference point, guide snapping, and gesture math.
- The front-side rule already applied by the projective engine.
