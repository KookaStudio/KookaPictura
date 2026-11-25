# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ra_settings_image` | Open the dialog with `Ex12=1.5;Temp=20`, move Exposure to 3, choose Image Settings. | Exposure returns to 1.5 and the settings text carries the opening values. | `tst_acr_dialog::imageSettingsRestoresOpeningSettings` |
| `ra_settings_defaults` | Open the dialog with edited settings, choose Camera Raw Defaults. | The settings text is empty and Exposure and Dehaze read 0. | `tst_acr_dialog::defaultsClearsEveryControl` |
| `ra_settings_previous` | Accept a dialog with Exposure 2, open a fresh dialog, choose Previous Conversion. | The fresh dialog's Exposure is 2 and its settings text carries `Ex12=2`. | `tst_acr_dialog::previousConversionRestoresLastAccepted` |
| `ra_settings_previous_unavailable` | Reset the session record, open a dialog, read the Previous Conversion row; accept a dialog; open another and read it again. | The row is disabled first and enabled after an accepted conversion. | `tst_acr_dialog::previousConversionAvailability` |
| `ra_settings_clear_imported` | Save a dialog with Exposure 2 to an XMP file, load it into a dialog opened with Exposure 0.5, choose Clear Imported Settings. | The dialog reports Exposure 2 after the load, then 0.5 after the clear, and the imported flag is false. | `tst_acr_dialog::clearImportedRevertsToImageSettings` |
| `ra_settings_presets_disabled` | Build the Presets panel and read the nine rows. | The rows are the nine settings actions; Custom Settings, Preset Settings and Apply Preset are disabled, as are Previous Conversion and Clear Imported Settings before their state exists. | `tst_acr_dialog::presetsPanelListsSettingsActions` |
| `ra_files_round_trip` | Save `Ex12=1.5;Dhze=7;Temp=20` to a file, check the packet, load it into a fresh dialog. | The packet carries `crs:Exposure2012` and `crs:Dehaze`; the fresh dialog reports the same three values. | `tst_acr_dialog::saveAndLoadSettingsRoundTrip` |
| `ra_files_non_xmp` | Write a file with no `crs:` property, load it into a dialog opened with `Ex12=1.5`. | Loading returns false and the dialog still reports `Ex12=1.5`. | `tst_acr_dialog::loadRejectsNonXmpFile` |
| `ra_zoom_wheel` | From Fit, send a wheel-up event over the preview, then another, then a wheel-down. | The readout leaves Fit, changes again, then returns to the first step. | `tst_acr_dialog::wheelStepsPreviewZoom` |
| `ra_zoom_ladder` | Wheel up once from Fit in one dialog; press + once from Fit in another. | Both readouts match. | `tst_acr_dialog::wheelStepsPreviewZoom` |
| `ra_layout_tabs` | Open the dialog and read the tab strip. | It draws no base line, expands its tabs, and uses a 16 x 16 icon size. | `tst_acr_dialog::tabStripIsEvenIconStrip` |
| `ra_layout_spacing` | Open the dialog and read the outer layout spacing. | The spacing is at least 12 px. | `tst_acr_dialog::controlsClearPreview` |

## Verification record

- `python3 scripts/check-test-plan.py`: 1 change checked, no failures.
- `openspec validate --all --strict`: 532 passed, 0 failed.
- `bash scripts/with-build-lock.sh -- bash -c 'cmake -S . -B build -G Ninja && cmake --build build --parallel'`: passed.
- `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(acr_dialog|acr_settings|raw_open|pictura_raw|theme_apply|design_system_dialog)$'`: 6/6 passed.
- `cargo nextest run -p pictura_app verb_manifest bridge_sources build_config`: 20/20 build-config tests passed, including the four bridge-layout rules.
- `cargo clippy -p pictura_app --all-targets -- -D warnings`: clean.
- `cargo nextest run --workspace --no-fail-fast`: 6888/6889 passed; the one failure is the pre-existing environment-dependent `pictura-render::oracle imagemagick_fixtures_reproduce` (`ramp_Divide.rgba`), which no file in this change touches.
- `bash scripts/check-file-size.sh`: OK (5 files over budget, all allowlisted).
- `bash scripts/guard.sh`: OK.
