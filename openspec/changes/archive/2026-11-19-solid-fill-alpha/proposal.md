# Proposal

## Why

`add_solid_fill(rgba: [u8; 4])` silently discards the color's alpha: it always
hands three components to `encode_solid_color_fill`, so `Solid([10, 20, 30, 40])`
is created and rendered as `[10, 20, 30, 255]` (#805). The render model, the
edit path, and the save path already carry alpha; only the create paths bypass
the existing encode rule, which makes the API lie about its parameter.

## What Changes

- Expose `pictura_codec`'s existing fill-encode rule (`encoded_fill`) as the one
  seam both create paths use: opaque fills keep the standard version-16 `SoCo`
  descriptor, non-opaque fills use the native 4-byte `SoCo` that the decoder,
  the compositor, and the writer already support.
- `add_solid_fill` and the app's `fill_layer` create path route through that
  seam instead of `encode_solid_color_fill` directly, so the requested alpha
  survives creation, compositing, `Rasterize Fill Content`, and save/reopen.
- Keep the app's 6-digit payload and the opaque shape-fill argument unchanged;
  no UI can request a translucent color yet, so this fixes the engine contract
  rather than adding a visible control.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `imaging/adjustment-layer-rendering`: the authored solid fill descriptor
  requirement gains the non-opaque form (the native 4-byte `SoCo`), and the
  encoder rule is stated once for both opaque and non-opaque fills.

## Impact

- `crates/pictura-codec/src/stage_content.rs` and `crates/pictura-codec/src/lib.rs`:
  the encode rule becomes public.
- `crates/pictura-render/src/document_ops/layer_ops/create.rs` and
  `crates/pictura-app/src/cxxqt_object/impl_layers/fill_content/imp.rs`: both
  create paths use it.
- Render and codec tests: create-at-alpha, composite, and save/reopen.
- No dependency, no wire-format invention, no behaviour change for opaque fills.
