# Proposal

## Why

The compositor and PSD persistence support three advanced blending flags, but the Layer Style dialog does not expose them.

## What Changes

- Expose Transparency Shapes Layer, Layer Mask Hides Effects, and Vector Mask Hides Effects in Advanced Blending.
- Bind them to the existing layer flags and preserve the dialog's live preview, cancel, and undo behavior.
- Cover control values and PSD round trips. Blend If remains in #264.

## Capabilities

### New Capabilities

- `ui/advanced-blending-controls`: Layer Style controls for the existing advanced blending flags.

### Modified Capabilities

## Impact

Layer Style dialog and layer-style value bridge; Qt and Rust regression tests. No dependencies.
