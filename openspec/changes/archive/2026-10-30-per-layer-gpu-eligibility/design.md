# Design

## Context

See `proposal.md` for motivation and the delta specs for the contract. The
current GPU path is shaped by four facts:

- Eligibility is one document-wide walk (`check_supported`) that returns the
  first decline, and dispatch is a second, hand-written route decision inside
  `Gpu::composite_layer` (`gpu/backend.rs:623`). They agree today by
  construction, not by sharing code.
- The GPU canvas is packed 8-bit RGBA (`pack_word`/`unpack_word`); the CPU
  accumulator is `f32` (`Canvas.px`). The parity contract is ±1 LSB against the
  `f32` CPU oracle, and a region composite must be byte-identical to the full
  composite's slice.
- The CPU already builds every expensive per-layer artifact and caches it by
  content: type rasters, smart-filter chains, embedded sources, decoded effects,
  and whole-layer effect passes (`layer_source`). It does **not** cache the
  composited source for a smart-filtered layer with a filter mask, nor the placed
  smart-object source.
- The CPU clipping algorithm is a sibling-group pass (`composite_clipping.rs`):
  base at full opacity into an empty group, coverage from the base alone, each
  clipped layer lerped into the group by that coverage, then the group blended
  onto the backdrop with the base's opacity and mode.

## Goals / Non-Goals

**Goals:**

- One per-layer plan drives both eligibility and dispatch.
- Accept the common CS6 routes: cached raster sources, fills, the four LUT
  adjustments, effects, and clipping.
- Preserve ±1 LSB parity, region byte-identity, and the unchanged CPU oracle.
- Keep the cached raster and fill uploads keyed by content, so interactive
  presents do not re-upload unchanged sources. Effect-pass and clipping-group
  uploads are transient because their CPU artifacts are built per composite.
- Split the new code along the existing module seams so no file crosses its cap.

**Non-Goals:**

- A mixed GPU/CPU canvas inside one composite.
- 16/32-bit native-depth compositing, knockout, non-default Blend If, and
  Dissolve.
- GPU-native fill generation, effect building, or a GPU 3-D colour lookup.
- Any behavioral change to the CPU compositor or the public API. A pure
  refactor seam that exposes the clipped group buffer is allowed.

## Decisions

### A per-layer plan, a per-document fallback

`plan_layer(layer, doc)` returns one plan node; `plan_stack(doc)` returns the
whole tree:

```text
LayerPlan { effects: Passes, body: Body }
Body:
  Pixels                      channels, uploaded as today
  Raster { buffer, rect, id } a cached composited source
  Fill   { buffer, rect, id } a rasterized Solid/Gradient/Pattern/vscg fill
  Adjustment(AdjustmentPlan)  inline kind or LUT table
  Group  { pass_through, children }
  ClipGroup { base, clipped } the base plus its clipped run, planned as one unit
  Decline(GpuError)
```

`plan_stack(doc)` walks the visible layers in stack order, carries the
clipping-run state, and returns the first `Decline` plus the plan tree for
dispatch. Eligibility is `plan_stack(doc).is_ok()`. `composite_gpu_region_raw`
takes the plan tree and `Gpu::composite_layer` dispatches the node it is given;
neither re-derives the route.

A hidden layer is not dispatched. A hidden clipping base suppresses its whole
run, so the run is not planned either; the walk must carry that state rather
than skipping the base in isolation.

This is the decision unit the issue asks for. The *fallback* stays per document:
if any visible layer declines, the whole composite runs on the CPU. A mixed
canvas is rejected because a GPU→CPU handoff would quantize the backdrop to 8
bits mid-stack, which can exceed the ±1 LSB contract against the `f32` oracle.
Pre-compositing a declining layer on the CPU and uploading it is only valid when
the layer does not read the running backdrop; effects, Blend If, and knockout
do. A clipping group is isolated, so it can be precomposited (see below) without
being a mixed canvas: the group buffer is uploaded once as a source.

Alternatives considered: a per-layer readback/merge loop (breaks parity, one
sync per boundary); a fully `f32` GPU canvas (memory and shader rewrite);
keeping the two route implementations and widening both (the drift this change
exists to remove).

### The uploaded raster is the composited source, not the chain

For a proxy-less type layer, a smart-filtered smart object, and a channel-less
smart-object source, the GPU needs the exact straight-alpha RGBA the CPU feeds
`blend_into`, not the intermediate chain. `layer_source::composited_source`
returns a `CachedBuffer`:

- type: the existing `text_raster` at `layer.rect`;
- smart-filtered: `filtered_source` mixed per pixel with the unfiltered
  `render_smart_source` through the smart-filter mask, reproducing
  `composite_smart_filtered_source` exactly;
- smart source: `render_smart_source` at `layer.rect ∩ document`.

