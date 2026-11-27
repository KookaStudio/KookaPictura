## Why

A zoomed-in canvas can require repeated drags to reach another area. Issue #534 adds a temporary overview and momentum for Hand navigation.

## What Changes

- Holding an accepted plain H Hand shortcut and dragging on the canvas shows a fit overview with a destination outline. Releasing the mouse restores the previous zoom at the chosen location.
- A three-state Flick Panning preference in General controls momentum after a fast Hand drag on a GPU-capable view: Automatic honors reduced motion, On explicitly overrides it, and Off disables flick.
- Bird's Eye View affects the initiating view only. Flick displacement follows the existing Scroll All Windows option and temporary Shift behavior.

## Capabilities

### New Capabilities

- `tools/hand-navigation-gestures` covers the overview and inertial Hand pan.

### Modified Capabilities

None.

## Impact

Canvas input, view painting, Hand shortcuts, General preferences and Qt Test. No dependencies or document-format changes. Scrubby and Animated Zoom belong to #533.
