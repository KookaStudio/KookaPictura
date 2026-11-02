# Proposal

## Why

The Brush paints only while the pointer moves. `docs/03-tools/brush-and-pencil.md`
and `docs/07-color-painting/airbrush-and-flow.md` specify an Airbrush option for
the Brush: with Airbrush on, a held button keeps adding paint toward the Opacity
cap; with it off, a still pointer adds nothing. The option, its Brush-panel
Build-up view, and `Shift+Alt+P` are missing (#1120). The stamp tools gained a
CPU-only held-dab path in #799; the Brush also paints through the reduced-level
preview, the GPU stroke, and layer-mask strokes, which that path does not cover.

## What Changes

- **Brush Airbrush option.** The Brush options bar gains an off-by-default
  Airbrush toggle, and the Brush panel's **Build-up** row becomes a working
  checkbox for the same flag. Either view updates the other.
- **Held dabs.** While a Brush stroke is open with Airbrush on, a bounded timer
  re-dabs at the last sample at 20 dabs per second. Releasing, switching tools,
  or cancelling the session stops it.
- **Every Brush backend.** `paint_hold` handles the exact CPU stroke, the
  reduced-level preview, the GPU stroke, and a layer-mask stroke. A previewed
  stroke records its held dabs so the commit replays them and the committed
  pixels include the build-up.
- **`Shift+Alt+P`.** The key toggles the active tool's Airbrush option: the
  Brush's flag, or the per-tool flag of Clone Stamp, Pattern Stamp, and History
  Brush. A tool without an Airbrush option ignores it.

## Capabilities

### Added Capabilities

- `tools/airbrush-build-up`: the Brush Airbrush option, the two-view flag, the
  shortcut, and held dabs on every Brush backend.

## Impact

- `crates/pictura-app`: `paint_hold` routing, `mask_hold`, `gpu_hold`,
  `preview_hold`, and the preview replay log.
- `crates/pictura-app/cpp`: `PaintOptions`, the Brush options bar, the Brush
  panel, the stamp options-bar sync, `ToolController::toggleAirbrush`, the
  `Shift+Alt+P` shortcut, and the keyboard-shortcut list.
- No engine or codec change: `pictura-paint::Stroke::hold` and
  `DabPlacer::repeat_last` already exist from #799.
- Ceiling (`ponytail:`): held dabs are not captured in recorded actions, so a
  replayed action still reproduces the path only.
