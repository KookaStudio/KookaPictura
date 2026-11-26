# Tasks

## 1. Tests first

- [x] 1.1 Add `tst_acr_dialog::bottomControlsShareABaseline` and `tst_acr_dialog::sizeGripSitsBelowOk` covering `rb_baseline` and `rb_size_grip`
- [x] 1.2 Add `tst_acr_dialog::photoInfoForDocument` covering `rb_photo_document` and `rb_photo_missing`
- [x] 1.3 Add `tst_raw_open::rawDialogShowsPhotoInfo` covering `rb_photo_raw`
- [x] 1.4 Add `tst_acr_dialog::pickerSamplesHoveredPixel` and `tst_acr_dialog::pickerRestoresAverage` covering `rb_picker_hover` and `rb_picker_leave`

## 2. Photo information

- [x] 2.1 Capture `RawPhotoInfo` in `PendingRaw` at decode time from the raw file's EXIF, with Rust tests for the f-number, exposure, ISO, focal length, and ppi formatting
- [x] 2.2 Add the `photo_info(view)` free function to the `filter_tools` bridge and implement it for the pending raw and the document
- [x] 2.3 Add the status bar (name, profile, depth, dimensions, megapixels, ppi, file size) and the `QSizeGrip` to `PicturaRawDialog`

## 3. Histogram and picker

- [x] 3.1 Add the exposure labels beside the RGB readout to `AcrHistogram` and populate them from the photo rows
- [x] 3.2 Sample the preview pixel under the cursor in `AcrPreviewPane`, emit it, and re-sample after a settings render
- [x] 3.3 Route the preview's picked colour to the histogram and restore the average on leave

## 4. Bottom bar

- [x] 4.1 Remove the preview bottom row's 4 px inset so the zoom/Shadows row and the right buttons share a baseline
- [x] 4.2 Keep the status bar and size grip below the body so the grip sits under the OK button

## 5. Docs and trackers

- [x] 5.1 Update `public_docs/pictura-raw.md` with the status bar, camera readout, picker, and resize grip
- [x] 5.2 Run `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`, `cargo clippy`, the focused suites, and the full nextest
- [x] 5.3 Archive the change with `openspec archive raw-dialog-inspector`
