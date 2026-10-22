# Proposal: Compositor, layer and native colour bugs

## Why

The audit found compositor paths that drop masks, opacity and clipping, under-bound layer effects, and a profile conversion that transforms only the 8-bit working copy.

## What changes

- #846 — a Pass Through group applies its vector mask by falling back to the
  isolated path; the mask is no longer ignored because only the raster mask was
  checked.
- #847 — layer opacity scales content and styles (effects included); adjustment
  layers ignore Fill while fill-content layers keep it.
- #848 — effect reach includes Spread; channel-less smart objects use their
  rendered source alpha for effect mattes, with the opaque-rect fallback kept
  for an unreadable embedded source.
- #849 — Merge Down composites a clipped target and source without letting the
  target become its own base, preserves the target's clip flag, and drops the
  stale merge-local clipping fold now that the compositor has a clipping pass.
- #861 — Convert to Profile transforms native-depth planes and layer channels at
  their stored depth, including a 32-bit float path, instead of only the 8-bit
  copy.

## Capabilities

### Modified Capabilities

- `compositing/layer-compositing`: pass-through masks include vector masks; Fill
  is ignored by adjustment layers and still honoured by raster and fill-content
  layers.
- `compositing/layer-effects`: layer opacity scales below- and above-content
  effects; effect reach includes Spread and effect mattes use real source alpha.
- `compositing/layer-management`: Merge Down composes through the clip-aware
  scratch without making a clipped target its own base, keeps the target's clip
  flag, and applies the base coverage once in Merge Clipping Mask.
- `color/color-management`: profile conversion supports 32-bit float samples in
  addition to 8- and 16-bit samples.
- `codec/psd-icc-conversion`: Convert to Profile transforms the retained native
  planes and layer channels and the saved file re-emits them.

## Impact

`crates/pictura-render/src/composite.rs`, `composite_native.rs`,
`smart_source.rs`, `gpu/backend.rs`, `layer_effects/`,
`document_ops/layer_ops/merge.rs`, `crates/pictura-color/src/lib.rs`,
`crates/pictura-codec/src/icc.rs`, and `crates/pictura-codec/src/icc_convert.rs`.

## Non-goals

The Layers panel still offers its Fill field for an adjustment layer; the
renderer ignores it. Disabling that control needs a new row capability and is a
separate UI change.

## Issues

Closes #846, #847, #848, #849, #861.
