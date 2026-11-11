# Adjustment dialog Auto buttons

## Why

Issue #578 records three gaps in the Image > Adjustments dialogs: the
Brightness/Contrast Auto button is created disabled, Curves has no Auto button,
and Levels keeps its Auto logic inline instead of sharing it. The CS6 specs say
Levels, Curves, and Brightness/Contrast all use the same improved Auto
correction (`docs/04-image-ops/adjustments-overview.md`), but the configurable
Auto Color Correction Options dialog is a separate gap (#559, parity row
`IMG-7`).

## What Changes

- **Shared Auto bounds.** `AdjustmentDialog` gains one histogram helper that
  returns the black/white endpoints after discarding 0.1% of the pixels at each
  end of the dialog's canvas histogram. Levels, Curves, and Brightness/Contrast
  all use it.
- **Levels Auto.** The existing behaviour moves onto the shared helper and the
  shared button wiring; it still sets the selected channel's input black/white.
- **Curves Auto.** The Curves dialog gains an Auto button that replaces the
  selected channel's curve with a two-point stretch from the shared black
  endpoint to output 0 and the white endpoint to output 255.
- **Brightness/Contrast Auto.** The disabled button becomes live. It derives a
  brightness and contrast pair from the composite histogram: brightness centres
  the occupied range through the engine's modern gamma curve, and contrast
  scales the S-curve slope to expand the remaining range. Adobe's exact fit is
  closed, so this is behavioural parity and is marked as such.
- **No Auto Options dialog.** `Alt`/`Option`-click and the persisted Auto
  defaults remain with #559. The Properties panel Auto button remains out of
  scope.

## Capabilities

### New Capabilities

- `ui/adjustment-dialog-auto`: the shared histogram Auto bounds, the button
  wiring, and the per-dialog behaviour of Levels, Curves, and
  Brightness/Contrast.

## Impact

- **C++ app**: `crates/pictura-app/cpp/adjustment_dialog.{h,cpp}`,
  `levels_dialog.{h,cpp}`, `curves_dialog.{h,cpp}`, `slider_dialog.{h,cpp}`.
- **Tests**: new cases in `tst_image_adjustments`.
- **No engine, bridge, document-format, or dependency change.**
