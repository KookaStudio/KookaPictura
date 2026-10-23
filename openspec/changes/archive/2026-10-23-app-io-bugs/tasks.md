# Tasks

## 1. #871 guides during load (crash first)

- [x] Add the failing regression test `tst_rulers_guides::togglesDuringLoadingDoNotCrash`: start a temp PSD with `setOpenDelayForTest` and `startOpenPath` (the `openDocumentAtPath` wrapper blocks until completion, so it cannot dispatch during the load), dispatch `ViewRulers` and `ViewShowGuides` while the tab is loading, then finish the open and assert the completed canvas has visible rulers and `guidesVisible()`; reset the delay at the end.
- [x] Guard `applyGuideViewState` so a `DocEntry` with null `canvas` or `canvasHost` is skipped; keep the persisted flag update.
- [x] Confirm `setupCanvas` → `prepareCanvasGuides` applies the current state when the load completes.
- [x] Run only that test, then the `tst_rulers_guides` suite.

## 2. #874 raster Open keeps a path

- [x] Add `tst_open_save.cpp` to `crates/pictura-app/cpp/tests/CMakeLists.txt` and the failing test `rasterOpenKeepsFilePath`: open a temp PNG through `openDocumentAtPath`, assert `documentPath(0) == path`, paint a pixel (keeping one layer), call `saveActive()`, assert it returns true and the file changed without Save As, then call `revertActive()` and assert it also returns true.
- [x] Change `decode_image` to return `OpenedPath::Set(Some(path))`; remove `OpenedPath::Keep` and its `previous_path` bookkeeping when no caller remains.
- [x] Pass `path` to `beginLoadingDocument` in `startOpenImagePath` (it currently passes `QString()`, so the C++ `DocEntry::path` behind `documentPath`, `saveActive`, and `revertActive` stays empty) and update the `open_image` bridge doc comment that calls a fresh raster open untitled.
- [x] Update `crates/pictura-app/src/cxxqt_object/open_jobs/tests.rs::open_image_matches_sync` to assert the path is recorded; do not delete the test.
- [x] Run only that test, then the `tst_open_save` suite.

## 3. #875 Save As format and metadata

- [x] Add the failing `tst_open_save::chosenFormatWinsOverTypedSuffix`: with the PNG filter and the typed name `report.jpg`, `pathForSaveFilter` returns `report.png`, saving that path decodes as PNG, and `report.jpg` is not created; also assert All Formats keeps a typed `.jpg`.
- [x] Add `pathForSaveFilter(path, chosenFilter, fallbackFilter)` in `dialogs.cpp` and declare it in `dialogs.h`: a single-format filter replaces the suffix with `defaultSuffixForFilter(chosenFilter)`, while All Formats keeps a typed suffix and appends `defaultSuffixForFilter(fallbackFilter)` only when the name has none; lowercase known image suffixes. Use it in `saveAsWithDialog` before the PDF and feature-warning branches. The test calls the helper directly because the native `getSaveFileName` has no test seam.
- [x] Add the failing Rust tests `pictura-codec::jpeg_metadata_injection` and `tiff_metadata_injection` over minimal container fixtures.
- [x] Add `crates/pictura-codec/src/raster_metadata/` (`mod.rs`, `jpeg.rs`, `tiff.rs`): gather EXIF (1058/1059), IPTC-IIM (1028), and XMP (1060) from `Document.image_resources`; skip an absent resource and leave a document with none unchanged; inject into JPEG APP1/APP13 and TIFF tags 34665/33723/700; strip existing copies before inserting. No new dependency.
- [x] In `document_api::save`, encode the raster to a temp path, inject the metadata, and rename over the target for JPEG/TIFF only; a failed injection removes the temp and leaves the old file untouched and returns false. Extract the temp/inject/rename helper to `export.rs` if `impl_core.rs` would pass its 1200-line cap.
- [x] Add the failing `tst_open_save::rasterSaveKeepsFileInfo`: open `crates/pictura-codec/tests/fixtures/metadata.psd` (via `QFINDTESTDATA`), save as `.jpg` and `.tif`, and use exiftool to assert Title, ObjectName, and Make; `QSKIP` when exiftool is absent.
- [x] Run only those tests, then the `pictura-codec` and `tst_open_save` suites.

## 4. #876 Print Settings page setup

