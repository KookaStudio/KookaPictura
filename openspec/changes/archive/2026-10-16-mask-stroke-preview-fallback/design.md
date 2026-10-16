# Design: mask-stroke-preview-fallback

## Policy

`begin_mask_paint` keeps the order the pixel stroke uses: the GPU stroke when
the dab is over the raster budget, the mode has a GPU form and GPU compute is
on; otherwise the reduced-level preview when the dab is over the budget and the
pyramid has the level; otherwise the exact per-dab raster. `mask_dab` dispatches
in the same order.

## Preview

A mask edit changes how much of the layer shows, not a colour, so the pixel
preview's source-over blend does not apply. At stroke start a `FilterProxy` at
the preview scale composites the document twice, with the mask constant 0 and
constant 255 (`mask_extremes`), and reduces the mask's coverage to the same
grid. A level pixel whose preview coverage is `k` and whose mask value would
move from `m0` toward the painted value `v` presents
`snapshot + (v - m0) * k / 255 * (revealed - hidden) / 255`. This is linear in
the mask value, which the compositor is not under every blend mode, so it is an
approximation; only the committed mask must be exact.

ponytail: the two proxy composites cost two reduced-document composites at
stroke start; a cached pair per mask would remove them if large documents with
many layers need it.

## Commit

The preview never fed the exact stroke, so the commit replays the logged
samples on the mask document and writes the stroke's whole dirty union back.
Before the refresh, the preview's extent on its level is patched back to the
stroke-start snapshot: the preview rasterizes one dab per sample with no
spacing and does not clip to the mask's rectangle, so it can paint level pixels
outside every region the exact stroke dirties. The refresh then rebuilds the
regions the exact stroke changed from level 0.

## Stroke start

`mask_document_coverage` shares the mask's `Plane` instead of copying it and
sizes the document after construction so no composite is allocated. The stroke
forks the plane on its first write.
