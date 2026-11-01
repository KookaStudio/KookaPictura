# Design

## Context

See `proposal.md` — Why. The relevant current state:

- `composite_layers` (`crates/pictura-render/src/composite_knockout.rs:19`) walks
  the layer tree through `composite_siblings` (`composite_clipping.rs`), and each
  layer reaches `composite_layer_inner` (`composite.rs:207`). A banded composite
  calls that whole walk once per 64-row band (`composite_region.rs:77`).
- `composite_layer_inner` resolves, per call: `decoded_effects`
  (`layer_source/mod.rs:543`), `effect_passes` (`:570`, keyed by `content_key`),
  `content_route` (`composite_route.rs:55`, decoding the adjustment), and then a
  route-specific source lookup (`text_raster`, `filtered_source`,
  `embedded_source`; `composited_source` is the GPU path). Nested source lookups
  can take the process-wide cache mutex four to six times.
- `BlendCtx::new` (`composite_blend.rs:34`) resolves the mask gates and
  `SourcePlanes::resolve` (called at `composite_rows.rs:215`, defined at
  `composite_blend.rs:99`) resolves the native planes, also per call.
- `content_key` (`layer_source/key.rs:10`) already hashes channel and mask
  idents, but still hashes adjustment payloads, every extra block, and the
  smart-filter `options` plane through `Plane`'s `Deref`; `effect_digest`
  (`:99`) hashes the effect block bytes. Hashing a diverged plane materializes
  it.
- Clipping and adjustment gating build synthetic `Layer` clones
  (`composite_clipping.rs:89,124,166`, `composite_native.rs:163`) and composite
  those, so a plan keyed by layer identity would miss them.
- `gpu/plan.rs` already resolves a stack once into `LayerPlan` nodes; the CPU
  path never got the equivalent. Both share `content_route`, which stays the one
  route classifier.

## Goals / Non-Goals

**Goals:**

- One resolution per layer per composite, shared by every band and region.
- Keys from plane identity, so the band path never reads plane bytes.
- Synthetic composite variants reuse resolved content instead of re-resolving.
- Every existing byte-identity oracle keeps passing unchanged.

**Non-Goals:**

- Changing the blend equation, the `Px` layout, banding thresholds, or GPU
  eligibility.
- Unifying the CPU plan with `gpu::plan::LayerPlan`; the GPU plan carries
  GPU-specific bodies (`AdjustmentPlan`, `Decline`) and the CPU plan must accept
  stacks the GPU declines.
