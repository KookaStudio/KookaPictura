# Design

## Context

See `proposal.md` for motivation and
`specs/compositing/layer-source-cache/spec.md` for the contract.

The compositor is stateless: `composite_rgba(&Document)` and
`composite_rgba_region(...)` take a borrowed document and are called from the
app, the GPU fallback, the oracle tests, and the rasterizer. Layer sources and
effects are rebuilt inside those calls. `pictura-render` already has a
content-keyed cache precedent: `mask_refine` keys a bounded shared cache by
`Plane::ident()`, with builds running outside the lock through
`Arc<OnceLock<_>>`. `Plane::ident()` exists specifically for caches derived from
pixel bytes.

Effect kernels currently assume the canvas is the document: `clip_rect`,
`pad_rect`, and the overlay `coverage` clamp to `0..canvas.w/h`, and the
unaligned gradient/pattern geometry in `overlays` and `strokes` uses
`canvas.w/h` as the document size, while the effect matte reads document
coordinates. Region composites avoid the mismatch by setting `skip_effects` and,
when an effect can reach the region, full-compositing and slicing.

## Goals / Non-Goals

**Goals:**

- Reuse expensive source and effect work across composites without changing a
  single output byte.
- Make effect rendering document-space so a region composite can draw effects
  directly and equal the full-composite slice.
- Keep the cache bounded and observable (purge, hit/miss/bytes/effect-decode
  counters).
- Leave public render entry points unchanged; no app-side cache threading.

**Non-Goals:**

- Hybrid GPU compositing (cached CPU sources uploaded as pixel layers). Tracked
  separately in the issue notes.
