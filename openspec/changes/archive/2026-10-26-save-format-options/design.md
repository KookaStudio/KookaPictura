# Design

## Context

`saveAsWithDialog` (`frame_menus.cpp`) picks a filter, derives a path, and for
PSD/PSB/JPEG/TIFF shows `SaveOptionsDialog` (Embed Color Profile). Raster saves
then run `PictureView::save` → `write_raster_with_metadata` → `encode_raster` →
`encode_image_rgba`, which builds a `QImage` and writes it through
`QImageWriter` with a fixed quality of 90 and no other options.

`QImageWriter` (Qt 6.11) exposes `setQuality(int)`, `setCompression(int)`,
`setOptimizedWrite(bool)`, and `setProgressiveScanWrite(bool)`. The Qt TIFF
handler treats `compression` as the TIFF Compression tag value; the PNG handler
treats it as the zlib level (0–9); JPEG uses quality and the two write flags.
The BMP handler exposes no options.

## Decisions

### D1 — Separate dialogs, one shared option value

Each format gets its own `QDialog` subclass (`JpegOptionsDialog`,
`PngOptionsDialog`, `BmpOptionsDialog`, `TiffOptionsDialog`) matching the CS6
windows, rather than one dialog with pages: the controls and their gating differ
enough that a single dialog would be a page-switching shell with four unrelated
bodies. They share one plain `SaveFormatOptions` value (a header-only struct)
that carries every field and its JSON (de)serialization, so the session store
and the bridge have one shape to agree on.

### D2 — Options travel on the view

The chosen options are stored on `PictureViewRust` (`save_raster_options`), set
by a `save_format.rs` bridge right before the save and read by
`PictureView::save`, exactly as `save_embed_profile` already works. This keeps
the many `view->save(path)` call sites unchanged and makes "remember the last
choice for this document" fall out for plain Save.

The bridge takes primitives (`quality`, `compression`, `optimized`,
`progressive`, `matte`, `alpha`), not a shared cxx struct: the option set is
small, cxx-qt shared structs would add a generated type to two languages for no
gain, and the PDF path already crosses options as flat values.

### D3 — The encode edge carries the options

`encode_image_rgba` gains `compression`, `optimized`, `progressive`, and
`matte` parameters; `encode_raster` threads them, and `export_image` passes the
format defaults so Export As is unchanged. A new `encode_image_bytes` returns
the encoded bytes without touching the disk, used by the JPEG dialog for the
size estimate and the preview (decode with `QImage::fromData`).

`matte` is packed `0xRRGGBB`, or `-1` for "none" (the current opaque-white
flatten). JPEG and BMP still flatten alpha; TIFF/PNG keep it unless Save
Transparency is off (TIFF), which passes the matte.

### D4 — Quality mapping

CS6's quality is 0–12; Qt's is 0–100. Photoshop's Save As 0–12 is a
high-quality range, but its lowest setting is still well above the libjpeg
floor: on a reference photo it produces a file close to libjpeg quality 60, and
its top is roughly 98. The shape follows the reverse-engineered
Photoshop-to-libjpeg mapping (Raphaël Quinet, GIMP 2.4, 2007), lowered and
stretched at the low end so quality 0 matches the file size a CS6 Save As at 0
produces (`jpegQualityPercent` in `save_format_options.cpp`). This is an
approximation (`ponytail:`): Adobe's exact quantisation tables are unpublished,
and Photoshop's chroma-subsampling switch is not reproduced because
`QImageWriter` cannot set it.

### D5 — Ceilings shown, not hidden

The user chose to show every CS6 control. Controls the encoder cannot honor are
`setEnabled(false)` with a tooltip naming the reason and a `ponytail:` comment at
the construction site. This keeps the dialog a faithful picture of CS6 while
never implying a write that does not happen; the parity register's `WF-1` row
narrows to exactly these controls.

### D6 — Session persistence

`SessionState` gains `QJsonObject saveFormatOptions` keyed by
`jpeg`/`png`/`bmp`/`tiff`, each an object of fields. Absent keys load the CS6
defaults, so no schema bump is needed (the existing additive-key convention).
`PicturaMainWindow` holds the loaded object in a member, updates it when a
dialog is accepted, and `saveSession` writes it back — the same shape as the
swatches and panel state.

### D7 — Preview and size estimate on a debounce

The JPEG dialog encodes the current composite at the chosen settings on a
150 ms `QTimer` debounce (a full-resolution encode per spinbox tick is wasteful
on large documents) and reports the byte count. The Preview checkbox shows the
decoded, scaled result in a pane inside the dialog. CS6 previews on the canvas;
an in-dialog pane is the offscreen-testable approximation and is recorded as the
ceiling (`ponytail:`).

## Alternatives considered

- **One dialog with a `QStackedWidget`.** Rejected (D1): no shared chrome worth
  the indirection.
- **A Rust encoder for the unsupported options** (BMP depth, PNG interlace,
  TIFF byte order). Rejected for this change: it needs new dependencies and
  duplicates work Qt already does for the supported subset; deferred to the
  narrowed `WF-1` row.
- **Passing a shared cxx struct for the options.** Rejected (D2): more generated
  surface than the flat-value convention already in use.