The cache key is the existing content key plus the route kind and the
smart-filter mask (its rect, enabled flag, extend-with-white flag, and bytes);
`content_key` alone does not cover the mask, so the mix would go stale after a
mask edit. Uploading `filtered_source` alone is wrong whenever the filter mask
is enabled: the CPU shows the unfiltered base outside the mask.

If a cached raster cannot render (an unreadable embedded payload, an empty
type model), the layer takes the pixel route when it carries a proxy; with no
proxy it declines with the existing `UnsupportedSmartObject` or
`UnsupportedText`, so the CPU keeps the opaque-black channel-less result.

Alternatives considered: upload the chain and the base and mix in the shader
(extra binding and shader branch); build the mix on every present (defeats the
cache).

### Raster uploads are keyed by content, not stamps

`UploadKey::Raster { content: u64, rect, doc, gray }` where `content` is a
content hash exported by `layer_source`. The existing stamp key cannot apply: a
raster has no `Plane` stamps. An `Arc` pointer is not safe because the resident
store holds the buffer, not the `Arc`, and an evicted allocation can be reused.
A region build that misses is not retained, matching the existing source rule.

Pattern fills read `doc`'s pattern library, so they are not retained (same rule
as pattern-driven effect passes); a repeated composite rebuilds them.

### Fills rasterize on the CPU

Solid is a constant RGBA buffer over `layer.rect`; Gradient uses the existing
`gradient_rgba`; Pattern uses `pattern_tile_rgba` with the document's decoded
patterns. A decoded `vscg` vector fill takes the same route: `decode_layer_fill`
already resolves it to a Solid, Gradient, or Pattern adjustment. The buffer is
uploaded as a four-plane source with the layout at `layer.rect`, so the existing
mask/opacity/Fill/blend kernel applies unchanged.

A GPU fill kernel would duplicate the gradient/pattern geometry that the CPU
oracle already defines; the raster is cached, so the CPU cost is paid once per
content change.

A pattern fill is bounded by its fill layer's rect, which can be the whole
canvas; a canvas-sized pattern fill therefore rebuilds a canvas-sized tile on
every composite. Keying the pattern content is the upgrade if a profile shows
it.

### LUT adjustments are built by the CPU code

`pictura-adjust` gains a public `adjustment_lut(&Adjustment)` returning either
three 256-byte channel tables or one 256-entry luma table of `(r, g, b, a)`:

- Levels, Curves, and Exposure are applied to a neutral `0..=255` ramp through
  the existing `apply` path, which composes the per-channel table with the
  composite table exactly as the CPU does.
- Gradient Map uses the existing `gradient_lut` sampled at each luma index and
  carries its alpha, which `apply` discards. The index uses the CPU's Rec.601
  weights (`0.299 R + 0.587 G + 0.114 B`) and the same rounding.

The GPU uploads the table to a new 256-word storage binding and dispatches an
adjustment kind that reads it. Re-deriving Levels/Curves/Exposure math in WGSL is
rejected: the CPU is the oracle and the table is the only way to stay byte-exact
without duplicating the formulas.

### Effect passes are dispatches

`EffectPass { rect, mode, color, alpha }` maps directly onto the existing blend
kernel:

- Below-content passes dispatch before the layer's content; above-content passes
  after it, preserving the CPU order.
- A constant-colour pass and a per-pixel-colour pass are packed into the
  existing 8-bit RGBA source word over the composite region. The CPU keeps the
  pass colour and alpha in `f32`; the packed word quantizes both. The per-family
  parity scenes are the proof, and the upgrade if a family exceeds ±1 LSB is an
  `f32` alpha plane rather than a tolerance change.
- The dispatch uses the pass's blend mode, `opacity = layer.opacity / 255`,
  `fill = 1`, and constant coverage 255: the pass alpha already includes the
  layer mask and the effect's own opacity.
- The dispatch passes `text = false`: the CPU replays passes through
  `blend_parts`, which does not apply the text-gamma curve.

An enabled effect the CPU resolves to a no-op (a bevel in a style the CPU does
not render, a legacy effect that maps to no renderer, or a stroke or overlay
whose payload cannot decode) contributes no pass and does not decline.

The pass groups stay cached by the existing `layer_source` path; a region
composite builds the transient pass it already builds.

### Clipping groups are built with the CPU algorithm and blended on the GPU

The clipped group is composited with the exact CPU sibling-group algorithm over
the requested region (base, coverage, per-clipped-layer lerp), then uploaded as a
packed source and blended onto the running GPU canvas with the base's opacity and
blend mode. This is parity-safe: the group crosses the 8-bit boundary once, the
same way an isolated group already does, instead of once per clipped layer.

