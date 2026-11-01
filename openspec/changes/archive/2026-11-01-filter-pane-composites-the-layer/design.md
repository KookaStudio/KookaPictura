# Design

A base plus an overlay cannot show a blend mode that reads its backdrop, so both dialogs take one
composite from a new `filter_proxy_composite`, which shares the cached `FilterProxy` of the canvas
proxy. The caller keeps scaling pixel-sized slots. A kind the mapping refuses renders the document
unfiltered, as the old layer proxy did.

`reduce_layer` maps vector-mask points through the region and scale. Points are 1/256 document
pixels, so the mapping is the pixel map at that unit. `set_feather` scales the radius and retires
the coverage cache with a new identity, so a reduced mask cannot read the full-size mask's cached
feather plane.

## Limits

- The copy is clamped to the canvas, so a layer overhanging it is filtered without the overhang. A
  Shear wrap over such a layer differs from the commit there.
- A Smart Object already carrying Smart Filters composites its filtered source, so these previews
  show the existing chain without the new filter. A plain Smart Object shows it on its proxy pixels.
- Layer effects and live type stay at document scale inside the copy.
- Each update now composites the reduced stack, and the Blur Gallery's two sizes share one cache
  slot, so entering or leaving a drag rebuilds the copy.
