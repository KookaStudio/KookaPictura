# Proposal

## Why

Cancelling an edit dialog is not a failure, but the Image ▸ Adjustments and
HDR Toning handlers treat a rejected dialog as a refusal. After Cancel, the
status bar's banner reads "The filter could not be applied." even though the
user asked to abandon the edit and the document is untouched.

## What Changes

- Treat a rejected adjustment or HDR Toning dialog as a cancel: no refusal
  banner and no refresh.
- Keep reporting refusals for commands refused before their dialog opens and
  for commits the engine refuses.
- Add Qt Test coverage for cancelling an adjustment, HDR Toning, and a filter
  dialog, plus the existing empty-blur refusal as the counter-case.

## Capabilities

### New Capabilities

- `ui/edit-dialog-cancel-feedback`: cancelling any filter, effect, or
  adjustment dialog is silent.

### Modified Capabilities

None.

## Impact

`crates/pictura-app/cpp/frame_menus_adjust.cpp`; Qt Test suites
`tst_image_adjustments` and `tst_filter_menu`. No engine, codec, dependency, or
file-format changes.
