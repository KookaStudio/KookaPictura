## Why

Roadmap P3: a CS6 shape layer stores its fill in the per-layer `vscg`
additional-layer-information block (a 4-byte fill key followed by a version-16
fill descriptor), and its outline in `vmsk`. The codec already preserves `vscg`
opaquely in `Layer.extra_blocks`, and the just-shipped vector mask renders the
outline as a clip, but nothing decodes the fill, so a shape layer's own fill
never renders. The nested descriptor is the same shape as the `SoCo`/`GdFl`/`PtFl`
fill blocks the engine already decodes and composites, so this is a small
extension of shipped machinery rather than a new fill model.

## What Changes

- **`pictura-render` decodes `vscg` into the existing typed fill content.** A new
  `decode_vector_fill(&[u8]) -> Option<Adjustment>` parses the preserved block
  (`[4-byte fill key][version-16 descriptor]`) and dispatches on the descriptor's
  content — `Grad` gradient, `Ptrn` pattern, `Clr ` solid — exactly as ag-psd's
  `parseVectorContent` does. It reuses the existing `decode_gradient_fill` /
  `decode_pattern_fill` / `decode_solid_fill` decoders unchanged; a malformed or
  unknown descriptor is `None`, never a panic. No new `Adjustment` variant.
- **One layer-fill helper routes the three render paths through one decision.** A
  new `decode_layer_fill(&Layer) -> Option<Adjustment>` returns the layer's
  modeled adjustment block first (unchanged behavior), else its `vscg` vector
  fill. The compositor layer dispatch and the layer-effects fill-coverage matte
  both use it, so a shape layer's fill composites and its effects are gated by
  the same content. The preserved `vscg` bytes are never rewritten.
- **A vector fill is clipped by the layer's vector mask.** A `vscg` fill
  composites through the shipped generative fill path
  (`composite_solid_fill`/`composite_gradient_fill`/`composite_pattern_fill`), so
  the layer's mask, opacity, fill, and blend apply and the already-shipped `vmsk`
  coverage (`mask_alpha`) clips it. With no vector mask it fills the layer rect.
- **The GPU falls back to the CPU for a vector fill.** `check_supported` rejects a
  visible layer carrying a `vscg` block (there is no GPU fill shader, exactly as
  for `SoCo`/`GdFl`/`PtFl`), so parity holds without a shader.
- **Fixture + oracles + render test.** A committed `vector_fill.psd` carries a
  `vscg` solid-fill shape layer clipped by a rectangle `vmsk`; psd-tools and
  ag-psd read it as independent structural oracles, and a render test proves the
  fill appears inside the mask and not outside.
- **No app change, no `CMakeLists.txt` change, no `docs/` change, no new
  dependency.** The app renders through the existing composite path; no
  bridge/CPP/self-test code is needed.
- **BREAKING**: none. A `vscg` that previously no-op'd now renders; the block is
  still preserved byte-for-byte.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `psd-layer-io`: adds a committed `vscg` fixture requirement with independent
  psd-tools and ag-psd structural oracles, and asserts the block stays
  byte-preserved through a whole-`Document` round trip.
- `layer-compositing`: adds the `vscg` decode into typed fill content and the
  requirement that a shape layer's vector fill composites and is clipped by the
  vector mask, with the CPU/GPU parity fallback.

## Impact

- `crates/pictura-render/src/fill.rs`: `decode_vector_fill`, `decode_layer_fill`;
  reuse the existing fill decoders/params.
- `crates/pictura-render/src/composite.rs`: `decode_solid_fill` becomes
  `pub(crate)`; `composite_layer`'s dispatch uses `decode_layer_fill`.
- `crates/pictura-render/src/fill.rs`: `fill_coverage_matte` uses
  `decode_layer_fill`, so layer effects are gated by the vector fill.
- `crates/pictura-render/src/gpu/mod.rs`: `check_supported` rejects a layer
  carrying `vscg` so the composite falls back to the CPU.
- `crates/pictura-render/src/tests/vector_fill.rs` (new) and
  `crates/pictura-render/src/tests/mod.rs`: decode, clip, precedence, malformed,
  and no-mask tests.
- `scripts/generate-fixtures.py`,
  `crates/pictura-codec/tests/fixtures/vector_fill.psd` (new), and
  `.../tests/fixtures/README.md`: the `vector_fill()` builder and the fixture.
- `crates/pictura-codec/tests/psd/vector_fill_oracle.rs` (new): the psd-tools oracle
  (`oracle.rs` is 1398/1400 lines and cannot be extended).
- `crates/pictura-codec/tests/psd/agpsd_oracle.rs`: a `vscg` `vectorFill` test.
- `crates/pictura-app`, `CMakeLists.txt`, `docs/`: unchanged.

## Out of scope (deferred)

- **The vector stroke** (`vstk`, the `StrokeDescriptor`, and its
  `strokeStyleContent`), including stroke width/caps/joins/alignment/dashes,
  gradient/pattern stroke content, and stroke rendering. Decoded only if trivial;
  otherwise deferred.
- **`vogk` vector origination** (live-shape properties) and `vsms`.
- **Noise gradients, multi-subpath boolean ops, text, and antialiasing** stay as
  in the shipped fill/vector-mask slices.
- **Rasterizing a `vscg` shape layer** (`Rasterize Fill Content`) is not enabled;
  the fill-content predicate stays adjustment-block-based.
- **App UI, authoring, and editing of a shape layer's `vscg`** — render-only.
