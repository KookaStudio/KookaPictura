# Design

## Context

See `proposal.md` for motivation and the delta specs for the contract.

Three facts shape the approach:

- The app already composites exactly the dab region:
  `refresh_region_buffer` calls `composite_region_active(doc, rect)`; the commit
  refreshes the pending present, and the stroke tiles are the history hint. The
  composite region is therefore the dirty rectangle the issue asks for; only the
  effect build ignores it.
- `Plane::ident()` assigns a fresh id when the ident is zero, and every mutable
  access clears it. A write therefore changes the pass cache key once, on the
  next lookup, and that is what triggers the whole-layer rebuild.
- Every effect stage is local with a known support. `dilate`/`erode` are
  separable O(n) max/min filters, the Gaussian is `ceil(3σ)` per side
  (`sigma_from_radius`), and each builder already pads its matte by the sum of
  its stages' supports (`padded`). A build over `region + reach` therefore has
  every sample an output pixel in `region` needs, provided `reach` also includes
  the sample offset for a shifted family: drop shadow and inner shadow read at
  `x - ox`, so they grow by `padded` support plus `|ox| + |oy|`. Satin already
  includes its offset in `padded`.

## Goals / Non-Goals

**Goals:**

- A stroke present on a styled or feathered layer costs O(present region +
  effect reach), independent of the layer area.
- Region output stays byte-identical to a full composite.
- A paint present, commit, and cancel refresh the effect halo and mask feather
  ramp they change.
- No write-tracking API and no per-effect merge protocol.

**Non-Goals:**

