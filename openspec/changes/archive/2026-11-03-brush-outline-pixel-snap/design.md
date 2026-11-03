# Design

## Context

See `proposal.md` for motivation and `specs/tools/brush-tools/spec.md` for the
contract.

Today `ImageView::paintEvent` draws the ring while the painter is in image space
(`rotation * translate(offset) * scale(zoom)`). The device geometry is therefore
`rotation(offset + zoom × imagePos)` with radius `diameter × zoom / 2`, and a
cosmetic pen keeps the stroke at one device pixel. The size is mathematically
right, but the centre and radius are fractional device coordinates, so the
aliased 1-pixel stroke lands on different subpixel positions at each zoom. The
circle's white halo is `1.0` **image** pixel, so its radial offset from the black
ring is 0.5 device pixels at 50 % and 2 at 200 %; the white stroke's own
thickness is one device pixel.

The screen-space overlay pattern already exists: `paintCropGroupOverlays`
(`image_view_overlays.cpp:490`) sets `painter.setTransform(viewRotation())`,
clears clipping, and maps image points with `p * zoom_ + offset_`. The eyedropper
sampling ring is drawn the same way.

## Goals / Non-Goals

**Goals:**

- One screen-space geometry for the ring, snapped to the nearest whole screen
  pixel (a device pixel at a device pixel ratio of 1), at every zoom.
- Keep the ring under the other overlays, outside the document clip, and hidden
  during pan and transform sessions exactly as today.
- Keep the Eraser Block square axis-aligned in image space, so it still turns
  with Rotate View.

**Non-Goals:**

- No change to the brush tip, stroke engine, hit testing, or dab footprint.
- No change to the sampling ring, selection ants, or other overlays.
- Device-perfect snapping under Rotate View is not required; see Risks.

## Decisions

### Draw the ring in the canvas screen frame

The ring block is not inside an existing save/restore: it runs after the
document-clip block restores the painter, so the painter is still in image space.
The block SHALL therefore save and restore explicitly:

```cpp
painter.save();
painter.setTransform(viewRotation());
painter.setClipping(false);
// ... draw the ring ...
painter.restore();
```

Map the pointer with `brushOutlineImagePos_ * zoom_ + offset_`, mirroring
`paintCropGroupOverlays`. `viewRotation()` is a rotation about the widget centre
(a rotation plus a translation), not a pure rotation; it preserves the Eraser
Block square's orientation under Rotate View while removing the `scale(zoom)`
that produced the fractional geometry. The `restore()` is required so the
selection ants, transform overlay, guides, crop overlays, path/type overlays, and
move outline that follow still draw in image space.

Alternative considered: keep the image-space transform and snap in image space.
That requires inverse-rotating a snapped device point back through the view
rotation, and the square's edges would still be scaled. The screen-frame draw is
smaller and matches the existing overlays.

### Snap the diameter and centre

```cpp
const double diameter = brushOutlineDeviceDiameter();
const QPointF raw = brushOutlineImagePos_ * zoom_ + offset_;
const QPointF centre(std::round(raw.x()), std::round(raw.y()));
const double radius = diameter / 2.0;
```

`brushOutlineDeviceDiameter()` is a private helper returning
`std::max(1.0, std::round(brushOutlineDiameter_ * zoom_))`; the paint block and
`brushOutlineScreenDiameterForTest()` both call it. Clamping to one screen pixel
keeps a visible cursor for a sub-pixel brush. The centre snaps to whole screen pixels
(device pixels at a device pixel ratio of 1), which keeps the stroke crisp. At a
fractional device pixel ratio an odd logical coordinate maps to a half-device
pixel, so the centre can be off the device grid; this matches the other canvas
overlays, and a DPR-aware pass is a follow-up. Symmetry is exact when
`round(D × Z)` is even; an odd rounded diameter has a half-integer radius, so the
ring can be one pixel asymmetric. At integer zoom and offset the snapped edge
coincides with the scaled image-pixel boundary; at fractional zoom it is on the
screen-pixel grid, not on every scaled image-pixel boundary. Both the circle and
the square use the same snapped centre and `radius`.

Alternative considered: round only the diameter and leave the centre fractional.
That fixes the reported size but leaves the ring's edge subpixel and the stroke
asymmetric, which is the actual complaint.

### One-device-pixel halo for both shapes

Replace `halo = brushOutlineSquare_ ? 1.0 / zoom_ : 1.0` with
`halo = 1.0 / devicePixelRatioF()` in the screen frame. The cosmetic pen is one
device pixel, so this keeps the radial offset between the black and white rings
one device pixel at every zoom and DPR. The square's old `1.0 / zoom_` was one
logical pixel; both shapes now use one device pixel, and the circle's halo stops
growing with zoom.

### Test helper reports what is drawn

`brushOutlineScreenDiameterForTest()` returns `brushOutlineDeviceDiameter()`,
matching its comment. The helper lives in `image_view.cpp`, so `image_view.h`
needs no `<cmath>`. Existing callers (`tst_eraser_tools`, `selftest_canvas_view`)
use values where rounding is a no-op, so they keep passing. The new suite
computes the snapped centre from `offset()` and `zoom()` (not `imageToWidget`,
which includes the view rotation); the suite runs with the view unrotated.

### Test measurement

A 1-px cosmetic stroke on a circle of geometric diameter `round(D × Z)` spans
`round(D × Z) + 1` pixels across the centre row (`hi - lo + 1`), with one stroke
pixel at each edge. The tests therefore measure the outer dark-edge distance
`hi - lo`, not the pixel count, and assert
`abs((hi - lo) - round(D × Z)) <= 1`; when `round(D × Z)` is even the assertion is
exact. Symmetry is asserted only for even rounded diameters: odd rounded
diameters have a half-integer radius and may be off by one pixel. The zoom and
symmetry rows use `D=8` (all products at 0.5/0.75/1/1.25/1.5/2/3/4 are even:
4, 6, 8, 10, 12, 16, 24, 32); a separate rounding case uses `D=10` at 1.25×
(`12.5 -> 13`) with the ±1 tolerance and no symmetry claim.

## Risks / Trade-offs

- **Rotate View still fractional** → the centre is snapped in the unrotated
  widget frame; a non-zero view rotation maps it to a fractional device point, so
  the ring can still be soft while the canvas is turned. This issue is about
  zoom; device-perfect rotation snapping is a follow-up if it is reported. The
  test suite runs with the view unrotated.
- **HiDPI snapping is in logical pixels** → the centre snap quantises to the
  screen-pixel grid, which is the device grid only at an integer device pixel
  ratio. The halo is divided by `devicePixelRatioF()` so it stays one device
  pixel. This matches every other overlay in the canvas; physical-pixel centre
  snapping would need a DPR-aware pass over all of them.
- **Odd scaled diameters** → a half-integer radius can put the cardinal stroke on
  either side of a pixel boundary, so symmetry is exact for even rounded
  diameters and may be off by one pixel for odd ones. The zoom/symmetry test uses
  `D=8`; the `D=10` at 1.25× rounding case asserts the diameter with the ±1
  tolerance and makes no symmetry claim.
- **Existing sampling tests** → `tst_canvas_pan::panHidesTheBrushRing` and
  `tst_transform_overlay` sample the old unrounded edge. They must use the snapped
  centre or a small scan, or they can flake. `tst_eraser_tools` also samples the
  block edge; the Eraser Block's size requirement is unchanged and only its centre
  snaps, so it keeps its existing tolerance.
