# Proposal

## Why

The Eyedropper ignores modifiers: it cannot set the background colour or place a colour sampler. Temporary sampling in paint handlers does not share the Eyedropper's sampling settings or continuous drag behavior.

## What Changes

- Alt with the Eyedropper samples the background colour; Shift-click places a Color Sampler.
- Temporary foreground sampling uses Eyedropper settings for Brush, Pencil, Color Replacement, Pattern Stamp, History Brush, and Art History Brush, including in Quick Mask without changing the mask. Preserve existing Alt gestures for Clone Stamp, Healing Brush, Eraser, and Mixer Brush.
- Add canvas interaction coverage for modifiers, sampling, restoration, sampler limits and existing gestures.

## Capabilities

### New Capabilities

- `tools/eyedropper-modifiers`: Eyedropper modifier gestures and temporary sampling from painting tools.

### Modified Capabilities

- `tools/color-sampler`: Placement from the Eyedropper.

## Impact

Canvas tool routing, Eyedropper and paint handlers, foreground/background colour state, annotation placement, and Qt Test coverage. No new dependencies are anticipated.
