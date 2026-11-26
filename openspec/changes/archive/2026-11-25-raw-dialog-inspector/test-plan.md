# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rb_baseline` | Show the dialog and compare the bottom edge of the Shadows toggle with the OK button. | Both bottom edges are equal. | `tst_acr_dialog::bottomControlsShareABaseline` |
| `rb_size_grip` | Show the dialog, find the size grip and the OK button. | The grip is visible and its top edge is at or below the OK button's bottom edge. | `tst_acr_dialog::sizeGripSitsBelowOk` |
| `rb_photo_raw` | Open a 16 by 12 DNG named `shot.dng` through the raw-open driver and read the status labels. | The name is `shot.dng`; the summary reports 16 by 12 and 16 bits. | `tst_raw_open::rawDialogShowsPhotoInfo` |
| `rb_photo_document` | Build a 16 by 16 8-bit RGB document and open the filter dialog; read the status labels. | The summary reports 16 by 16, 8 bits, sRGB, and 72 ppi. | `tst_acr_dialog::photoInfoForDocument` |
| `rb_photo_missing` | Read the histogram exposure labels for a document with no EXIF exposure tags. | Both exposure labels are empty. | `tst_acr_dialog::photoInfoForDocument` |
| `rb_picker_hover` | Show the dialog, give the preview a two-colour image, and move the cursor over the red quadrant. | The RGB readout shows the red pixel's values. | `tst_acr_dialog::pickerSamplesHoveredPixel` |
| `rb_picker_leave` | Hover a pixel, then send a leave event to the preview. | The readout returns to the average it showed before the hover. | `tst_acr_dialog::pickerRestoresAverage` |

## Verification record

- CMake configure + build: passed.
- `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(acr_dialog|acr_settings|raw_open|pictura_raw|theme_apply)$'`: 5/5 passed.
- `cargo nextest run -p pictura_app raw_photo_info build_config`: 22/22 passed.
- `cargo nextest run -p pictura-codec camera_exposure_rows resolution_ppi`: 2/2 passed.
- `cargo clippy -p pictura_app -p pictura-codec --all-targets -- -D warnings`: clean.
- `cargo fmt --all --check`: clean.
- `bash scripts/check-file-size.sh`: OK (5 files over budget, all allowlisted).
- `bash scripts/guard.sh`: OK.
- `openspec validate --all --strict`: 532 passed, 0 failed.
- `cargo nextest run --workspace --no-fail-fast`: 6892/6893 passed; the one failure is the pre-existing environment-dependent `pictura-render::oracle imagemagick_fixtures_reproduce` (`ramp_Divide.rgba`), which no file in this change touches.
