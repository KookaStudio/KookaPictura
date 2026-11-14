# Proposal

## Why

Photoshop stores some shape layers' vector masks under the alternate `'vsms'`
(`VECTOR_MASK_SETTING2`) key instead of `'vmsk'`. The codec reads only `'vmsk'`,
so such a layer loses its mask: a `'vscg'` shape fill paints its whole rectangle
instead of its outline (#1114), and a layer clipped to it is confined to that
rectangle instead of the shape (#1284). A reported reference document shows both symptoms.
Decoding the block also exposes the layer's vector-mask authoring controls, so
those controls and the layer's shape identity must work for `'vsms'` too.

## What Changes

- Read a layer's vector mask from `'vmsk'`, falling back to `'vsms'` only when
  `'vmsk'` is absent; `'vmsk'` wins even when malformed. Both keys carry the
  same version-3 body, so the existing decoder and derived `Layer.vector_mask`
  view are unchanged otherwise.
- Make vector-mask authoring use the layer's effective block: enable/disable,
  link/unlink, delete, rasterize, and block sync patch or remove `'vsms'` when
  it is the effective key, and delete/rasterize remove every vector-mask block
  so the mask cannot return on reload.
- Recognize a `'vsms'` fill shape as a shape layer: outline reads, shape row
  identity and forced locks, shape path edits, and Rasterize Shape include the
  `'vsms'` key.
- Add a generated `'vsms'` fixture and coverage in the codec oracle, render,
  authoring, and shape-rasterization tests.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `codec/psd-layer-io`: the vector-mask decode requirement covers `'vsms'` as
  well as `'vmsk'`, with precedence, malformed-block behavior, and round-trip
  preservation.
- `compositing/layer-compositing`: a mask derived from `'vsms'` clips a `'vscg'`
  shape fill exactly like a `'vmsk'` mask.
- `compositing/clipping-masks`: a clipping base whose vector mask is derived
  from `'vsms'` confines its clipped run to the path alpha, not the base rect.
- `compositing/vector-masks`: flag, delete, rasterize, and read operations use
  the effective `'vmsk'`/`'vsms'` block.
- `compositing/layer-management`: a `'vsms'` fill shape is a shape layer and
  Rasterize Shape applies to it.

## Impact

- `crates/pictura-codec/src/vector_mask.rs`: block-key resolution.
- `crates/pictura-render/src/document_ops/layer_ops/vector_masks.rs`,
  `shape_layer.rs`, `rasterize.rs`, and `document_ops/rotate_in.rs`: effective
  block for authoring, shape identity, and block sync.
- `scripts/generate-fixtures.py` and
  `crates/pictura-codec/tests/fixtures/`: a small `'vsms'` fixture.
- `crates/pictura-codec/tests/vector_mask_oracle.rs` and the codec/render unit
  tests: decode, precedence, base rendering, clipping coverage, authoring, and
  shape rasterization.
- Closes #1284 and #1114. No dependency or API changes; both keys were already
  preserved verbatim on save.
