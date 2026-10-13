# Proposal

## Why

In Image > Adjustments > Photo Filter, choosing **Color** opens the Color
Picker, but its eyedropper cannot reach the image (#413). In CS6 the pointer
becomes an eyedropper over the canvas while the picker is open, and a click
takes the image's colour. Here `runDialog`'s input blocker swallows every
click on the main window, so the only way to match a colour from the photo is
to read it off by eye.

## What Changes

- A Color Picker opened from the Photo Filter dialog arms the tool
  controller's canvas sampler (the same seam Replace Color and Color Range
  use): the canvas shows the eyedropper cursor and a left click sets the
  picker's colour instead of running the active tool.
- The colour is the composite as it was before the dialog's preview, so the
  filter's own tint is not sampled back.
- `runDialog`'s input blocker lets left-button input over a canvas through
  while a canvas sampler is armed (`setDialogCanvasSampling`), so the click
  passes both the Photo Filter and the picker blockers.
- The sampler is disarmed when the picker closes.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `imaging/image-adjustments`: adds "Photo Filter colour picker samples the
  image".

## Impact

- `crates/pictura-app/cpp/color_picker_dialog.{h,cpp}` — `sampleCanvas`, and
  `getColor` takes an optional tool controller and colour source.
- `crates/pictura-app/cpp/adjustment_dialog.{h,cpp}` — `setCanvasSampling`,
  `originalColorAt`; `runAdjustmentDialog` takes the `ToolController`.
- `crates/pictura-app/cpp/photo_filter_dialog.{h,cpp}` — wires the swatch's
  picker to the canvas.
- `crates/pictura-app/cpp/dialogs.{h,cpp}`, `frame_menus_edit.cpp`,
  `frame_menus_adjust.cpp` — the blocker passthrough and its frame hook.
- `crates/pictura-app/cpp/tests/tst_image_adjustments.cpp` — regression test.
- No Rust, no new dependencies, no document or codec behaviour change.
