# Design: App IO, frame, panel and print bugs

## Approach

Guard loading entries in the guide/ruler view state, carry the opened raster path
through `OpenedPath`, make the Save As filter authoritative over a typed suffix,
inject the document's metadata into JPEG/TIFF, persist the `QPrinter` page setup,
quarantine failed recovery sessions, and normalize packed Bitmap extras on write.

Order: #871 (crash) → #874 → #875 → #876 → #873 → #872 → #845.

Risk: the print, metadata, and recovery paths have little automated coverage;
add the tests before changing behaviour.

## Decisions

### #845 — preserve packed extras, expand on fallback

`writes_bitmap` currently refuses any document with extra channels. A Bitmap
document whose composite is unchanged still has a valid packed color plane, so
the writer can emit mode 0 with the extras at the depth-1 stride. When the
composite changed, `write_container` falls back to 8-bit RGB and
`output_depth` returns 8 (the Bitmap read records `source_depth = None`), so the
extras must be expanded from `ceil(width / 8)` bits per row to one byte per
pixel. Both paths are required: the unedited repro keeps the Bitmap mode, and
the edited document must not fail the `width * height` channel check. A Bitmap
document without a merged composite keeps the existing no-image-data behaviour;
the extras ride with the composite, so there is no section to expand them into.

The write path special-cases Bitmap extras like the composite: accept extras only
when every one is at the depth-1 packed stride, validate the packed length and
push the bytes verbatim in Bitmap mode; in RGB fallback, expand only packed
extras with the existing `bitmap_rows_to_rgb` row walker and take one plane, and
pass an already-8-bit extra through unchanged. The read side already splits
packed extras correctly (`split_planes` uses the depth-aware stride).

`write.rs` is at 1188/1200 lines, so the Bitmap plane assembly (packed composite
plus extras, and the fallback expansion) moves into `write_bitmap.rs`; `write.rs`
must not grow past its cap. `codec/psd-color-modes` still states that
"layered or extra-channel Bitmap write-back is out of scope". This change's
`codec/psd-codec` requirement is the narrower new exception; reconciling that
stale clause needs a separate spec-sync change (a MODIFIED delta would copy 21
unchanged scenarios and their test-plan rows).

### #873 — quarantine under the recovery root

`recoverOrphans` discards every orphan session after recovery, even when a
snapshot failed to open. Quarantining inside the orphan session would be deleted
by `offerRecovery`, which discards sessions with no top-level documents. Move the
failed snapshot and manifest to
`$XDG_STATE_HOME/kooka-pictura/recovery/quarantine/<session>/` instead.
`RecoveryStore::orphans()` scans every direct child of the recovery root and both
Recover and Later discard a child with no documents, so it must skip the
`quarantine` directory explicitly; otherwise the next launch deletes the only
copy. The quarantine directory is never passed to `discard`.

