# Test Plan

One row per scenario in the change's spec deltas. Format logic is covered by
Rust unit tests in `pictura-codec` and `pictura-app`; the command, dialog, and
file outcomes are covered offscreen by `tst_file_export_tools`,
`tst_export_layers`, and the existing `tst_busy_job`. Illustrator and a real
browser are not available in CI, so their end-to-end checks are reviewer items
at the end of this plan.

## Zoomify export

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `zf_dialog` | Open `File ▸ Export ▸ Zoomify…` on an open document. | The dialog shows Output Location, Browser Options width/height, Image Tile Options quality, the Template menu, Cancel, and Export. | `tst_file_export_tools::zoomifyDialogShowsControls` |
| `zf_dialog_defaults` | Open the dialog on a 1200 × 800 document. | Base width/height are 1200/800, quality is 75, and the quality range is 0 to 100. | `tst_file_export_tools::zoomifyDialogDefaults` |
| `zf_zero_base` | Set a base dimension to 0, then above the document size. | Zero is refused; a larger value clamps to the document dimension. | `tst_file_export_tools::zoomifyDialogBounds` |
| `zf_dialog_unchanged` | Open the dialog and cancel. | Pixels, path, modified state, and history are unchanged. | `tst_file_export_tools::zoomifyDialogLeavesDocumentUnchanged` |
| `zf_no_document` | Dispatch the command with no document. | The action is disabled and no dialog opens. | `tst_file_export_tools::exportCommandsNeedADocument` |
| `zf_one_tier` | Export a 200 × 100 base to a temporary folder. | `TileGroup0/0-0-0.jpg` exists and `ImageProperties.xml` reports `NUMTILES="1"`. | `pictura-codec::zoomify_plan_small_base_is_one_tile`, `tst_file_export_tools::zoomifyWritesOneTier` |
| `zf_multi_tier` | Export a 1000 × 700 base. | Tiers hold 1, 4, and 12 tiles, `NUMTILES="17"`, and `TileGroup0/2-3-2.jpg` exists. | `pictura-codec::zoomify_plan_multi_tier_tiles`, `tst_file_export_tools::zoomifyWritesEveryTier` |
| `zf_tile_group_rollover` | Export a layout with at least 257 tiles. | Tile 256 is in `TileGroup1/` and tile 255 in `TileGroup0/`. | `pictura-codec::zoomify_tile_group_rolls_over`, `tst_file_export_tools::zoomifyRollsTileGroups` |
| `zf_edge_tiles` | Export a 300 × 200 tier. | The right-edge tile is 44 × 200 and keeps its full cell index. | `pictura-codec::zoomify_edge_tiles_keep_full_cells`, `tst_file_export_tools::zoomifyEdgeTileDimensions` |
| `zf_white_background` | Export a document with a transparent pixel and read the tile. | A flat transparent area decodes to near-white within the JPEG tolerance. | `tst_file_export_tools::zoomifyTilesFlattenOntoWhite` |
| `zf_viewer_tiles` | Export and inspect the page and folder. | The page references the tile data and base dimensions; the tile files exist. | `pictura-codec::zoomify_viewer_html_references_tiles`, `tst_file_export_tools::zoomifyWritesTilesAndPage` |
| `zf_viewer_offline` | Inspect the generated HTML text. | No absolute URL, remote script, stylesheet, `fetch`, `XMLHttpRequest`, or module import is present. | `pictura-codec::zoomify_viewer_html_is_self_contained` |
| `zf_cancel` | Start a multi-tile export, hold it by the export delay seam, and cancel. | The export stops between tiles and the document and history are unchanged. | `tst_file_export_tools::zoomifyCancelLeavesDocument` |
| `zf_unchanged` | Export successfully and compare the document. | Path, modified state, history, and pixels are unchanged. | `tst_file_export_tools::zoomifyLeavesDocumentUnchanged` |
| `zf_bad_dest` | Point Output Location at a missing or read-only folder and export. | The export reports failure and writes no tile or HTML file. | `tst_file_export_tools::zoomifyRefusesBadDestination` |
| `zf_other_files` | Put an unrelated file in the output folder and export. | The unrelated file is unchanged. | `tst_file_export_tools::zoomifyLeavesUnrelatedFiles` |

