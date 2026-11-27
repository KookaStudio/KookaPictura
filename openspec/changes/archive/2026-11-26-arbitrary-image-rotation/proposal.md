## Why

Issue #567 tracks the gap between the documented Rotate Canvas workflow and the single-angle prompt with fixed bilinear sampling. Users should be able to choose a direction, level a measured reference line, and control interpolation independently of the angle.

## What Changes

- Add the Rotate Canvas angle and direction controls and prefill the correction from the active document's ruler measurement.
- Persist Image Interpolation in General preferences and use it for interactive arbitrary canvas rotation.
- Record the resolved preference ID with new interactive Actions steps. Persist recorded IDs through Pictura Actions files; existing angle-only commands continue to use bilinear at non-right angles. Preserve exact centre quarter turns even with vector masks.

## Capabilities

### New Capabilities

### Modified Capabilities

- `document/image-orientation` adds the dialog, ruler prefill, and preference-driven interpolation to arbitrary image rotation.

## Impact

Qt Image menu, General preferences, whole-document rotation bridge and tiled renderer, command recording, and Rust and Qt test sources.