Quarantine copies the snapshot and manifest into the quarantine directory, then
removes both from the orphan session only after both copies succeed; on any
failure it removes the partial quarantine copy and reports failure. Remove the
recovered documents' snapshot/manifest from the orphan session, then discard the
session only when no quarantine failed and every recovered document's new
snapshot landed. If a quarantine or that snapshot write fails, keep the session
directory and the recovered orphan copies rather than deleting the only copy. `RecoverableDocument` gains the
manifest path (or derives it from the snapshot's complete base name).

### #875 — the filter wins; metadata is injected after encoding

`saveAsWithDialog` uses the typed suffix whenever one exists. Extract the suffix
decision into `pathForSaveFilter(path, chosenFilter, fallbackFilter)`:
`chosenFilter` is the actual entry index (the All Formats entry included) and
`fallbackFilter` is `defaultSaveFilterIndex(view)`. A single-format filter
replaces the suffix with `defaultSuffixForFilter(chosenFilter)`; the All Formats
entry keeps a typed suffix and appends
`defaultSuffixForFilter(fallbackFilter)` only when the name has none. Known image
suffixes are lowercased. Apply the helper before the PDF and feature-warning
branches so they see the normalized suffix. The recorded document path is the
normalized path. The helper is declared in `dialogs.h`; the Save As test calls it
directly and then saves the returned path, because the native
`getSaveFileName` has no test seam.

Metadata is written by a new Qt-free `pictura-codec` module. It gathers the
document's EXIF (1058/1059), IPTC-IIM (1028), and XMP (1060) resources and
injects them into the encoded JPEG (APP1 Exif, APP1 XMP, APP13 Photoshop IRB) or
TIFF (tags 34665, 33723, 700 with a rebuilt IFD0); an absent resource is skipped
and a document with none writes unchanged. The app encodes the raster to a
temporary file, injects, and renames; a failure leaves the previous file
untouched. PNG/BMP/WebP keep their current drop behaviour; no new dependency is
added. The raster-save branch in `impl_core.rs` is already near its 1200-line
cap; extract the temp/inject/rename helper to `export.rs` if it would exceed it.

### #876 — page setup in the settings struct

`PrintSettings` carries `QPageSize pageSize` and `QPrinter::PaperSource
paperSource` alongside the printer, copies, and orientation. An invalid
`QPageSize()` means the destination's default (Letter for Save as PDF); the
paper-source default is `QPrinter::Auto`. `configurePrinter` applies both to
every job, including Save as PDF. `PrintDialog` keeps the chosen values in
members, uses them for the preview and label, and updates them from the
page-setup dialog. Print Settings is enabled for Save as PDF as well, so the
page size is selectable without a printer; the paper source is stored and applied
to a printer job. A `std::function<bool(QPrinter&)>` page-setup provider replaces
the driver dialog in tests; the default opens `QPrintDialog` for a printer and
`QPageSetupDialog` for Save as PDF, then reads the accepted
`pageLayout().pageSize()` and `paperSource()` back.
`printOneCopyFromView` already passes the last settings through
`configurePrinter`, so it inherits the page setup. The existing
`tst_print::previewAndOrientation` and `saveAsPdf` defaults stay 8.5×11 portrait
by falling back to Letter when the retained page size is invalid.

### #872 — document ppi and active-layer opacity

`formatMeasure` reads `document_ppi(*view_)` (72 when no document) instead of
hard-coding 72. Add `active_layer_opacity()` to the `PictureView` bridge:
resolve the active path, return `-1` when it is the Background or missing, else
the resolved layer's opacity. The Info panel's Opacity readout uses that value
and blanks the row on `-1`; the sampled alpha is no longer used. The existing
`tst_info_panel::colorMenuModes` Opacity assertion runs on a Background today and
must add and select a pixel layer; the Background case moves to the new
`opacityReadoutUsesLayerOpacity` test. The 300 ppi test document is created with
`image_size_apply` (same dimensions, ppi 300) or `create_document`, because the
C++ `newDocument` helper takes no ppi.

### #874 — raster opens record their path

`decode_image` returns `OpenedPath::Set(Some(path))`; `Keep` has no other caller
and its `previous_path` bookkeeping is removed. The existing
`open_jobs::tests::open_image_matches_sync` assertion that a fresh raster open
has no path is updated to assert the path is recorded.

The Rust path alone is not enough: `startOpenImagePath` currently calls
`beginLoadingDocument(view, QString(), …)`, so the C++ `DocEntry::path` (which
backs `documentPath`, `saveActive`, and `revertActive`) stays empty. Pass `path`
there as the PSD path already does, and update the `open_image` bridge doc
comment that says a fresh raster open is untitled. Revert then reloads the same
path in place.

### #871 — skip null canvases

`applyGuideViewState` iterates `docs_`; a loading entry has `canvas` and
`canvasHost` null until `setupCanvas` runs. Guard both pointers (and keep the
toggle state update). The completion path already calls `setupCanvas` →
`prepareCanvasGuides`, so the new canvas receives the current state.

## Constraints

- Follow `docs/dev/testing-conventions.md`: Qt Test for GUI, std `#[test]` for engine.
- Do not weaken or delete an existing test; update a baseline only with a before/after assertion.
- Keep each file under its size cap; split along the seams named above.
- No new dependencies; the metadata containers are written by hand in `pictura-codec`.