- Tile-granular plane storage (issue #1014).
- GPU effect rendering; a styled layer still declines the GPU path.
- Changing the effect math or the existing paint-bounds reach (`effects_pad`);
  the paint refresh uses a separate `paint_reach` that also covers inner
  effects. The shared-cache budget is unchanged.

## Decisions

### Build the pass for the composite region, not a patched full pass

The alternative the issue sketches is to keep a cached full pass and, on a
write, rebuild `dirty ∪ reach` and merge it into the cached pass. That needs a
per-layer dirty registry, a structural cache key that excludes the content
ident, and a merge/clear rule per effect family (including clearing alpha where
an eraser removed content). Because the composite only ever needs the pass
inside its own region, the region build *is* the dirty-rect rebuild: there is
nothing outside the region to keep. This removes the merge problem entirely.

Byte-identity argument, per stage: for output in `R`, the matte buffer must
hold `R` grown by the total reach. The buffer edge then sits at or beyond the
last tap the output needs, so clamp-to-edge never contributes to a pixel in
`R`. Chained stages keep their existing summed pad; intersecting that `padded`
with `grow(R, reach + |ox| + |oy|)` preserves the property for a family that
reads a shifted sample (drop shadow and inner shadow), while satin already
includes its offset in `padded`. The region builders clip both the matte buffer
and the output loop, since several families currently iterate the whole
`source` rect.

### Sub-region passes are transient; the full-canvas pass stays cached

A full-canvas composite keeps the existing content-keyed cache. A non-banded
sub-region composite builds its pass transiently. Putting the region into the cache key
would make every dab's slightly different rectangle a new entry and would let a
stroke evict the smart-filter, text, and embedded sources of other layers.

Banded composites are the exception that forces care: a banded region composite
would otherwise rebuild the pass once per band. `composite_banded` therefore
keeps the full-pass path (`effect_region = None`), so one cached pass is replayed
into every band, exactly as today. The banded path already accepts a whole-layer
pass; this preserves it.

### The canvas carries the overall region and the optional build region

`Canvas` gains:

- `clip: PsdRect` — the overall requested composite region, used for mask
  feather sampling during content compositing. Derived canvases (isolated
  group, clipping coverage, knockout base) inherit it, so a band and a knockout
  temp do not each build their own feather plane.
- `effect_region: Option<PsdRect>` — `Some(region)` when a transient region
  build is wanted (a non-banded sub-region composite), `None` for the cached
  full pass (a full composite or a banded composite).

`Canvas::new` and `Canvas::new_region` set `clip` to their own bounds and
`effect_region = None`. `composite_rgba_region` sets `Some(region)` only when the
region is a proper sub-rect of the canvas and the composite is not banded;
`composite_whole` sets `None`, and `composite_banded` sets `clip` to the overall
requested region for every band and `effect_region = None`. `with_cover_from` and
the derived-canvas constructors copy both fields.

### Builders take an optional region

`build_layer_passes(layer, doc, effects, region: Option<PsdRect>)` passes the
region to each family. With `Some(R)`, a builder:

1. computes its existing `padded` from the layer source and its total reach;
2. intersects `padded` with `grow(R, reach + |ox| + |oy|)` for drop shadow and
   inner shadow (satin already includes its offset), or `grow(R, reach)` for the
   other families, and returns empty when that misses;
3. clips its output loop to `R`;
4. passes one shared per-layer clip, `grow(R, max enabled build reach)`, to
   `mask_alpha` in every builder, so the raster and vector feather planes are
   built once per layer rather than once per effect family. The shared build
   reach includes the shift for drop shadow and inner shadow.

With `None`, behaviour is the current whole-layer build. The gradient overlay
uses a new `gradient_rgba_region` that keeps the full layer (or document) extent
for the gradient geometry but fills only the sub-rect; the pattern overlay uses
the existing `pattern_tile_region` with an offset index.

### Mask feather is clipped to the sampled region

Content compositing passes `canvas.clip` to `mask_alpha_unit` instead of
`mask_refine::canvas(doc)`. The feather cache already keys on `clip`, and its
contract is that samples inside the clip are exact, so this is a pure extent
reduction. It also aligns the CPU path with the GPU path, which already passes
`region.rect()`.

### The paint reach is computed once and applied at every refresh site

At stroke start the app resolves the target layer and stores
`paint_pad = paint_reach(layer) + (mask stroke ? mask_feather_reach : 0)` on the
stroke session. `paint_reach` is a new public `pictura-render` helper, not the
existing private `effects_pad`: `effects_pad` deliberately bounds only outer
influence and omits inner shadow, inner glow, and satin. `paint_reach` is the
maximum change spread over all enabled effects: drop shadow
`|distance| + spread_dilate(spread, size) + blur_support(size)`, outer glow
`spread_dilate(spread, size) + blur_support(size)`, stroke `size`, bevel `reach`, inner shadow `|ox| + |oy| + erode + blur`, inner glow
`erode + blur`, satin `|ox| + |oy| + blur`. `mask_feather_reach` becomes a small
public helper over `mask_refine::reach` and returns 0 for a mask with no feather.
The app grows the dab rect by `paint_pad` before `mark_stroke_tile` and
`queue_present`, the preview replay's `stroke_tiles.mark`, the mask preview
replay's `tiles.mark`, the mask commit's `tiles.regions`, and the cancel's
internal `composite_region_active` patch; `paint_pad` is threaded into
`end_mask_paint`, `mask_dab`, and `mask_gpu_dab`. The reduced-resolution preview
present is exempt: it carries the level-space extent and the exact commit
repairs the composite.

This keeps the present rectangle incremental (one dab's footprint plus a constant) and
makes the history hint cover the halo, so undo does not capture a stale
composite.

### Rejected: freeze passes during the stroke

It is the smallest diff, but the in-stroke composite must match a full composite
(`tools/paint-engine`) and cached sources must recompute when their inputs change
(`compositing/layer-source-cache`). A stale pass violates both and shows a halo
that lags the brush.

## Risks / Trade-offs

- **Byte-identity regression at region edges.** → Each builder intersects its
  summed `padded` with `R + total reach`, plus the sample offset for drop shadow
  and inner shadow; add region-vs-full equality tests for every effect family
  and for a mask dab, not just one composite oracle.
- **A tall sub-region composite still builds the whole-layer pass.** → It is the
  current behaviour and keeps one build per band; a future tiled pass (#1014)
  can remove it.
- **Repeated same-region composites (scroll, thumbnails) rebuild the pass.** →
  The build is O(region + reach) and the expensive sources remain cached; the
  ignored benchmark gains a canvas-sized styled-layer case rather than relying
  on the warm/cold ratio of the old 400×200 text layer.
- **A very large effect reach grows the present rect.** → The growth is bounded
  by the paint reach and never exceeds the full composite; the alternative is a
  stale halo.
- **Feather-plane churn during a mask stroke.** → The builders pass one shared
  per-layer clip (`grow(R, max enabled build reach)`), so the plane is built
  once per layer, not once per effect family. The planes are region-sized, so
  the shared cache's four-entry ceiling churns cheap planes; leave the ceiling
  and revisit only if a profile shows it.
- **Overlay geometry must keep absolute coordinates.** → The region variants
  keep the full extent for the gradient/pattern math and only iterate the
  sub-rect; equality tests cover an aligned and an unaligned gradient and a
  linked and an unlinked pattern.

## Migration Plan

No data or API migration. Revert the change to restore the previous build
region; the cache format is unchanged.

## Open Questions

- Whether to cache sub-region passes if a scroll/thumbnail profile shows the
  transient rebuild; deferred until a measurement exists, and it would not
  change the specs or task breakdown.
