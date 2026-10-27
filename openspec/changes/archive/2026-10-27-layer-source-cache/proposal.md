# Proposal

## Why

Smart-filtered layers, proxy-less smart objects, proxy-less type layers, and
layer effects are rebuilt on every composite. A single brush dab that touches a
styled layer's reach forces a full-document CPU composite and a serial slice,
costing roughly 0.5–1.5 s on a 24 MP document where a few milliseconds should
suffice. Layer Style slider ticks pay the same cost on every tick.

## What Changes

- Add a content-keyed source cache in `pictura-render` that holds, per layer:
  the decoded embedded smart-object source, the smart-filter chain output, the
  type-layer raster, the decoded effects, and the document-space effect passes.
  The cache is one process-wide map with a default 256 MiB budget; an artifact
  larger than the budget is built but not stored.
- Make layer effects document-space and replayable: build each effect once into
  `(blend mode, colour, alpha)` passes over its reach, then blend the passes into
  any canvas rectangle. This lets region composites draw effects instead of
  full-compositing and slicing.
- Decode a layer's effects descriptor once per layer per composite instead of
  twice per layer.
- Widen banded compositing to stacks with smart objects, type, and effects now
  that their sources are cached.
- Bound the Layer Style preview to the layer paint bounds before and after the
  change instead of a full recomposite per slider tick; groups use their
  descendants' union and adjustment layers use the canvas or their bounded mask.
- Add cache hit/miss/byte counters and an effects-decode counter to the metrics
  report, a purge entry point, and an ignored benchmark that measures cold vs
  warm region and full composites on a 4000² document.
- Fix the existing region/full divergence for geometric smart filters (a Lens
  Correction chain applied to a region-sized buffer is not the same as the same
  slice of the full composite). The cache renders the chain once at the layer
  rect clipped to the document bounds, so region output becomes the slice of
  full output without changing that full output.
- Rebuild, rather than cache, any effect pass whose build reads the document
  pattern library (pattern overlay, pattern-filled stroke, and a matte over
  pattern-fill content).
- Mark a document recovery-stale on `regionDamaged`, because the Layer Style
  preview no longer emits `changed` on every slider tick.

Composite output matches the uncached path byte-for-byte; the only intended
changes are the smart-filter region fix above (region output becomes the true
full-composite slice) and rasterizing a styled object, which now bakes its
effect reach (previously clipped). The existing oracle and effect tests remain
the contract.

## Capabilities

### New Capabilities

- `compositing/layer-source-cache`: content-keyed caching of per-layer render
  sources and effect passes, byte-identical reuse, bounded memory with purge and
  counters, region composites that draw cached effects, and a bounded Layer
  Style preview.

### Modified Capabilities

- `compositing/described-damage`: a layer whose content box and effect reach are
  determinable becomes a bounded operation for Layer Style previews and region
  composites; the effect-block full-recomposite rule is removed.
- `compositing/banded-compositing`: band eligibility widens from per-pixel stacks
  to cached source layers (smart objects, smart filters, type, effects).

## Impact

- `crates/pictura-core`: `SmartObject` gains a payload digest used only as a
  cache identity (excluded from equality). Its accessor returns the stored value
  when non-zero and hashes the payload when it is zero, so a literal-built object
  cannot collide with another at digest 0.
- `crates/pictura-codec`: sets the digest when reading or patching a smart-object
  payload.
- `crates/pictura-render`: new `layer_source` module; new effect `EffectPass`
  build/apply seam; effects become document-space; region compositing no longer
  falls back to full composite for bounded effects; banding eligibility widens;
  sets the payload digest for convert/place/replace; metrics (`source_hits`,
  `source_misses`, `source_bytes`, `effect_decodes`) and a benchmark test.
- `crates/pictura-app`: Layer Style preview uses a region refresh over the paint
  bounds union; metrics JSON includes the source-cache and effect-decode
  counters; recovery marks a document stale on `regionDamaged`.
- No new dependencies. No GPU behaviour change: hybrid GPU upload of cached
  sources stays out of scope.
