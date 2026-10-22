# Test plan

All automated tests run on CPU. The Rust cases live in `pictura-codec`; the GUI cases use Qt Test and the offscreen platform.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| An unchanged Bitmap keeps its packed extra channel | A depth-1 Bitmap PSD with one packed extra channel, written and read back without changing the composite | Header mode Bitmap/depth 1; the extra channel's pattern survives | `pictura-codec::bitmap_with_extra_channel_roundtrips` |
| An edited Bitmap falls back to RGB and expands the extra channel | Same document, change the composite, write and read back | Output is 8-bit RGB; the extra channel has one sample per pixel with the same pattern | `pictura-codec::bitmap_extra_channel_survives_rgb_fallback` |
| Toggling while a document loads does not crash | Start a temp PSD with `setOpenDelayForTest` and `startOpenPath`; dispatch ViewRulers and ViewShowGuides while loading | No crash; both flags change; the completed canvas shows the new state | `tst_rulers_guides::togglesDuringLoadingDoNotCrash` |
| `info_unit_mm` | 72 ppi document, cursor at 2 px, unit Millimeters | X and Y read `0.71` | `tst_info_panel::measurementUnitReformatsPosition` |
| `info_unit_per_block` | Set X/Y to Millimeters with a selection present | Position converts and W/H is reformatted in the same unit | `tst_info_panel::unitInheritanceAndNoSizeMenu` |
| `info_unit_size_no_menu` | Inspect the size block | No menu; its unit follows the position block | `tst_info_panel::unitInheritanceAndNoSizeMenu` |
| Physical units use the document resolution | 300 ppi document, Inches, cursor at 300 px | X reads `1.00` | `tst_info_panel::physicalUnitsUseDocumentPpi` |
| `info_color_mode_cmyk` | Set the top-left block to CMYK | Rows become C/M/Y/K with 0–100 percentages | `tst_info_panel::colorBlockReadsRgbAndSwitchesModes` |
| `info_color_mode_gray` | Set the top-left block to Grayscale over white | `K : 255` | `tst_info_panel::colorBlockReadsRgbAndSwitchesModes` |
| Total Ink and Opacity rows | Set Total Ink then Opacity over opaque white with an opaque active layer | `Ink : 0`, then `Op : 100` | `tst_info_panel::colorMenuModes` |
| Top-left defaults to Actual Color | Construct the panel | Top-left Actual Color; top-right CMYK | `tst_info_panel::defaultColorModeIsActualColor` |
| Opacity reads the active layer | 50%-opacity pixel layer over an opaque pixel, set Opacity | `Op : 50`, independent of the sampled alpha | `tst_info_panel::opacityReadoutUsesLayerOpacity` |
| The Background has no opacity readout | Select the Background, set Opacity | The `Op` row is blank | `tst_info_panel::opacityReadoutUsesLayerOpacity` |
| A crashed document comes back | Leave a 64×48 Untitled-1 snapshot with a Work Path in a lock-less session; recover in a new window | Opens as `Untitled-1-Recovered`, dirty, no path, Work Path kept; orphan session deleted | `tst_recovery::recoverReopensAnOrphanedDocument` |
| A partial recovery quarantines the failed document | Stage one readable and one unreadable snapshot; run Recover; scan orphans again | Readable document opens; failed snapshot and manifest live under `recovery/quarantine/<session>/`; the orphan session is gone; a second scan reports no session for the quarantine directory and the quarantined files still exist | `tst_recovery::partialRecoveryQuarantinesFailedDocument` |
| A raster open keeps its path | Open a temp PNG, paint a pixel, call `saveActive()`, then `revertActive()` | `documentPath(0)` equals the PNG path; save succeeds without Save As and rewrites the file; Revert also succeeds | `tst_open_save::rasterOpenKeepsFilePath` |
| The chosen format wins over a typed suffix | PNG filter, type `report.jpg` | `report.png` is written and decodes as PNG; no `report.jpg` | `tst_open_save::chosenFormatWinsOverTypedSuffix` |
| JPEG and TIFF keep the File Info fields | Open `metadata.psd`, save as `.jpg` and `.tif`, inspect with exiftool | Title, ObjectName, and Make survive (skip without exiftool) | `tst_open_save::rasterSaveKeepsFileInfo`, `pictura-codec::jpeg_metadata_injection`, `pictura-codec::tiff_metadata_injection` |
| A4 chosen in Print Settings reaches the preview and the job | Stub the page-setup provider with A4 and Cassette; accept | Label/preview use A4; settings retain A4 and Cassette; `configurePrinter` produces an A4 QPrinter; real CUPS queue confirms the source | `tst_print::printSettingsPersistPageSetup`, manual: real CUPS queue for the paper source |