## Paths to Illustrator

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `pt_dialog` | Open `File ▸ Export ▸ Paths to Illustrator…`. | Destination is shown; the Format menu defaults to Illustrator 8 (EPS-based) and also offers Illustrator (PDF-based). | `tst_file_export_tools::pathsDialogOffersBothFormats` |
| `pt_suffix` | Confirm a destination named `logo` and then `logo.eps`. | Both write `logo.ai`. | `tst_file_export_tools::pathsAppendsAiSuffix` |
| `pt_dialog_unchanged` | Open the dialog and cancel. | Pixels, path, modified state, and history are unchanged. | `tst_file_export_tools::pathsDialogLeavesDocumentUnchanged` |
| `pt_no_document` | Dispatch the command with no document. | The action is disabled and no dialog opens. | `tst_file_export_tools::exportCommandsNeedADocument` |
| `pt_all_paths` | Write EPS output for a document with a Work Path and two named saved paths. | The file contains all three paths and both recorded saved-path names. | `pictura-codec::illustrator_eps_carries_all_paths`, `tst_file_export_tools::pathsCarriesWorkAndSaved` |
| `pt_name_ascii` | Write output for a saved path named with a non-ASCII character and one with a newline. | The file is ASCII and both names are recorded escaped. | `pictura-codec::illustrator_escapes_non_ascii_names` |
| `pt_open_closed` | Write output for a path with one open and one closed subpath. | The open subpath has no close operator; the closed one does. | `pictura-codec::illustrator_keeps_open_and_closed_subpaths` |
| `pt_handles` | Write output for a curved segment with distinct handles and parse the operators. | Anchors and both handles round-trip within three decimals. | `pictura-codec::illustrator_keeps_cubic_handles` |
| `pt_page_size` | Write both flavours for a 640 × 480 document. | The EPS bounding box and PDF media box are 640 × 480 points. | `pictura-codec::illustrator_page_matches_document` |
| `pt_eps_header` | Write the EPS flavour and inspect the text. | It starts with EPSF-3.0, carries the bounding box and `%%Pages: 1`, and ends with `showpage` and `%%EOF`. | `pictura-codec::illustrator_eps_header_and_trailer` |
| `pt_eps_stroke` | Write the EPS flavour for one path and inspect the operators. | The path construction operators and a stroke operator are present. | `pictura-codec::illustrator_eps_strokes_paths` |
| `pt_deterministic` | Write the same document twice. | The two EPS byte vectors are identical. | `pictura-codec::illustrator_eps_is_deterministic` |
| `pt_pdf_structure` | Write the PDF flavour and parse the page tree and content stream. | One page, the document media box, `m`/`l`/`c` operators, optional `h` for closed subpaths, and a stroke are present. | `pictura-codec::illustrator_pdf_is_one_page` |
| `pt_pdf_no_raster` | Inspect the PDF flavour for image objects. | No image XObject or embedded raster data is present. | `pictura-codec::illustrator_pdf_has_no_raster` |
| `pt_no_paths` | Run the command on a document with no Work Path and no saved paths. | `No paths to export` is reported and no file is created. | `tst_file_export_tools::pathsNoPathsWritesNothing` |
| `pt_atomic` | Attempt to write into a missing or unwritable parent directory. | The export reports failure and the previous file, if any, is unchanged. | `tst_file_export_tools::pathsUnwritableDestinationKeepsOldFile` |
| `pt_unchanged` | Export successfully and compare the document. | Path, modified state, history, and pixels are unchanged. | `tst_file_export_tools::pathsLeavesDocumentUnchanged` |

