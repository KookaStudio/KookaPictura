# Proposal

## Why

Review of the Crop options bar (#104): the guide-overlay (grid) menu's icons
sat on the menu edge, and its entries should be picked by clicking them, like
CS6's, not ticked as checkboxes.

## What Changes

- The grid menu lists plain, non-checkable items: the overlays (each with its
  icon), the show modes, and the cycle keys in the right-hand column. The icons
  start at the Settings menu's checkbox inset.
- The button icon follows the overlay however it changes (the menu or the `O`
  key).
- **BREAKING**: none.

## Capabilities

### Modified Capabilities
- `tools/crop-overlay-feedback`: the overlay menu requirement.
