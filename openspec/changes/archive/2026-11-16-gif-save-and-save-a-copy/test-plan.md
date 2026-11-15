# Test plan

Covers every scenario in the `interop/gif-save`, `interop/save-a-copy`,
`document/document-lifecycle`, `interop/raster-export`, and
`interop/save-format-options` deltas. GIF saves write a snapshot: the document
keeps its mode, depth, pixels, and history; Save a Copy writes bytes without
changing the document. The copied lifecycle and export scenarios keep their
existing coverage; the GIF and copy scenarios are the new tests.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `Save As writes a file` | Save As a document to a new PSD path and reopen it | The file is a valid PSD at that path and the document becomes unmodified | `tst_import_save_coordinator::saveOpenAndRemember`, `tst_document_api::saveAndReopenRoundTrip` |
| `Save uses the recorded path` | Edit a document recorded at `photo.png` and run Save | The same path is rewritten in place | `tst_open_save::rasterOpenKeepsFilePath` |
| `Save on an untitled document prompts for a path` | Run Save on an untitled document through the save-path driver | A path is requested before any write | `tst_save_gif_copy::saveUntitledPromptsForPath` |
| `Save after an edit writes the edited composite` | Edit a raster document, save, reopen | The reopened composite is the edited render, not the pre-edit merge | `tst_open_save::rasterOpenKeepsFilePath` |
| `Save As defaults to the import source format` | Import a single-layer PNG and open Save As | PNG is preselected | `tst_save_format_options::formatKindForSuffix`, `tst_save_gif_copy::gifImportPreselectsGif` |
| `A raster save clears modified and re-saves the same format` | Save a document to `photo.png`, edit, save again | The second save writes a PNG and clears modified | `tst_open_save::rasterOpenKeepsFilePath` |
| `A layered document saved to a flat format warns and flattens` | Save a two-layer document to PNG and accept the warning | The flattened composite is written | `tst_save_gif_copy::layeredGifWarnsAndFlattens` |
| `A layered document defaults Save As to PSD` | Add a layer to a PNG import and open Save As | PSD is preselected | `tst_import_save_coordinator::saveOpenAndRemember` |
| `Save on a layered raster path behaves as Save As` | Save a layered document recorded at `.png` | The format-aware Save As opens instead of overwriting the PNG | `tst_import_save_coordinator::saveOpenAndRemember` |
| `A PSB path writes a PSB container` | Save a document to `.psb` and read the header | The file is a version-2 PSB | `pictura-codec::write_psb_small_document_widens_lengths` |
| `dl_save_current` | Save a modified document through the worker without editing it | The file holds the snapshot, the path is recorded, and the document is unmodified | `tst_offthread_jobs::aCompletedSaveWritesAndClearsModified` |
| `dl_save_stale` | Edit while the save worker runs | The file holds the snapshot and the document stays modified | `tst_offthread_jobs::anEditDuringSaveLeavesTheDocumentModified` |
| `dl_save_failed` | Save to an unwritable target | The save fails, path and dirty state are unchanged, and no temporary file remains | `tst_offthread_jobs::aFailedSaveLeavesTheDocumentUntouched` |
| `dl_save_replaced` | Replace the document while the save worker runs | The file holds the original snapshot and the replacement keeps its path and dirty state | `pictura-app::a_replaced_document_keeps_its_path_and_dirty_state` |
| `The chosen format wins over a typed suffix` | Choose PNG and type `report.jpg` | `report.png` is written and `report.jpg` is absent | `tst_open_save::chosenFormatWinsOverTypedSuffix` |
| `PNG preserves the composite alpha` | Export a document with a transparent pixel to PNG | The pixel is transparent | `tst_save_options::pngNoOption` |
| `JPEG flattens alpha onto an opaque background` | Export a transparent pixel to JPEG | The pixel is opaque | `tst_save_format_options::jpegMatte` |
| `BMP flattens alpha onto an opaque background` | Export a transparent pixel to BMP | The pixel is opaque | `tst_save_options::unsupportedFormats` |
| `Scale resizes the output` | Export a `W x H` document at 50 percent | The output is `W/2 x H/2` and keeps its aspect ratio | `tst_webp::exportHonorsScale`, `tst_export_as::imageSizeAndCanvas` |
| `An unwritable target fails without a partial file` | Write through the encode edge into a missing directory | Failure is returned with no file | `tst_save_pdf::failedWriteCreatesNoFile` |
| `raster-export-gif` | Save a document to `.gif` | The engine GIF encoder writes the file and the Qt raster edge is not used | `tst_save_gif_copy::quantizedGifMatchesDocumentSize` |
| `save-options-jpeg` | Confirm Save As with `.jpg` | JPEG Options is shown first | `tst_save_format_options::jpegQualityChangesSize` |
| `save-options-png` | Confirm Save As with `.png` | PNG Options is shown first | `tst_save_format_options::pngCompressionChangesSize` |
| `save-options-bmp` | Confirm Save As with `.bmp` | BMP Options is shown first | `tst_save_format_options::dialogDefaults` |
| `save-options-tiff` | Confirm Save As with `.tif` | TIFF Options is shown first | `tst_save_format_options::tiffCompressionTag` |
| `save-options-gif` | Confirm Save As with `.gif` | The Indexed Color dialog is shown first | `tst_save_gif_copy::rgbGifSaveShowsDialog` |
| `save-options-native` | Confirm Save As with `.psd`, `.psb`, `.webp`, or `.pdf` | No format options dialog is shown | `tst_save_format_options::formatKindForSuffix` |
| `save-options-cancel` | Cancel a format options dialog | No file is written and the document is unchanged | `tst_save_format_options::cancelWritesNothing` |
| `gif-save-filter` | Open Save As and choose CompuServe GIF for a name without a suffix | The format list contains GIF and the destination ends in `.gif` | `tst_save_gif_copy::gifFilterOffered` |
| `gif-save-recorded-path` | Save a single-layer document to `photo.gif`, edit, run Save | The file starts with `GIF89a`, decodes to the document size, and is not a PSD payload | `tst_save_gif_copy::saveRecordedGifPathRewritesGif` |
| `gif-save-typed-suffix` | Save As with All Formats and type `cat.gif` | A GIF89a file is written at `cat.gif` | `tst_save_gif_copy::typedGifSuffixWritesGif` |
| `gif-save-import-default` | Open a single-layer GIF and invoke Save As | GIF is preselected | `tst_save_gif_copy::gifImportPreselectsGif` |
| `gif-save-atomic` | Save a GIF over an existing file with a failing write | Failure is reported, the old bytes are unchanged, and no partial file remains | `tst_save_gif_copy::gifWriteFailurePreservesPreviousFile`, `pictura-app::gif_write_failure_preserves_previous_file` |
| `gif-dimension-limit` | Save a document wider or taller than 65,535 px as GIF | The save is refused and no file is written | `tst_save_gif_copy::gifDimensionLimitRefused`, `pictura-app::gif_bytes_reject_oversized_dimensions` |
| `gif-rgb-dialog` | Save an RGB document to `.gif` through the options seam | The format-dialog request is for GIF | `tst_save_gif_copy::rgbGifSaveShowsDialog` |
| `gif-save-cancel` | Reject the GIF dialog and inspect the destination | No file is written; the document is unchanged | `tst_save_gif_copy::cancelledGifSaveWritesNothing` |
| `gif-indexed-no-dialog` | Save an Indexed document to `.gif` | No format dialog is requested | `tst_save_gif_copy::indexedGifSaveSkipsDialog` |
| `gif-preview-restores` | Toggle Preview during a GIF save, then accept or cancel | The pre-preview document is restored and no history state is added | `tst_save_gif_copy::gifPreviewRestoresAfterDialog`, `tst_image_mode::indexedPreviewsCancelsAndFlattens` |
| `gif-settings-remembered` | Accept non-default GIF settings and reopen the dialog | Controls hold the accepted settings | `tst_save_gif_copy::gifSettingsRemembered`, `tst_save_format_options::gifSettingsRoundTrip` |
| `gif-exact-fallback` | Remember Exact, then open a >256-color document | The dialog selects an available palette and the file uses it | `tst_save_gif_copy::gifExactFallback` |
| `gif-indexed-palette` | Save an unchanged Indexed source as GIF | The written palette and indices match the source | `tst_save_gif_copy::indexedSourcePalettePreserved`, `pictura-app::gif_bytes_prefer_the_indexed_source` |
| `gif-edited-indexed` | Edit an Indexed document, then save it as GIF | The file uses the edited composite, not stale indices | `tst_save_gif_copy::editedIndexedUsesComposite`, `pictura-app::edited_indexed_source_is_stale` |
| `gif-quantized` | Save a >256-color RGB document as GIF | The file is the document size with at most 256 colors | `tst_save_gif_copy::quantizedGifMatchesDocumentSize`, `pictura-app::gif_bytes_quantize_the_composite` |
| `gif-16bit-snapshot` | Save a 16-bit RGB document as GIF | The file is 8-bpc GIF and the document stays 16-bit | `tst_save_gif_copy::gifFrom16BitKeepsDepth` |
| `gif-32bit-snapshot` | Save a 32-bit RGB document as GIF | The file is 8-bpc GIF and the document stays 32-bit | `tst_save_gif_copy::gifFrom32BitKeepsDepth` |
| `gif-cmyk-snapshot` | Save a CMYK document as GIF | The file is 8-bpc GIF from the sRGB composite and the document stays CMYK | `tst_save_gif_copy::gifFromCmykKeepsMode` |
| `gif-layered-warning` | Save a two-layer document to `.gif` and accept the warning | The flattened composite is written | `tst_save_gif_copy::layeredGifWarnsAndFlattens` |
| `copy-command-enabled` | Open a document, then close all documents, and inspect Save a Copy | Enabled with a document, disabled without one | `tst_save_gif_copy::saveCopyEnabledWithDocument` |
| `copy-dialog-matrix` | Open Save a Copy for a single-layer PNG import | Filters and default format match Save As; PNG is preselected | `tst_save_gif_copy::saveCopyDialogMatchesSaveAs` |
| `copy-cancel` | Cancel the copy path dialog and a format options dialog | Nothing is written and the document is unchanged | `tst_save_gif_copy::saveCopyCancelWritesNothing` |
| `copy-profile-dialog` | Cancel the Embed Color Profile dialog during a copy | Nothing is written and the document is unchanged | `tst_save_gif_copy::saveCopyProfileDialog` |
| `copy-nonmutating-raster` | Save a modified `a.psd` document as a copy to `b.png` | `b.png` holds the flattened composite; `a.psd` stays recorded, modified, and with the same history | `tst_save_gif_copy::saveCopyRasterLeavesDocument`, `pictura-app::copy_intent_skips_path_and_dirty` |
| `copy-psd` | Save a copy to `backup.psd` and reopen it | The copy is a layered PSD; the open document keeps its path and format | `tst_save_gif_copy::saveCopyPsdLeavesDocument` |
| `copy-pdf` | Save a copy to `print.pdf` through the PDF dialog driver | The PDF is written; the document keeps its path, format, and dirty state | `tst_save_gif_copy::saveCopyPdfLeavesDocument` |
| `copy-untitled` | Run Save a Copy on an untitled document through the save-path driver | The copy is written at the chosen path; the document stays untitled | `tst_save_gif_copy::saveCopyUntitledStaysUntitled` |
| `copy-gif` | Save an RGB document as a copy to `web.gif` | The file is GIF89a; the document keeps RGB mode, path, dirty state, and history | `tst_save_gif_copy::saveCopyGifLeavesRgb` |
| `copy-no-recorder` | Record an action and save a copy | The action gains no step | `tst_save_gif_copy::saveCopyNoRecorder` |
| `copy-failed` | Save a copy over an existing file with a failing write | Failure is reported; the old bytes and the document are unchanged; no partial file remains | `tst_save_gif_copy::saveCopyFailurePreservesPreviousFile` |
