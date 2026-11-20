# Proposal

## Why

Free Transform (scale, rotate, skew, distort, perspective) resamples each layer
channel on its own, so the colour of a transparent pixel is averaged into its
visible neighbours. An erased pixel is alpha 0 with colour black (Clear mode),
so resizing a layer with erased areas draws a dark line along every erased edge,
and along the layer's own edge, where the kernel reads transparent padding
(#1446). The warp engine already resamples colour premultiplied by alpha; the
transform family does not.

## What Changes

- The transform resampler weights each colour tap by its alpha: a colour channel
  of a layer with an alpha channel resamples as `Σ wᵢ·aᵢ·cᵢ / Σ wᵢ·aᵢ`, so a
  fully transparent pixel contributes no colour. Where every contributing tap is
  opaque, the result is the existing kernel's value, byte for byte.
- The same rule applies to the layer's retained 16-bit and 32-bit store, so its
  narrowing still matches the 8-bit channel.
- The alpha channel itself, the layer mask, and the selection-only transform are
  unchanged.
- A transform or warp no longer resamples a stale retained store: a brush or
  eraser stroke edits only the 8-bit planes, and the transform narrowed the old
  store back over them, so erased pixels reappeared after a resize on a layer
  with a 16- or 32-bit store (#1446). Each stale store plane is refreshed from
  its 8-bit plane first.
- Auto-Align reads layer luma premultiplied by alpha. Its matcher had relied on
  the dark fringe to soften a transformed layer's anti-aliased edge; with the
  colour kept, the edge stepped to zero and its stair-steps read as corners.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `document/transform-interpolation`: adds the alpha-weighted colour
  requirement and the stale-store requirement.
- `document/auto-align-layers`: adds the premultiplied-luma requirement.

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/kernel.rs`: an
  alpha-weighted sampler.
- `transform_tiled.rs`, `transform.rs`, `transform_native.rs`: colour planes
  carry the layer's alpha plane into the resampler; `prepare_layer` refreshes
  stale store planes (`transform_native::current_store`).
- No dependency, no file-format change. An opaque layer resamples exactly as
  before.
