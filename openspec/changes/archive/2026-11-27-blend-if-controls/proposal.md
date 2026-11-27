# Proposal

## Why

Layers already store Blend If ranges, but users cannot edit them in Blending Options. The layer badge is a text placeholder.

## What Changes

- Add Gray ranges for RGB and grayscale documents and RGB color-channel ranges for RGB documents, with split handles and live preview. Other document modes and pass-through groups show unavailable controls and keep their ranges untouched.
- Preserve other channels and PSD ranges across edits, cancellation, undo and save. An unparseable raw block is retained until the user explicitly chooses to replace it.
- Render split ranges as gradual transitions and replace the placeholder badge with an original asset.

## Capabilities

### New Capabilities

- `ui/blend-if-controls`: Edit and preview layer blending ranges.

### Modified Capabilities

- `ui/layers-panel`: Replace the Blend If text chip with an icon while retaining its layout and raw-range behavior.

## Impact

Layer Style dialog, bridge, CPU compositing and cache invalidation, Layers panel badge, PSD range handling and Qt/Rust regression coverage. No new dependencies.