- [x] Add the failing `tst_print::printSettingsPersistPageSetup`: reset `lastPrintSettings`, use the default Save as PDF destination, stub the page-setup provider to set A4 and Cassette, assert the label/preview use A4, the settings retain A4 and Cassette, and `configurePrinter` produces an A4 `QPrinter`; Done persists the settings.
- [x] Add `QPageSize pageSize` and `QPrinter::PaperSource paperSource` to `PrintSettings` (include `<QtGui/QPageSize>` and `<QtPrintSupport/QPrinter>`); an invalid `QPageSize()` means the destination default (Letter for Save as PDF), the source defaults to `QPrinter::Auto`, and `configurePrinter` applies both, including Save as PDF.
- [x] Enable Print Settings for Save as PDF so the page size is selectable without a printer; give the button a test object name (`printPageSetup`).
- [x] Add `PrintDialog::setPageSetupProvider(std::function<bool(QPrinter&)>)`; the default opens `QPrintDialog` for a printer and `QPageSetupDialog` for Save as PDF, then reads the accepted page size and source back. Keep the chosen values in members and use them in `settings()`, `paperInches()`, and `updatePreview()`.
- [x] Keep `tst_print::previewAndOrientation` and `tst_print::saveAsPdf` passing: an invalid retained page size still previews and prints Letter (8.5×11 in) for Save as PDF.
- [x] Run only that test, then the `tst_print` suite; note the real-CUPS row for the PR checklist.

## 5. #873 partial recovery

- [x] Add the failing `tst_recovery::partialRecoveryQuarantinesFailedDocument`: stage one readable and one unreadable snapshot, recover, assert the readable document opens, the failed snapshot and manifest exist under `recovery/quarantine/<session>/`, the orphan session is gone, a second `RecoveryStore::orphans()` scan reports no session for the quarantine directory, and the quarantined files still exist.
- [x] Add `RecoveryStore::quarantine(sessionDir, doc)` under `rootPath()/quarantine/<session>/`; create the directory, refuse a source outside the session/recovery root and a destination outside the recovery root, copy both files, and remove the session copies only after both copies succeed (remove a partial copy on failure).
- [x] Skip the `quarantine` directory in `RecoveryStore::orphans()`; it scans every direct child of the recovery root and both Recover and Later discard an empty session, which would delete the quarantine. Never pass it to `discard`.
- [x] In `recoverOrphans`, remove each recovered document's snapshot/manifest, quarantine each failed document, and discard the orphan session only when no quarantine failed and every recovered document's new snapshot landed. If either fails, keep the session intact.
- [x] Run only that test, then the `tst_recovery` suite.

## 6. #872 Info readouts

- [x] Add the failing `tst_info_panel::physicalUnitsUseDocumentPpi`: create a 300 ppi document (`image_size_apply` on the same dimensions, or `create_document`), then a 300 px cursor reads `1.00` inches.
- [x] Add the failing `tst_info_panel::opacityReadoutUsesLayerOpacity`: a 50%-opacity pixel layer reads `Op : 50`; the Background leaves the row blank.
- [x] Update the existing `tst_info_panel::colorMenuModes` Opacity assertion: its document has the Background active, which now blanks the row; add and select a pixel layer before checking `Op : 100`. Do not delete the check.
- [x] Change `formatMeasure` to divide by `document_ppi(*view_)` (72 without a document); include `pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h` for the free `document_ppi` declaration.
- [x] Add `active_layer_opacity()` to the `PictureView` bridge, returning `-1` for the Background or no active layer; use it for `ColorReadout::Opacity` and blank the row on `-1`.
- [x] Run only those tests, then the `tst_info_panel` suite.

## 7. #845 Bitmap extra channel saves

- [x] Add the failing `pictura-codec::bitmap_with_extra_channel_roundtrips`: build a depth-1 Bitmap PSD carrying one packed extra channel, write, read back, and assert the selected/unselected pattern survives.
- [x] Add the failing `pictura-codec::bitmap_extra_channel_survives_rgb_fallback`: change the composite so the Bitmap write-back is refused, write, and assert the output is RGB with an expanded extra channel.
- [x] Let `writes_bitmap` accept extra channels only when every extra's data length is `row_bytes(width, 1) * height`; keep the unchanged-composite and no-layer requirements.
- [x] In `write_container`, special-case the Bitmap extras: validate the packed length and write them verbatim in Bitmap mode; in the RGB fallback, expand only packed extras with the existing row walker before `native_plane` and pass an already-8-bit extra through unchanged.
- [x] Move the Bitmap plane assembly (packed composite plus extras, and the fallback expansion) into `write_bitmap.rs`; `write.rs` is at 1188/1200 lines and must not grow past the cap.
- [x] Update the Bitmap write-back comments to state the new scope. Do not edit the stale `codec/psd-color-modes` clause in this change; the design records the follow-up.
- [x] Run only those tests, then the `pictura-codec` suite.

## Final

- [x] `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/verify-fast.sh`
- [x] `python3 scripts/check-test-plan.py`
- [x] `openspec validate app-io-bugs --strict`
- [x] `openspec validate --all --strict`
- [x] `openspec archive app-io-bugs`
- [ ] PR body lists the manual rows as a reviewer checklist.