## Export Layers To Files

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `elf_dialog` | Open `File ▸ Scripts ▸ Export Layers to Files…`. | Destination, prefix, visible-only, File Type, ICC, Cancel, and Export are shown. | `tst_export_layers::layersDialogShowsControls` |
| `elf_defaults` | Open the dialog on `poster.psd`. | Destination is the document folder, prefix is `poster`, PSD is selected, visible-only and ICC are off. | `tst_export_layers::layersDialogDefaults` |
| `elf_format_menu` | Open the File Type menu. | PSD, JPEG, TIFF, BMP, and PDF are offered; no other format is. | `tst_export_layers::layersFormatMenuMatches` |
| `elf_icc_control` | Switch File Type across all five formats. | ICC is enabled for PSD/JPEG/TIFF/PDF and disabled for BMP. | `tst_export_layers::layersIccControlFollowsFormat` |
| `elf_dialog_unchanged` | Open the dialog and cancel. | Pixels, path, modified state, and history are unchanged. | `tst_export_layers::layersDialogLeavesDocumentUnchanged` |
| `elf_no_document` | Dispatch the command with no document. | The action is disabled and no dialog opens. | `tst_file_export_tools::exportCommandsNeedADocument` |
| `elf_one_file_per_layer` | Export a three-pixel-layer document. | Exactly three files are written. | `tst_export_layers::layersWritesOneFilePerLayer` |
| `elf_document_canvas` | Export a layer in a 200 × 100 document as TIFF with the background hidden. | The TIFF is the document size and the untargeted area is transparent. | `tst_export_layers::layersUseDocumentCanvas` |
| `elf_group_children` | Export a document with one group of two pixel layers. | Two files are written, one per child, and no file for the group. | `pictura-app::export_layers_group_children_are_targets`, `tst_export_layers::layersGroupChildren` |
| `elf_group_effects` | Export one child of a group. | The child's file exists and no group file is written. | `tst_export_layers::layersGroupEffectsNotApplied` |
| `elf_adjustment` | Export a document containing an adjustment layer to TIFF, PSD, and PDF. | The TIFF target is fully transparent; the PSD and the PDF editing payload carry the adjustment layer. | `pictura-app::export_layers_adjustment_is_a_target`, `tst_export_layers::layersAdjustmentTarget` |
| `elf_hidden_off` | Export with Visible Layers Only off and a hidden layer. | The hidden layer produces a non-empty file. | `pictura-app::export_layers_hidden_exported_when_off`, `tst_export_layers::layersHiddenLayerRenders` |
| `elf_hidden_on` | Export with Visible Layers Only on. | A hidden layer produces no file. | `pictura-app::export_layers_hidden_skipped_when_on` |
| `elf_hidden_group` | Export with Visible Layers Only on and a visible child in a hidden group. | The child produces no file. | `pictura-app::export_layers_hidden_ancestor_skips_child` |
| `elf_psd_layer` | Export one pixel layer as PSD and reopen it. | The PSD has one layer at its original position and a merged composite for that layer. | `tst_export_layers::layersPsdKeepsSingleLayer` |
| `elf_raster_format` | Export one layer as TIFF and decode it. | The TIFF is the document size with the layer's pixels and transparency elsewhere. | `tst_export_layers::layersTiffDocumentCanvas` |
| `elf_jpeg_white` | Export a layer with transparent pixels as JPEG. | The JPEG is opaque and a flat transparent area is near-white within tolerance. | `tst_export_layers::layersJpegFlattensOntoWhite` |
| `elf_pdf` | Export one layer as PDF. | One page at the physical document size (`pixels × 72 / ppi`) with one page object. | `tst_export_layers::layersPdfSingleLayerPage` |
| `elf_icc` | Export a layer as JPEG with ICC on, and project a CMYK-source scratch document. | The JPEG carries the profile; the CMYK projection trims to three working planes. | `tst_export_layers::layersIccEmbedsWorkingProfile`, `pictura-app::export_layers_cmyk_projection_trims_the_display_alpha` |
| `elf_icc_off` | Export as JPEG with ICC off. | The JPEG is untagged. | `tst_export_layers::layersIccOffWritesUntagged` |
| `elf_name_sanitize` | Export a layer named `a/b:c?` with prefix `Export` as TIFF. | The file is `Exporta_b_c_.tif`. | `pictura-app::export_layers_name_sanitizes` |
| `elf_name_collision` | Export two layers that sanitize to the same name. | The second gets ` 2` before its suffix and neither is overwritten. | `pictura-app::export_layers_name_collisions` |
| `elf_name_empty` | Export a layer whose name and prefix are empty. | The file name starts with `Layer`. | `pictura-app::export_layers_name_falls_back` |
| `elf_name_long` | Export a layer whose assembled name exceeds 255 UTF-8 bytes. | The name is truncated on a character boundary before the suffix and the write succeeds. | `pictura-app::export_layers_name_truncates_utf8` |
| `elf_replace` | Pre-create the computed file and export. | The existing file is replaced with the layer's output. | `tst_export_layers::layersReplaceExistingFiles` |
| `elf_directory_target` | Pre-create a directory with the computed file name and export. | The batch stops, reports the layer and path, and the document is unchanged. | `tst_export_layers::layersDirectoryTargetStopsBatch` |
| `elf_cancel` | Hold a multi-layer export by the export delay seam and cancel. | The batch stops before the next file; written files remain; the document is unchanged. | `tst_export_layers::layersCancelStopsBatch` |
| `elf_bad_dest` | Point Destination at a missing or read-only folder and export. | The export reports failure and writes no file. | `tst_export_layers::layersRefusesBadDestination` |
| `elf_write_failure` | Put a directory at one target and export. | The pre-flight check stops the batch and the document is unchanged. | `tst_export_layers::layersWriteFailureStopsBatch` |
| `elf_unchanged` | Export successfully and compare the document. | Path, modified state, history, and pixels are unchanged. | `tst_export_layers::layersLeavesDocumentUnchanged` |
| `elf_single_layer` | Export a one-Background-layer document. | Exactly one file is written. | `tst_export_layers::layersSingleLayerWritesOneFile` |

## Long-operation progress

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lop_quick_job` | Run a cancellable job under 250 ms through `runCancellableJob`. | The wait cursor is set, no window appears, and the cursor is restored. | `tst_busy_job::quickJobShowsOnlyTheWaitCursor` |
| `lop_window_delay` | Hold a cancellable job past 250 ms by its test seam. | The `jobProgress` window appears with Cancel and timers keep running. | `tst_busy_job::slowJobShowsTheProgressWindow` |
| `lop_esc_cancels` | Press Esc on the visible progress window. | The cancel callback runs. | `tst_busy_job::escCancelsTheJob` |
| `lop_export_pumped` | Run a Zoomify or Export Layers batch through the same window. | Progress is reported, Cancel stops between units, and the document is unchanged. | `tst_file_export_tools::exportProgressShowsTheJobWindow` |

## Manual reviewer checks

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Illustrator opens both path flavours | Export a document with a curved Work Path and a saved path in both formats; open each `.ai` in Illustrator. | Both files open, the paths are editable, and the anchors/handles match the document. | manual: Illustrator is not installed in CI; open the two files by hand |
| Zoomify viewer pans and zooms | Export a document and open `index.html` in a browser from the filesystem. | The image displays at the base size, pans, and zooms through the tile levels with no network requests. | manual: no browser automation in the Qt test harness |
