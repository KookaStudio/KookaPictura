# Tasks

## 1. Tests first

- [x] 1.1 Add `tst_image_adjustments::cancellingAdjustmentDialogsIsSilent` covering `edc_adjustment_cancel` and `edc_hdr_cancel`: open an image, clear the status bar, trigger Levels and HDR Toning with a zero-delay reject, and assert no message, unchanged pixels, and unchanged history.
- [x] 1.2 Add `tst_filter_menu::cancellingAFilterDialogIsSilent` covering `edc_filter_cancel`: dispatch Gaussian Blur with a zero-delay reject and assert no status message and unchanged history.

## 2. Handler fix

- [x] 2.1 In `frame_menus_adjust.cpp`, split the readiness check from the dialog result for Image ▸ Adjustments and HDR Toning so only a not-ready target reports a refusal.

## 3. Integration

- [x] 3.1 Update the `runAdjustmentDialog` comment to state that a false result is a cancel or an unopenable dialog.
- [x] 3.2 Run `openspec validate --all --strict`, the two Qt suites, and `bash scripts/verify-fast.sh`.
- [x] 3.3 Archive the change and verify `openspec/changes/` has no non-archived change.