`composite_clipping.rs` gains a pure-refactor `build_sibling_group` that returns
the raw group canvas without blending it onto the parent; `composite_siblings`
then calls it and keeps its current behavior. The final GPU blend uses the base's
opacity and blend mode, `fill = 1`, and no layer mask or vector mask; the base's
mask was already applied when it was composited into the group.

A GPU-native lerp kernel would keep the group on the device, but each lerp would
quantize the group and the temp canvas to bytes while the CPU keeps `f32`; two or
more clipped layers can accumulate past the ±1 LSB budget. The CPU-precomposited
group is the correct first implementation; the kernel is the upgrade if a
profile shows the group dominates a document.

### Scope decision

The change implements the issue's phases 1–3 (sources and fills, LUTs, effects,
clipping). It does not implement 16/32-bit, knockout, Blend If, or Dissolve,
which the issue explicitly defers. The motivating document (drop shadow, clipping
mask, Curves, Solid Color fill) needs effects, clipping, a LUT adjustment, and a
fill, so a smaller scope would leave it declining and deliver none of the stated
win. This call is recorded here because the user asked for the biggest win but
did not choose between the full unlocking set and the low-risk step.

### Metrics

`count_decline` keeps its per-composite counters. A new per-layer counter
increments for every declining layer the plan walk sees, so a corpus run ranks
reasons without stopping at the first one. The app metrics JSON gains a
`layer_declines` object next to `declines`; the existing keys are unchanged.
`Counters` gains a `layer_declines` field, an additive change to a public struct;
the public compositor signatures are unchanged.

### Module seams

`gpu/backend.rs` is 991 lines, `gpu/mod.rs` 1157, and `layer_effects/mod.rs`
1192, all under the 1200 cap but with little headroom. The new work lands in
`gpu/plan.rs` (plans), `gpu/raster.rs` (cached raster and fill uploads),
`gpu/effect.rs` (pass uploads), `gpu/clip.rs` (group build), and `gpu/lut.rs`
(tables), so `mod.rs` and `backend.rs` do not grow. `EffectColor` is re-exported
unconditionally from `layer_effects` so `gpu/effect.rs` can name it. New tests
follow the existing `gpu_parity` scenes.

## Risks / Trade-offs

- **Effect-pass parity.** The pass colour and the blurred coverage alpha are
  quantized to the packed 8-bit word. The per-family parity scenes are the
  proof; if a family still diverges, the next lever is an `f32` alpha plane, not
  a tolerance change.
- **Effect-pass and clipping-group uploads are transient.** Their CPU artifacts
  are built per composite (a region pass) or are cheap next to the effect build,
  so they are not retained in the resident store. Cached rasters and fills are
  the content-keyed uploads; this is the ceiling until a profile shows the
  transient uploads matter.
- **Clipping group is CPU-composited.** The group does not get the GPU speedup;
  a document that is one clipped group gains little. The upgrade path is a GPU
  lerp kernel once a profile shows it matters. The rest of the stack still runs
  on the GPU.
- **Resident budget.** Cached raster uploads share the 2 GB store with pixel
  sources. Content keys prevent staleness; FIFO eviction may re-upload a hot
  raster once, which is accepted. A full-canvas proxy-less type layer is the
  worst case (4 bytes/pixel).
- **Region raster misses.** A region present on a raster miss uploads the whole
  cached raster, not the region intersection a pixel source uploads; the raster
  is document-clipped, so the upload is bounded by the layer content, and the
  CPU raster stays cached.
- **Pattern fills are rebuilt** per composite because the pattern library is not
  keyed; the rebuild is bounded by the fill layer's rect, which can be the whole
  canvas. Keying the pattern content is the upgrade.
- **File-size caps.** The module split keeps `backend.rs` and `mod.rs` flat; new
  files are added to the crate normally (Rust modules, no CMake list).
- **Shader and uniform changes.** The adjustment branch gains the LUT kinds, the
  normal branch gains the effect-pass f32-alpha path, and the params uniform
  grows from 80 bytes to carry a constant effect colour. The other branches are
  unchanged.

## Migration Plan

No data or API migration. The change is internal to `pictura-render` and
`pictura-adjust`. Reverting restores the document-wide gate; the CPU oracle and
the public signatures of `composite_gpu`, `composite_active`,
`composite_region_active`, `gpu_available`, and `Backend` are unchanged.
`GpuError` gains no variant.

## Open Questions

- Whether a GPU-native clipping lerp is worth the parity risk is deferred until
  a profile shows the CPU-precomposited group dominates a real document. It does
  not change the specs or the task breakdown.
- Whether the LUT path should extend to Photo Filter, Color Balance,
  Shadows/Highlights, or Channel Mixer is deferred; the same table mechanism
  applies, and the current change deliberately keeps the issue's four kinds.
