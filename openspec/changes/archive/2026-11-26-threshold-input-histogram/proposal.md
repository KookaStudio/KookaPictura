# Proposal

## Why

The Threshold slider gives no view of how the input's dark and light pixels are distributed. Issue #563 adds the luminance histogram the user needs to choose the cutoff.

## What Changes

- Show a single luminance histogram between the Threshold value row and slider in the Image Adjustments dialog and in adjustment-layer Properties.
- Use the dialog's opening edit target and selection, and the layer editor's source beneath the adjustment. Slider and numeric-field edits retain the existing preview and undo behavior.

## Capabilities

### New Capabilities

- `ui/threshold-histogram` displays the luminance distribution used to choose a Threshold Level.

### Modified Capabilities

None.

## Impact

The shared Qt adjustment controls and their two hosts display a read-only distribution provided by the document bridge. Qt and Rust regression coverage exercises those surfaces and source rules.
