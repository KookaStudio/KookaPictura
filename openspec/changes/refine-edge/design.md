# Design: refine-edge

## Engine model (`pictura-select::refine`)

Refine Edge is largely closed; the fetched CS6 corpus documents only the
observable behavior (`docs/08-selection/refine-edge.md`). This is an
implementation-agnostic approximation, marked inferred.

The pipeline, applied in CS6's documented order:

1. **Edge band.** Find the pixels within the boundary band. With Smart Radius
   off the band is a fixed `Radius` widening (dilate ∪ erode) of the 50 %
   contour; with Smart Radius on the band adapts to the local edge softness:
   a hard edge (large local gradient in the composite) gets a narrow radius
   and a soft edge a wide one, scaled by `Radius`.
2. **Coverage re-estimation.** Inside the band, re-estimate alpha from the
   underlying composite. The band's interior pixels keep the original coverage
   and a smooth field blends the outside back to the original so nothing
   outside `Radius` of the original boundary changes. A lightness/edge response
   nudges partial coverage toward where the image edge actually is (the
   `smart_radius` refinement the brushes would otherwise add).
3. **Smooth** — the existing majority filter (`Selection::smooth`).
4. **Feather** — the existing Gaussian blur (`Selection::feather`).
5. **Contrast** — steepen the coverage ramp about 128, pushing partial values
   toward 0/255 by `amount` percent.
6. **Shift Edge** — move the 50 % contour inward (negative) or outward
   (positive) by `amount` percent of the band width, using the distance field
   built in step 1.

`refine(mask, image, settings) -> Selection` runs these and is a bit-identical
no-op when every refinement is at its default (Radius 0, Smooth 0, Feather 0,
Contrast 0, Shift Edge 0).

### Decontamination

Documented behavior: color fringes are replaced with the color of nearby fully
selected pixels, with replacement strength proportional to edge softness. The
model is alpha matting foreground estimation: for an edge pixel estimate `F`
from nearby fully-opaque foreground colors, then replace the color with
`lerp(I, F, (1 - α) * amount)`. It writes a new pixel buffer and never mutates
in place, so CS6 forbids in-place output while it is on.

## App

`RefineEdgeDialog` is non-modal with its own event loop presented through
`runDialog` (capability `ui/dialog-presentation`), matching every other app
dialog. It computes a `kPreviewSize` proxy of the refined mask on every change
(the engine is cheap on a proxy), and on OK the bridge applies the settings to
the full document.

Output To maps to the app's existing operations:

- **Selection** — replace the active selection.
- **Layer Mask** — write the refined mask into the active layer's mask.
- **New Layer** — `layer_via_copy` of the active layer through the refined
  mask.
- **New Layer with Layer Mask** — as New Layer, plus a raster mask holding the
  refined coverage.

Decontaminate Colors is offered only with the new-layer / new-document outputs
(CS6 requires it); Selection and Layer Mask are disabled while it is checked.
The engine's `decontaminate` result is written into the copied/created layer's
color.

## Keyboard (deferred surface)

CS6 binds `F`/`Shift+F` (view mode), `X`, `P`, `J`, and `Shift+E`/brackets in
the dialog. Only the View Mode combo, Show Original (`P`) and Show Radius (`J`)
toggles, and the refinement brushes' *result* are modelled; the brush strokes
themselves are a `ponytail:` ceiling.

## Determinism

No RNG anywhere. All refinements are deterministic convolutions / array passes
on the CPU. Golden comparisons run on the CPU (rule 6).
