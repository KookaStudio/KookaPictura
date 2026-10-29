# Proposal

## Why

Painting a layer that carries a layer style or a feathered mask rebuilds the
effects over the whole layer on every stroke frame. The app already composites
only the dab's region, but the effect-pass cache ignores that region: a write
clears the plane ident, the next lookup assigns a fresh ident, and the pass is
rebuilt over the whole padded layer. On a near-canvas-sized layer the present
cost is dominated by that rebuild (a Gaussian over the layer area per effect),
not by the dab.

The same investigation found a correctness gap: a paint present, commit, and
cancel refresh only the dab rectangle, so the effect halo and a feathered mask's
ramp outside the dab stay stale in `doc.composite` and the displayed level until
some later full recomposite.

## What Changes

- Build effect passes for the composite region instead of the whole layer. A
  non-banded sub-region composite builds a transient pass over the region grown
  by the effect's build reach; a banded sub-region composite may build the
  whole-layer pass and replay it into each band; a full-canvas composite keeps
  the existing content-keyed whole-pass cache. Every effect family (drop shadow,
  outer glow, inner shadow, inner glow, stroke, satin, bevel,
  color/gradient/pattern overlay) clips its build buffer and output to the
  region, byte-identical to the same slice of a full build. The build reach
  includes the sample offset for drop shadow and inner shadow; satin already
  includes its offset.
- Region-scope the gradient and pattern overlay source buffers, so an overlay
  layer's present is O(region), not O(layer).
- Bound the feathered-mask plane by the mask's bounds intersected with the
  sampled region, grown by the feather reach, instead of the whole canvas, for
  both effect builds and content compositing.
- Grow a paint present, commit, and cancel rectangle by the painted layer's
  paint-change reach (outer and inner effects), plus the mask feather reach for
  a mask stroke, so the refreshed region matches a full recomposite.
- Add regression checks for region-pass equality per effect family, a mask dab,
  and a styled-layer dab whose halo moves.

No breaking changes. No new dependencies.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-source-cache`: effect passes build for the composite
  region and equal the full build's slice; the full-canvas pass stays cached,
  a non-banded sub-region pass is transient, a banded sub-region reuses one
  whole-layer pass; the benchmark gains a canvas-sized styled layer.
- `compositing/mask-refinement`: the feathered plane is bounded by the mask's
  bounds intersected with the sampled region, grown by the feather reach, and a
  region composite rebuilds only that region's plane.
- `tools/paint-engine`: the in-stroke present rectangle is the dabs' footprints
  grown by the painted layer's paint-change reach and, for a mask stroke, the
  mask feather reach, still incremental.
- `compositing/described-damage`: a paint present, commit, and cancel refresh the
  stroke's changed pixels grown by the paint-change reach (and mask feather reach), so the
  stored composite equals a full recomposite for a styled or feathered layer.

## Impact

- Render: `layer_source` (pass lookup/build region), `layer_effects` (every
  builder plus the shared matte/mask helpers), `mask_refine` (clip use),
  `composite`/`composite_rows`/`composite_native`/`composite_region` (canvas
  region), `composite_clipping`/`composite_knockout` (derived canvases),
  `gpu/assemble.rs` (the GPU mask clip), `fill` (region gradient), `overlays`.
- App: `impl_paint` (present, commit, cancel, mask dab), `document_session`
  (paint reach carried by the stroke session).
- Tests: `pictura-render` layer-effect cache/region suites, app damage-policy
  and paint suites; the ignored `source_cache_profile` benchmark.
- Specs: the four capabilities above; no other contract changes.
