# Proposal

## Why

Review of #583 with the real horse and lily photographs beside Photoshop. Stack
Images kept the flower's out-of-focus green background, showing its rectangle, and
Seamless took tone from the wrong place: Photoshop's petals take on what lies
beneath them, and its sky shifts in colour but not brightness.

## What Changes

- Flat and out-of-focus areas stay with the lowest layer; only blown or crushed
  areas lose on exposure.
- An upper layer with real detail wins unless clearly less sharp, so fine texture
  such as spray does not erase its subject.
- With Seamless on, Stack Images takes broad tone from the lowest covering layer
  and detail from the sharpest.
- Seamless gains are measured over flat areas. Stack Images keeps brightness and
  corrects colour; Panorama still matches exposure.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/auto-blend-layers`: stack selection and tone.

## Impact

`crates/pictura-render/src/document_ops/layer_ops/auto_blend/{stack,gains,blend,mod}.rs`.
The constants are tuned by eye against one Photoshop result, not derived.
