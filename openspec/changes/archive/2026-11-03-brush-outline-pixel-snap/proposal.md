# Proposal

## Why

The brush cursor ring is only drawn cleanly when the scaled centre and radius
land on the device-pixel grid. At other zooms the ellipse is rasterised through
the image transform at fractional device coordinates, so its 1-pixel stroke lands
between pixels: the ring looks soft or broken and its edge no longer marks the
pixels the dab will cover (issue #980). The white halo is also measured in image
pixels, so its radial offset changes from 0.5 device pixels at 50 % to 2 at
200 %; the white stroke's own thickness is one device pixel.

## What Changes

- Draw the brush outline in the canvas's screen frame at
  `round(diameter × zoom)` screen pixels, with the centre and diameter snapped to
  the nearest whole screen pixel (a device pixel at a device pixel ratio of 1) so
  the ring is crisp; at integer zoom and offset this coincides with the scaled
  image-pixel boundary.
- Clamp the screen diameter to at least one screen pixel so a sub-pixel brush
  keeps a visible ring.
- Use a constant one-device-pixel halo for the circle and the Eraser Block
  square (`halo = 1.0 / devicePixelRatioF()` in the screen frame), instead of the
  circle's current one-image-pixel halo.
- Share one `brushOutlineDeviceDiameter()` helper between the ring paint and
  `brushOutlineScreenDiameterForTest()` so the test oracle cannot drift from the
  render, and add a Qt Test suite that measures the drawn ring at several zoom
  levels in addition to the existing self-test measurements.

No behavioural change to painting, hit testing, panning, transform hiding, or the
Eraser Block's document-space size. The Eraser Block's size requirement is
unchanged; only its centre snaps.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/brush-tools`: the "Brush-size outline overlay" requirement now specifies
  a device-space, pixel-snapped ring at the scaled diameter with a constant
  one-device-pixel halo and a one-pixel clamp, and gains scenarios that check the
  drawn edge, the halo, and the sub-pixel clamp.

## Impact

- `crates/pictura-app/cpp/image_view.cpp` (ring paint block) and
  `crates/pictura-app/cpp/image_view.h` (shared helper and test accessor).
- New `crates/pictura-app/cpp/tests/tst_brush_outline.cpp`, registered in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- `tst_canvas_pan.cpp` and `tst_transform_overlay.cpp` sample points follow the
  snapped centre; `tst_eraser_tools.cpp` is rechecked because the Eraser Block's
  size requirement is unchanged (only its centre snaps).
- No new dependencies, no Rust changes, no document/format changes.
