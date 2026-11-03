# Proposal

## Why

The Brush options bar is missing CS6's two tablet-pressure overrides:
**Always use Pressure for Opacity**, to the right of Opacity, and **Always use
Pressure for Size**, at the far right after Airbrush. `docs/03-tools/brush-and-pencil.md`
lists the pressure buttons, but the bar has no controls and the engine never
reads `StrokeSample::pressure`, so even a tablet stroke paints at full size and
opacity.

## What Changes

- **Two icon toggles.** The Brush bar gains the pressure-override buttons in
  their CS6 positions, with the pressure-opacity and pressure-size icons. Both
  are off by default and persist with the brush options.
- **Engine pressure.** `StrokeConfig` carries the two overrides. Each sample
  scales the dab diameter by pressure for size and the per-dab flow and opacity
  cap for opacity; a held dab keeps the pressure it was pressed with.
- **Tablet routing.** The canvas takes `QTabletEvent` pressure and the Brush and
  stamp handlers pass it to `paint_dab`. A mouse press reports 1.0; a Qt-
  synthesized mouse event keeps the tablet pressure.
- **Preview.** The reduced-level preview applies the same per-sample pressure,
  so a large pressure stroke commits the build-up it showed.
- **GPU.** A pressure-override stroke stays on the exact CPU stroke or the
  preview; the GPU dab shader has static parameters and does not take per-dab
  pressure.

## Capabilities

### Added Capabilities

- `tools/brush-pressure-overrides`: the two options-bar toggles, their icons,
  persistence, and the pressure path through the stroke, preview, and tablet
  input.

## Impact

- `crates/pictura-paint`: `StrokeConfig::pressure_scaled`, the per-sample stroke
  raster, the held-dab pressure, and unit tests.
- `crates/pictura-app`: the options bar, `PaintOptions`, the brush JSON, the
  preview raster, the GPU decline, `ToolServices::pressure`, the canvas tablet
  event, and `tst_brush_airbrush`.
- Assets: `options.pressureOpacity.svg` and `options.pressureSize.svg`.
- Ceiling (`ponytail:`): the GPU path declines pressure strokes; the exact CPU
  path or the preview runs instead.
