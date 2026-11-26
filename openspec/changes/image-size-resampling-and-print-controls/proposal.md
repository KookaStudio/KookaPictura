## Why

Image Size exposes cubic modes that currently produce the same result and has unfinished print-sizing controls. Its tiled native-depth path also clips extended-range float inputs before interpolation, losing HDR and negative samples.

## What Changes

- Give Bicubic Smoother and Sharper distinct kernels and resolve Automatic by the direction of each resize axis.
- Preserve finite extended-range samples through tiled 32-bit resampling.
- Add Scale Styles for stored supported effects, including disabled effects, with rounded proportional sizing, replay and undo; enable Auto Resolution from screen frequency.
- Add physical pica and column units and editable Units & Rulers column dimensions.

## Capabilities

### New Capabilities

- `imaging/image-size-controls` covers interpolation, precision, layer-style scaling, and print controls.

### Modified Capabilities

- `ui/rulers-and-guides` enables persisted Column Size width and gutter preferences while retaining the other disabled controls.

## Impact

Changes touch the Image Size dialog and command, Units & Rulers preferences, the shared CPU resampler, and document resize and style operations. The spillable tile pipeline keeps its existing four-tap bound. No dependencies are added. Implements #565.
