# Proposal: Mixer Brush bar and Sample All Layers

## Why

#911: the Mixer Brush options bar differs from Photoshop CC, Sample All Layers is a disabled control, and the mixing has no reference test that records what is approximated. The CC reference from the #908 manual pass is not available to the project, so the bar follows `docs/03-tools/mixer-brush.md` and the issue text.

## What changes

- The Mixer Brush bar drops Mode and Opacity (added from the docs in #866); a Mixer stroke is Normal at 100 %. The engine keeps mode and opacity support.
- Sample All Layers works: the pickup reads the visible composite, snapshotted at the press, instead of the active layer.
- The mixing approximation limits are recorded in the engine header and the spec, with an ordering test across the Dry, Moist and Very Wet settings.
- Load Solid Colors Only and keyboard shortcuts stay with #540; the CC bar's icon toggles, 0 % field and brush angle are not modelled.

## Capabilities

- Modified: `tools/mixer-brush-tool`
