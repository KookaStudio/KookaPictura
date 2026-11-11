# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `auto_shared_clip` | Open a 40–192 ramp; click Auto in Levels, Curves, and Brightness/Contrast | Each dialog derives its edit from the same 0.1%-clipped endpoints | `tst_image_adjustments::levelsAutoStretchesTheRamp`, `tst_image_adjustments::curvesAutoStretchesTheChannel`, `tst_image_adjustments::brightnessContrastAutoRaisesContrast` |
| `auto_flat_noop` | Open a single-level image; click Auto in Levels, Curves, and Brightness/Contrast | Parameters are unchanged and no history state is added | `tst_image_adjustments::autoIsANoOpOnAFlatImage` |
| `auto_levels_stretch` | Open a 40–192 ramp; select Red; click Auto | Input black is about 40, input white is about 192, and the preset reads `Custom` | `tst_image_adjustments::levelsAutoStretchesTheRamp` |
| `auto_curves_stretch` | Open a 40–192 ramp; select a channel; click Auto | The channel curve is two points at `(40, 0)` and `(192, 255)`, within one level | `tst_image_adjustments::curvesAutoStretchesTheChannel` |
| `auto_bc_contrast` | Open a 64–192 neutral ramp; click Auto | Brightness stays near 0, contrast is positive, and the previewed range widens | `tst_image_adjustments::brightnessContrastAutoRaisesContrast` |
| `auto_button_present` | Open Levels, Curves, Brightness/Contrast, and Vibrance | The first three expose an enabled `Auto` button; Vibrance exposes none | `tst_image_adjustments::brightnessContrastPreviewsAndApplies`, `tst_image_adjustments::levelsEditsEachChannelAndPresets`, `tst_image_adjustments::curvesAutoStretchesTheChannel`, `tst_image_adjustments::exposureAndVibranceUseTheWideLayouts` |
