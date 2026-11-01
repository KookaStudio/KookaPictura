# Proposal

## Why

`composite_banded` (`crates/pictura-render/src/composite_region.rs:62`) composites
the whole layer stack once per 64-row band. Every band re-derives per-layer state
that cannot vary between bands: the content route and its decoded adjustment
(a Color Lookup re-parses its embedded `.CUBE`), the decoded effect descriptor,
the effect-pass content key (which hashes every extra-block byte and the type
model), the mask gates, the authoritative native planes, and the cache lookup
for every resolved source. A 4000² composite has about 62 bands and a 16k² one
about 250, so the added cost is O(bands × layers × metadata bytes) on every full
or tall-region composite. The layer-source cache module even documents that
"lookups happen once per layer per composite", an assumption the banded path
breaks.

## What Changes

- Add a `composite_plan` module that walks the layer tree once per composite and
  resolves, per layer, into immutable data: the content route and its decoded
  adjustment or LUT, the decoded effect descriptor, the effect passes for the
  composite's region, the resolved source raster (type, smart-filtered, or
  embedded), the mask gates, the native sample planes, and the blend constants.
- Make the whole, banded, region, native-depth, and render-session composites
  execute the same plan. A band performs only the pixel loops.
- Derive every content key from `Plane` stamps and idents, never from plane
  bytes, so no per-band path materializes a fractured plane.
- Resolve the layer's content at most once per composite: `decode_adjustment`
  (and `parse_cube`) and the source-cache lookup run once per layer per
  composite, not once per band.
- Replace the synthetic `Layer` clones that clipping, knockout coverage, and
  adjustment gating build per band with the plan node plus a small blend
  override, so a type or smart-object layer is not deep-copied per band.
- Add an ignored 4000² profile over a stack with Curves, Color Lookup, a
  proxy-less type layer, and a styled layer, recording before and after timings.
- Add test-only counters and a test asserting one adjustment decode and one
  source resolution per layer per banded composite, and a test asserting no
  banded-plane materialization on the band path.

No observable output changes: every composite and saved file stays byte-identical.

## Capabilities

### New Capabilities

- `compositing/composite-plan`: how one composite resolves each layer's route,
  content, effects, sources, masks, and blend constants once, and how every
  composite path executes that plan without repeating resolution work.

### Modified Capabilities

- `compositing/cpu-composite-kernel`: the per-layer state resolution requirement
  changes from once per layer pass (once per band) to once per composite, with
  bands replaying the resolved state.
- `compositing/layer-source-cache`: the banded pass-build requirement changes
  from building the first band to building the passes before the bands execute;
  the cache still builds each whole-layer pass exactly once.

## Impact

- New `crates/pictura-render/src/composite_plan.rs`.
- `crates/pictura-render/src/composite.rs`, `composite_region.rs`,
  `composite_knockout.rs`, `composite_clipping.rs`, `composite_native.rs`,
  `composite_rows.rs`, `composite_blend.rs`, `render_session.rs` — thread the
  plan through the recursive dispatch and remove per-band resolution.
- `crates/pictura-render/src/layer_source/mod.rs`, `layer_source/key.rs`,
  `smart_source.rs`, `smart_filter.rs`, `text_render.rs`, `fill.rs` — resolve
  sources and keys at plan time; keys from idents and stamps.
- `crates/pictura-render/src/gpu/clip.rs` — the CPU clipping-group build used by
  the GPU dispatch also executes a plan.
- `crates/pictura-render/src/mask_refine.rs` — the raster and vector gates become
  `Clone`.
- `crates/pictura-core/src/vector.rs` — `VectorCoverage` gains `Clone, Copy`.
- `crates/pictura-render/tests/composite_profile.rs` and new unit tests.
- No new dependencies. No `docs/` change.