- Caching `ScaleContext` or changing GPU text gamma (#976).
- Reworking the morphology kernels; #971 merged here and `layer_effects` already
  calls `pictura_core::mask_ops`.
- Caching any pass whose build reads the document pattern library (pattern
  overlay, pattern-filled stroke, or a matte over pattern-fill content); those
  passes are rebuilt per composite. Document-resource invalidation is not worth
  the complexity in this change; the upgrade path is a pattern-resource digest
  in the key.

## Decisions

### Cache lives in the engine, keyed by content, not by document lifecycle

Chosen: a new `pictura-render::layer_source` module with **one process-wide
`Mutex` map** from a content key to `Arc<OnceLock<Artifact>>`. A lookup takes the
lock, clones the entry's `Arc`, and releases the lock; the build runs in
`get_or_init` outside the lock. There is no per-thread front cache, so no thread
can pin an evicted artifact and the byte budget is enforceable. The map keeps a
default **256 MiB** byte budget and evicts least-recently-used entries; an
artifact larger than the budget is built but not stored. `source_bytes` reports
the shared resident bytes.

Alternatives:

- App-owned `RenderCache` threaded through every render signature: correct
  invalidation through revisions, but changes the public API and hundreds of
  call sites, and buys nothing the content key does not.
- Per-composite memo only: removes duplicate work inside one composite but does
  nothing for a brush dab or a slider tick.
- A per-thread front cache: pins full-rect artifacts after shared eviction and
  cannot honour a fixed global budget; it is also the `mask_refine` pattern that
  would panic on a re-entrant build here.

Content digests (payload, block bytes, type fingerprint) are shareable across
documents. `Plane::ident()` keys are allocation identities: a clone shares the
ident, but an independently loaded document with identical pixels gets different
idents and misses. That is correct, only less sharing. A cache entry is never
invalidated by lifecycle; it simply stops being requested.

### Artifacts are cached whole and sampled per region

Chosen: cache the full artifact (decoded source at source resolution; the
smart-filter chain at `layer.rect ∩ document bounds`; text raster at layer-rect
size; effect passes over reach) and let the caller sample or slice the requested
region. The clipped rect is what the full composite already feeds the chain
(`composite_smart_filtered_source` builds `layer.rect ∩ canvas`); building at the
unclipped layer rect would let a geometric filter read off-canvas source and
change the full composite's output.

The mask mix needs the unfiltered base for masked-out pixels as well as the
filtered chain. Both are sampled at the same clipped rect: the filtered output
is cached, and the unfiltered base comes from the cached decoded source.

Alternative: cache by `(layer, region)`. Every brush dab has a different region,
so it would miss every time.

This departs from the issue's `render_source(layer, region) -> &Planes`: a
reference cannot outlive the cache lock, and region-keyed buffers defeat the
purpose. Accessors return `Arc` handles instead.

### Cache key inputs

| Artifact | Key inputs |
|---|---|
| Decoded embedded source | Payload digest + global text state |
| Filtered smart-filter chain | Payload digest + full `layer.rect` + document width/height + chain hash (`smart_filters_enabled` plus each filter's `enabled`, `filter_id`, `options`) + global text state |
| Type raster | `TypeTool` debug fingerprint + rect + font-registry generation + missing-glyph flag |
| Decoded layer effects | Authoritative block tag + block bytes digest |
| Effect passes | Content key + effect block digest + rect + document width/height/mode |
| Content key (reused by effect passes) | Alpha plane ident; raster/vector mask idents plus rect, default colour, density, feather, disabled, invert; fill block bytes; payload digest / type fingerprint; rect; global text state |

No document pattern resources appear because any pass that reads them is not
cached (see Non-Goals), and a stack with such a pass is excluded from banding
so it cannot rebuild once per band.

### Smart-object payload identity is a digest stored on `SmartObject`

Chosen: add `payload_digest: u64` to `SmartObject`, set by a `set_payload`
helper at the five write sites (codec read, convert, place, replace, CRS patch),
and implement `PartialEq` manually to exclude it from equality. The accessor
`payload_digest()` returns the stored value when non-zero and otherwise hashes
`payload` bytes; the fallback only runs for a literal-built object, so it cannot
collide at digest 0 and does not pay the hash on the hot path.

Alternatives:

- Hash the payload on every composite: 20–50 ms for a large embedded PSD, which
  is exactly the dab hot path. The stored field is the fast path; the zero-value
  fallback is only a safety net for struct literals.
- Use `uuid`: cleared on replace and unchanged by a CRS patch, so it is neither
  stable nor content-sensitive.
- Pointer-identity memo: a freed and reallocated `Vec` can reuse the address,
  which risks a stale hit.

The digest is deliberately ignorable by equality, so only rendering checks catch
a missed write site. The replace and CRS-patch tests cover the update path; the
zero-digest fallback covers objects built by a literal, so those cannot collide
even if a writer is missed.

### Type identity is a fingerprint, not a new field

Chosen: hash the `TypeTool` `Debug` representation plus the font-registry
generation and `missing_glyph_protection()`. The debug string automatically
covers nested style, path, warp, and future fields; text models are small, and
the cost is negligible next to shaping. Mark it `ponytail:` in code.

Alternative: a hand-written hash over every field. More code and a permanent
maintenance trap when a field is added.

### Effect passes are `(mode, colour, alpha)`, not premultiplied RGBA

Chosen: each effect builds one or more passes in document space:

```text
EffectPass {
    rect,
    mode,
    color: EffectColor,   // Constant([f32; 3]) | PerPixel(PixelBuffer)
    alpha: Vec<f32>,      // per-pixel coverage over `rect`
}
```

Replay is a loop over `rect ∩ canvas` calling the existing
`blend_parts(canvas, x, y, colour, alpha, mode)`. `effect_scale` (layer opacity)
remains applied at replay time.

Why not premultiplied planes: the effect blend modes are non-separable with the
backdrop, and `blend_parts` is the exact existing operation. Storing straight
colour plus per-pixel alpha lets reuse reproduce the uncached call bit-for-bit,
including noise (baked at build), the layer mask, and dissolve.

Bevel emits **up to four** passes: `lit()` returns two `(coverage, sign)` slots
per pixel, and each slot emits a shadow and a highlight; the `Pillow` style uses
both slots and swaps which shading array is the highlight. Applying each pass
over the whole rect preserves the per-pixel order because `blend_parts` writes
only one pixel.

### Effects become document-space; reach becomes total

Chosen: build and clip against document bounds rather than `0..canvas.w/h` —
the clip rect, the padded matte box, the overlay/coverage rect, and the
unaligned gradient/pattern geometry in `overlays`/`strokes` that currently
treats `canvas.w/h` as the document size. Make `effects_reach` return a total
`PsdRect` (a fill or inverted mask is still bounded by the layer rect). This lets
`composite_rgba_region` drop the full-composite fallback and its own use of
`skip_effects`, and is what makes the region contract true. `skip_effects` stays
as an internal no-effects flag for `composite_clipping::coverage` and
`knockout_base`, which must exclude effects for reasons unrelated to the region
fallback (rename the field if clearer).

`effects_reach` must include the bevel's real padding:
`size + blur_support(rounding(technique, size)) + blur_support(soften)`; the
`size + soften` approximation under-bounds `Smooth` outer/emboss/pillow bevels.

This also fixes a latent divergence: a Lens Correction smart filter applied to a
region buffer is not the same as the same slice of the full composite. Caching
the chain output at `layer.rect ∩ document bounds` makes region output the true
slice without changing the full composite.

### Effects decode once per layer per composite

Chosen: cache the decoded `LayerEffects` by the authoritative block tag and bytes
and return `Arc<LayerEffects>`; `composite_layer_inner` resolves it once and
passes it to the below/above passes and the reach calculation. A process-wide
`effect_decodes` counter records builds so the "one decode" requirement has a
test hook.

The block hash is small; the expensive part is descriptor parsing, and the cache
also avoids it across composites.

### Static sources stay where they are; only the I/O moves

`render_smart_source`, `apply_smart_filter_chain`, and `render_text_buffer` keep
their sampling, filtering, and shaping logic. The cache wraps their inputs or
outputs. This keeps the diff reviewable and the byte-identity argument local.

### Banding widens only after the byte-identity corpus is green

Chosen: change `banded` to `rows > 2 * BAND` once sources are cached, because the
historical exclusions existed only to avoid re-rendering object layers per band.
Validate banded against whole on a corpus with smart, filter, text, and effect
layers; this adds `[bc_objects]` to `compositing/banded-compositing`. This is the
last task and can be dropped without affecting the other contracts (drop its
delta with it).

### Layer Style preview uses the existing region refresh

Chosen: capture `layer_paint_bounds` before and after mutating the style, take
the union, and call `refresh_region` (no history capture) while keeping the
content revision bump. `layer_paint_bounds` is:

- pixel, smart-object, type, fill: `layer.rect ∪ effects_reach`;
- group: the union of its descendants' paint bounds (a group's own rect is often
  `0,0,0,0`);
- adjustment layer: the document canvas, or its mask bounds when they are
  bounded (it affects every pixel below it);
- missing path: `None`.

Because the preview now emits only `regionDamaged`, and recovery marks a
document stale only on `changed`, the app also connects
`PictureView::regionDamaged` to the recovery-stale set (or emits a notify on the
`Edit::Record` commit) and covers it with `tst_recovery`.

## Risks / Trade-offs

- **Effect-pass replay diverges by one LSB** → keep the exact `blend_parts` call,
  store per-pixel alpha as `f32`, preserve pass order, and test every effect
  cold vs warm.
- **Coordinate rewrite changes off-canvas behaviour** → add offset-canvas tests;
  clip pass geometry to the document exactly as the current code clips to the
  canvas, including the document-size uses in `overlays`/`strokes`.
- **A missed payload-digest write site serves stale pixels** → the accessor
  hashes a zero-stored digest, so literal-built objects cannot collide; the
  replace and CRS-patch tests cover the update path. The digest is ignored by
  equality, so only rendering checks catch a writer bug.
- **A self-referential embedded payload** → the existing source path already
  recurses without a depth guard (`smart_source.rs` has a `ponytail:` note), and
  the cache changes the failure mode: a cyclic payload can re-enter the same
  `OnceLock::get_or_init`. Keep the note and add a depth pass-through when
  untrusted embeddings appear.
- **Memory growth from full-rect buffers and effect passes** → one shared
  256 MiB budget with LRU eviction; an artifact larger than the budget is built
  but not stored; `source_bytes` counts shared resident bytes. The 4000² profile's
  ~64 MiB filtered buffer plus ~64 MiB text raster fit.
- **Pattern passes are rebuilt every composite** → pattern overlay, pattern
  stroke, and pattern-fill mattes are excluded from the pass cache (the profile
  does not exercise them).
- **Shared cache and parallel tests** → hit-count tests serialize on a test
  mutex or use unique content and `purge`.
- **Lens Correction region output changes** → intended bug fix; add the
  region-equals-full test before enabling the cache so the divergence is visible.
- **File-size caps** → move `composite_rgba_region`/`banded`/`composite_banded`
  into `composite_region.rs` as a pure move before editing, and delete
  `slice_region` when its last caller goes; put the effect cold/warm matrix in
  `src/tests/layer_effects/cache.rs`, not the already-large
  `layer_effects.rs`. `composite.rs` is already near the 1200-line cap.
- **Cache correctness under history clones** → `Plane::ident()` is copied by a
  clone and cleared by every mutable access; cloned planes deliberately hit the
  same entry.

## Migration Plan

No persistent data migration. The cache is process-local, so a rollback is a
revert of the crate changes; purging at runtime is always safe. The app may call
`purge_source_cache()` on memory pressure. The metrics report gains additive
fields (`source_hits`, `source_misses`, `source_bytes`, `effect_decodes`);
consumers that parse it keep working.

## Open Questions

None.
