# Test Plan

One row per scenario in the change's delta specs under `specs/`. Coverage names
an automated Qt case; the new cases are written before the implementation.

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| `edc_adjustment_cancel` | Open an image, clear the status bar, trigger `Image ▸ Adjustments ▸ Levels`, then reject the dialog | The status bar shows no message; pixels and history are unchanged | `tst_image_adjustments::cancellingAdjustmentDialogsIsSilent` |
| `edc_hdr_cancel` | Trigger `Image ▸ Adjustments ▸ HDR Toning` on the same image, then reject the dialog | The status bar shows no message; pixels and history are unchanged | `tst_image_adjustments::cancellingAdjustmentDialogsIsSilent` |
| `edc_filter_cancel` | Clear the status bar, dispatch `Filter ▸ Blur ▸ Gaussian Blur`, then reject the dialog | The status bar shows no message; history is unchanged | `tst_filter_menu::cancellingAFilterDialogIsSilent` |
| `edc_refusal_reported` | Choose each Blur and Blur Gallery command on an empty layer | Each shows its critical message box and adds no history state | `tst_tilt_shift::everyBlurRefusesAnEmptyLayer` |
