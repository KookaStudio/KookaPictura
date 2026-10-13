# Tasks

## 1. Canvas sampling from the Color Picker

- [x] 1.1 `ColorPickerDialog::sampleCanvas` arms the tool controller's canvas sampler and disarms it when the picker finishes; `getColor` forwards an optional tool controller and colour source.
- [x] 1.2 `setDialogCanvasSampling` lets `runDialog`'s blocker pass left-button canvas input while a sampler is armed; the frame installs its tool controller's state.
- [x] 1.3 `runAdjustmentDialog` takes the `ToolController`; `AdjustmentDialog::originalColorAt` samples the pre-preview composite; Photo Filter's swatch passes both to the picker.

## 2. Verification

- [x] 2.1 `tst_image_adjustments::photoFilterColorPickerSamplesTheImage` clicks the canvas through both dialogs' blockers and asserts the picker takes the un-previewed colour, the swatch switches to Color, the sampler is disarmed, and no history is recorded.
- [x] 2.2 `openspec validate --all --strict` and `bash scripts/verify-fast.sh` pass.
