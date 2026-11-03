# Design

## Where the time went

`move_gesture_profile` (ignored) times one gesture on a background under a
canvas-sized opaque layer. On this machine it runs at half size per side
(`PICTURA_PROFILE_DIVISOR=2`, 8253×8098), a quarter of the area. The costs scale
with area.

| Phase | Half size | Full size, ×4 |
|---|---|---|
| Whole-content backdrop (before) | 845–981 ms | ≈3.4–3.9 s |
| Whole-layer image (before) | 361–420 ms | ≈1.4–1.7 s |
| View press at fit, level 4 (after) | 57 ms | ≈230 ms |
| View press at 100 %, level 0, 2560×1440 view (after) | 115 ms | 115 ms (view-sized) |
| Commit, region recomposite and display (unchanged) | 1031–1321 ms | ≈4.1–5.3 s |

Runs were capped at six cores and `nice 19`. The first view press at fit took
116 ms. Nearly all of it was `box_reduce` summing each footprint pixel by pixel,
so it now adds whole source rows into column sums (a loop the compiler
vectorizes), and each output sums its columns. The output is unchanged.

## View preview

The canvas already reports the level and document rect it presents
(`set_presented_view`). For one plain pixel layer whose content is larger than
`VIEW_PREVIEW_PIXELS` (4 MP), the press builds two images:

- **Backdrop**: the document with the layer hidden, over the layer's content
  clipped to the view and aligned to the level grid. Level 0 composites that
  region exactly, as the transform patch does. A deeper level composites a
  `FilterProxy` of it, a box-reduced copy of the region. The hidden layer's planes
  are dropped from the copy so the reduction skips them.
- **Moving image**: the layer over the view grown by a quarter view on each side,
  reduced to the level. The region may extend past the canvas, because a drag
  brings off-canvas pixels into view. A one-layer document the size of the region
  carries them through the proxy.

The canvas draws both images scaled by `2^level` in document coordinates. A
backdrop patch with no image (none of the layer is on screen) draws only the
layer over the live canvas.

The work follows the view. At fit zoom it reads each layer's pixels in the view
once and composites at the level; at 100 % it is a few viewports.

## Faithful reduction

`FilterProxy` reduces pixel planes and masks, but layer effects, live type,
smart-object sources and filters, fill layers and undecoded vector fills keep
their document-scale sizes. Above level 0, a document with any of them keeps the
full-resolution preview. `FilterProxy::reduces_faithfully` is the check.

## Following the drag

The moving image covers a quarter view past each edge. On each pointer move the
tool asks Rust whether the layer pixels now on screen (the view shifted by minus
the drag) are still covered. When they are not, Rust rebuilds the image around
the shifted view, and the canvas swaps it in with its new origin.

## Ants

A view preview's moving image is partial, so scanning it no longer gives the
layer's box. Rust reports the content bounds it measured. The full-resolution
path still scans its image, once per image.

## Limits

- ponytail: the view is the one at the press. Panning or zooming during a drag
  is not followed; the backdrop and the level stay those of the press.
- ponytail: a link set, a shape, and a layer with effects still bake their whole
  content, as before.
- The commit is unchanged; #1157 tracks it.