- Caching the plan between composite calls (#1015); this plan is built per call
  and can become the session's cacheable value later.
- Changing the type model's key representation (see D4).

## Decisions

### D1. A `CompositePlan` tree built once per composite, threaded through the walk

```
CompositePlan<'a> { roots: Vec<PlanNode<'a>> }

PlanNode<'a> {
    layer:    &'a Layer,
    body:     Body,                        // route + pre-resolved source
    passes:   Option<Arc<LayerPasses>>,    // None for groups and destructive adjustments
    blend:    Option<BlendCtx<'a>>,        // None for a hidden placeholder
    sources:  Option<SourcePlanes<'a>>,    // None for a hidden placeholder
    children: Vec<PlanNode<'a>>,           // empty for a non-group
}

Body {
    Hidden,                                // an invisible layer's placeholder
    Pixels,                                // UndecodedVectorFill folds in here
    Fill(Arc<Adjustment>),
    UndecodedAdjustment,                   // no-op
    SmartFiltered { filtered: Option<CachedBuffer>, filter_base: Option<Arc<PixelBuffer>>, source: Option<Arc<PixelBuffer>>, type_raster: Option<Arc<PixelBuffer>>, has_proxy: bool },
    SmartSource  { source: Option<Arc<PixelBuffer>>, type_raster: Option<Arc<PixelBuffer>>, has_proxy: bool },
    TypeSource   { raster: Option<Arc<PixelBuffer>>, has_proxy: bool },
    Group { pass_through: bool },
}
```

`CompositePlan::build(doc, clip, effect_region)` walks `doc.layers` once and
resolves each node. `CompositePlan::build_for_layers(&[&Layer], clip,
effect_region)` builds a plan for an arbitrary layer slice, for the GPU clipping
run. `build_prefix(doc, clip, effect_region, end)` resolves only `..end` (the
render-session backdrop) and `build_suffix(doc, clip, effect_region, start)`
resolves the document background plus `start..` (the render-session frame), both
leaving the other layers as index-preserving placeholders. The recursive walk takes the plan slice alongside the layer slice, so
`composite_siblings_range` can hand the callback `&plan[i]` and a group can
recurse into `node.children`.

The builder keeps one `PlanNode` per layer, including invisible layers, so the
plan slice stays index-aligned with the layer slice; an invisible node is a
placeholder with no resolved route, source, effect, or mask.

Rationale: the tree mirrors the recursion exactly, so no layer-identity lookup
is needed and synthetic variants are the only special case (D3). It also matches
the `gpu::plan` precedent.

Alternatives considered:

- *Pointer-keyed map* (`HashMap<*const Layer, Resolved>`): no signature changes,
  but synthetic clones miss, pointer reuse after a clone drops is a
  correctness hazard, and the map lookup still runs per band.
- *Content-key map*: computing the key to find the entry is the cost being
  removed.

### D2. Resolve the fallback chain at plan time, but keep it lazy where it is region-dependent

The CPU route arms fall through when a source cannot render. Resolving each
chain at plan time to a single `Body` variant preserves that:

- `SmartFiltered`: `filtered_source` → `filtered`; else, when the layer has no
  proxy, the unfiltered `embedded_source` in `source` → the smart-source arm,
  then the `text_raster` in `type_raster` → the type-source arm; else
  `has_proxy` → `Pixels`; else no-op.
- `SmartSource`: `embedded_source` → `source`; else the `text_raster` in
  `type_raster` → the type-source arm; else `has_proxy` → `Pixels`; else no-op.
- `TypeSource`: `text_raster` → `raster`; else `Pixels` (the current fallback,
  which is a no-op for an empty rect and the pre-existing channel-less black
  rect otherwise).

`filter_base` holds the unfiltered source placed over `cached.rect` for the
filter-mask mix; `source` holds the raw embedded source for the smart-source
fallback. They are different buffers (different dimensions), so the plan keeps
both.

Only the region-dependent scaling/blend loops stay at execution time. An empty
band region makes the source blend a no-op, which is the same observable result
as the current fall-through, because `composite_pixels` is also a no-op there.

The plan stores the unfiltered `base` for a smart-filter mask mix, resolved once,
so the mask-mix loop reads a buffer instead of re-rendering the source per band.

### D3. Synthetic variants use a blend override, not a `Layer` clone

Add:

```
struct BlendOverride {
    opacity: Option<u8>,
    fill: Option<u8>,
    mode: Option<BlendMode>,
    no_mask: bool,
    no_blend_if: bool,
    isolated_group: Option<bool>,
}
BlendCtx::with_override(&self, &BlendOverride) -> BlendCtx<'a>
```

`BlendCtx`, `MaskGate`, `RasterGate`, and `VectorGate` become `Clone` (cheap:
references and `Arc`s; `pictura_core::VectorCoverage` gains `Clone, Copy`). The
executor takes `&PlanNode` plus `Option<&BlendOverride>` and derives the context
for the variant. `isolated_group` re-decides the group arm because the clipping
`opaque`/`full` variants set blend Normal, which flips a pass-through group to
isolated; reusing the base route would change bytes for a pass-through group
with a non-default Blend If. The override is applied in the legacy per-pixel
body too, so `ck_legacy` keeps comparing the same attributes the resolved
context uses.

Replaces:

- clipping `opaque` (opacity 255, Normal), `full` (opacity/fill 255, Normal, mask
  kept), and `proxy` (mask and Blend If removed) — `composite_clipping.rs`;
- the adjustment `gated` proxy (fill 255) — `composite_native.rs`;
- `coverage`'s `full` clone.

Alternative considered: keep the clones and resolve them freshly. Rejected: a
clipped adjustment or type base would decode per band, violating
`cp_adjust_once`.

### D4. Keys come from plane identity

- `content_key` and `effect_digest` hash `Plane::ident()` instead of the plane's
  bytes. Channels and masks already use idents; the change adds adjustment
  payloads and every extra block.
- `chain_hash` hashes `SmartFilter::options.ident()` instead of the options
  bytes.
- `text_key` keeps its `format!("{type_tool:?}")` content. With the plan it runs
  once per type layer per composite instead of once per band; a structural hash
  is a separate change and is deliberately deferred (`text_key` keeps its
  `ponytail:` note).
- `payload_digest()` is precomputed for decoded files. A payload whose cached
  digest is still zero hashes its bytes once per composite, not once per band.

Correctness: `Plane::ident` is assigned on first query, kept by clones, and
cleared by every mutable access, so a mutated plane yields a different key. The
existing invalidation tests in `layer_source/tests.rs` exercise this.

### D4a. Destructiveness comes from the decoded body

The plan derives whether an adjustment is destructive from the already-decoded
`ContentRoute::Fill`/`Body` (for example `fill::is_fill_kind`) instead of calling
`is_destructive_adjustment`, which would call `decode_adjustment` again. Pass
building takes the decoded adjustment (or the decoded destructiveness) so
`no_passes` does not re-decode, and the effect matte (`content_matte` →
`fill_coverage_matte`) uses the same decoded fill instead of calling
`decode_layer_fill` once per effect family. The `cp_adjust_once` requirement
stays "at most once".

### D5. Plan build order and the region

`composite_rgba` and `composite_rgba_region` keep calling `banded()` first: it
needs `decoded_effects` to detect a pattern-driven stack, and the plan build then
hits the effects cache. The duplicate digest per layer per composite is accepted
(two hashes — one in `banded()`, one in the plan build — not one per band)
rather than splitting the build into a metadata phase and a pass phase.

- banded: `clip` = the whole requested region, `effect_region = None`.
- whole: `clip` = document rect, `effect_region = None`.
- non-banded sub-region: `clip` = region, `effect_region = Some(clip)`.

Bands already inherit the whole requested `clip`, so the resolved mask gates and
feathered planes are valid for every band; this is unchanged behavior, now
computed once.

### D6. Entry points

- `composite_rgba`, `composite_whole`, `composite_rgba_region`,
  `composite_banded`, `composite_native` build one plan and execute it.
- `render_session` builds one full-document plan (`clip = rect`,
  `effect_region = Some(rect)`), executes `&roots[k..]`, and passes `roots[0]` as
  the knockout base; the prefix resolution cost is deferred to #1015.
- `gpu/clip.rs` builds a plan with `CompositePlan::build_for_layers` for the
  clipping run it hands to the CPU sibling algorithm; that path composites once,
  so no band loop is involved.
- `composite_layer_inner(canvas, layer, node, doc, deep, shallow, over)` is the
  single executor; `composite_pixels`, `composite_canvas`, `composite_adjustment`,
  the fill loops, `smart_source`, `smart_filter`, and `text_render` take the
  resolved context and source instead of rebuilding them. The executor draws
  effect passes only when `!canvas.skip_effects`.
- Cached source rasters (type, smart source, smart-filtered) resolve once. A
  region-sized generated fill buffer (a gradient or pattern fill over a band) is
  still built per band by construction; only its decoded adjustment and context
  are hoisted.

### D7. Test-only counters

- A `cfg(test)` decode-count map keyed by the adjustment's `Plane::ident`, in the
  style of `pass_build_count`, so concurrent tests cannot pollute the assertion.
- A `cfg(test)` per-key source-lookup/fetch counter, like `pass_build_count`, so
  `sources_resolve_once_per_composite` can assert one fetch per layer per
  composite.
- A `cfg(test)` native-plane resolution counter in `SourcePlanes::resolve`, so
  `banded_reuses_resolved_mask_and_native_state` can assert one resolve.
- `pictura_core::plane::materializations()` (already public) before and after the
  band loop over a diverged plane.
- The existing `pass_build_count` covers pass builds; the existing source-cache
  counters are global and do not isolate a layer.

### D8. Keep the GPU plan separate

`content_route` stays shared. The CPU plan resolves the superset of routes; the
GPU plan keeps its decline logic. If the two route walks drift, the shared
`content_route` and the GPU parity suite catch it. Unifying them is deferred
until a third consumer needs it.

## Risks / Trade-offs

- **[Reordering or reusing a stale value changes bytes]** → The plan computes the
  same values the per-band code computes; the existing banded/whole, region,
  clipping, knockout, adjustment, merge, native-depth, and reference oracles are
  the acceptance gate.
- **[Effect passes are now built for coverage canvases that skipped them]** → A
  coverage pass composites a layer a normal composite also composites, so the
  passes are needed anyway; `pass_build_count` asserts one build, and the
  executor draws them only when `!canvas.skip_effects`, so the coverage alpha
  stays unchanged.
- **[The clipping run's plan slice can drift from its layer slice]** → The
  sibling walk indexes one slice with the other; clipping-group oracle tests and
  `cp_clip_reuse` cover it.
- **[A plan held across a long composite pins cached sources]** → It holds
  `Arc`s the cache already owns, so eviction does not free an artifact until the
  plan drops; the byte budget does not govern residency during the composite,
  and the plan is dropped at the end of the call.
- **[Lifetimes spread across modules]** → `CompositePlan<'a>` and `PlanNode<'a>`
  borrow `doc`, exactly like the current walk; no owned document clone.
- **[File-size caps]** → New code lives in `composite_plan.rs`; move the
  executor there if `composite.rs` (999/1200) would exceed the cap.

## Migration Plan

1. Add `composite_plan.rs` with the plan types, the builder, and the counters.
2. Make `BlendCtx`/mask gates cloneable and add `BlendOverride`.
3. Move `composite_layer_inner` onto the plan node; migrate the pixel, group,
   adjustment, and fill routes.
4. Migrate the smart-source, smart-filter, and type-source routes.
5. Replace the clipping, coverage, and adjustment-gate clones with overrides.
6. Switch `composite_rgba`, `_region`, `_banded`, `composite_native`,
   `render_session`, and `gpu/clip` to build a plan.
7. Switch keys to idents; add the counter and materialization tests.
8. Add the 4000² profile and record before/after numbers in the PR.
9. Run the oracle and gate suites, archive the change, and run `verify-fast`.

## Open Questions

None that change the approach or the task breakdown. Whether #1015 caches the
plan between calls is decided there.
