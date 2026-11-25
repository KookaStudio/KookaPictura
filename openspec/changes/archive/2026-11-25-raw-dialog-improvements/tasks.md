# Tasks

## 1. Tests first

- [x] 1.1 Add the Qt test `tst_acr_dialog::imageSettingsRestoresOpeningSettings` covering `ra_settings_image`; verify it fails before 3.3
- [x] 1.2 Add `tst_acr_dialog::defaultsClearsEveryControl` covering `ra_settings_defaults`
- [x] 1.3 Add `tst_acr_dialog::previousConversionRestoresLastAccepted` and `tst_acr_dialog::previousConversionAvailability` covering `ra_settings_previous` and `ra_settings_previous_unavailable`
- [x] 1.4 Add `tst_acr_dialog::clearImportedRevertsToImageSettings` covering `ra_settings_clear_imported`
- [x] 1.5 Add `tst_acr_dialog::presetsPanelListsSettingsActions` covering `ra_settings_presets_disabled`
- [x] 1.6 Add `tst_acr_dialog::saveAndLoadSettingsRoundTrip` and `tst_acr_dialog::loadRejectsNonXmpFile` covering `ra_files_round_trip` and `ra_files_non_xmp`
- [x] 1.7 Add `tst_acr_dialog::wheelStepsPreviewZoom` covering `ra_zoom_wheel` and `ra_zoom_ladder`
- [x] 1.8 Add `tst_acr_dialog::tabStripIsEvenIconStrip` and `tst_acr_dialog::controlsClearPreview` covering `ra_layout_tabs` and `ra_layout_spacing`

## 2. Bridge

- [x] 2.1 Add `crates/pictura-app/src/cxxqt_object/impl_pictura_raw/pictura_raw_xmp.rs` exposing `pictura_raw_settings_from_xmp(QByteArray) -> QString` and `pictura_raw_settings_to_xmp(QString) -> QByteArray` over `pictura_codec`
- [x] 2.2 Declare the module from `impl_pictura_raw.rs` and regenerate `bridge_sources.txt`
- [x] 2.3 Include the generated header from `pictura_raw_dialog.cpp`

## 3. Dialog

- [x] 3.1 Make `AcrPresetsPanel` list the nine settings actions, emit one signal per action, and expose `setPreviousConversionAvailable` / `setImported`
- [x] 3.2 Keep the opening settings text, the as-shot value, the stacked widget, and the control-panel list on `PicturaRawDialog`; rebuild panels 0–7 when settings are replaced
- [x] 3.3 Implement Image Settings, Camera Raw Defaults, Previous Conversion, Clear Imported Settings, and the session record in `accept()`
- [x] 3.4 Implement `loadSettingsFile` / `saveSettingsFile` and the Load/Save menu actions
- [x] 3.5 Set the tab strip to no base line, even tabs, and a fixed icon size; name it `acrTabBar` and add the theme rule
- [x] 3.6 Give the outer layout an explicit margin and spacing so the controls clear the preview
- [x] 3.7 Step the preview zoom from `AcrPreviewPane::wheelEvent`

## 4. Docs and trackers

- [x] 4.1 Update `public_docs/pictura-raw.md` Presets section to describe the working actions
- [x] 4.2 Update the Presets roadmap item and `PARITY.md`'s P-FLT-21 note when the change ships
- [x] 4.3 Archive the change with `openspec archive raw-dialog-improvements`
- [x] 4.4 Run `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`, and the focused suites
