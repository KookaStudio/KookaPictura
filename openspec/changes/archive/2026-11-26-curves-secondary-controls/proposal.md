## Why

Curves supports spline-point edits, presets and a destructive Auto button. Users still need the secondary editing tools described in issue #561 to work directly on the graph and image in both adjustment entry points.

## What Changes

- Share secondary graph controls between Image > Adjustments > Curves and the Curves Properties page.
- Add exact point controls, endpoint sliders, display options, pencil drawing and repeatable smoothing.
- Add targeted image adjustment and black, gray and white sample tools that participate in the existing preview and history lifecycle.
- Extend mode-specific editing to Gray, CMYK and Lab and reuse the fixed histogram Auto for RGB and Gray.

## Capabilities

### New Capabilities

- `imaging/curves-secondary-controls` describes the editing tools, display units and transaction boundaries.

### Modified Capabilities

None.

## Impact

Affected areas include the Qt dialogs, Properties panel, canvas tool routing, adjustment bridge, Curves serialization and CPU/native rendering and GPU eligibility. No new dependencies.
