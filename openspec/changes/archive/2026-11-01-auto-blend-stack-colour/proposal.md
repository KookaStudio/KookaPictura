# Proposal

## Why

Comparing the horse and lily result with Photoshop at matching scale: Photoshop's
petals stay pink, magenta over the dark horse and pale over bright water, while
ours took all colour channels of broad tone from the base and turned grey-blue.
A green halo also surrounded the subject where the focus map crossed its edges.

## What Changes

- The merged blend works on luma and colour differences. With Seamless on, a stack
  takes only broad brightness from the base; colour stays with the chosen layer.
- Focus maps are smoothed with a guided filter, so they stop at subject edges.
- Flat areas need more evidence before leaving the base layer, and focus saturates
  so the blur tails of a strong edge cannot select the background past it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/auto-blend-layers`: stack colour and focus edges.

## Impact

`crates/pictura-render/src/document_ops/layer_ops/auto_blend/{blend,stack}.rs`.
Constants are tuned by eye against one Photoshop result.
