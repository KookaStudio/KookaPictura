# Fix Quick Mask editing regressions

## Why

The mask currently shares the active pixel-selection slot. Creating another selection therefore replaces the mask, cached marching ants survive entry, and metadata-only undo can leave the old overlay cached. Every dab also emits a full shell refresh and renders its tint through per-pixel Qt calls.

## What Changes

- Separate Quick Mask coverage from the temporary editing selection; entry consumes the original selection into mask coverage and exit discards the editing constraint.
- Preserve and restore both planes through history, while keeping the mode itself outside history.
- Invalidate selection contours and mask presentation on mode transitions and history restores.
- Present brush damage through the existing bounded/deferred path, blend native RGBA crop bytes, and keep coverage planes shared in bands.

## Capabilities

### Modified Capabilities

- `tools/quick-mask`: independent temporary selections, visible undo, retained mask coverage and bounded painting presentation.

## Impact

Quick Mask runtime/history metadata, canvas crops, brush clipping, fills/filters, and Qt/Rust regression checks. No dependencies or serialized document fields are added.
