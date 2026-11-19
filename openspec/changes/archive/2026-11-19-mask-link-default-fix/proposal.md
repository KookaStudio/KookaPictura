# Proposal

## Why

A new fill or adjustment layer created with the default-mask preference and no
active selection receives an **unlinked** mask, so the Layers-panel link glyph
is dimmed from the moment the layer appears. `default_layer_mask` still writes
`flags: 1`, the pre-#1200 spelling of "linked"; under the corrected PSD
polarity bit 0 means unlinked, so the prefill contradicts the existing
`Raster mask link flag polarity` requirement.

## What Changes

- Stop writing `flags: 1` in `default_layer_mask`'s reveal-all branch; the mask
  takes `flags: 0`, the same linked default as the selection branch and
  `add_layer_mask`.
- Make the polarity requirement name the default fill/adjustment mask
  explicitly, so the prefill cannot drift from the mask-command path again.
- Add a regression test over both `default_layer_mask` branches.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `compositing/layer-masks`: the `Raster mask link flag polarity` requirement
  names the default mask a new fill or adjustment layer receives among the
  creation paths that must start linked, with a scenario for that path.

## Impact

- `crates/pictura-app/src/cxxqt_object/helpers_composite.rs`:
  `default_layer_mask` and its unit test. Callers (fill layers, adjustment
  layers, the rasterize path) inherit the fix.
- No codec, render, UI, or serialization change: the flag byte is already
  written and read with the corrected polarity.
