# Proposal

## Why

`File ▸ Save As` writes PNG and BMP with no dialog at all, and JPEG and TIFF
through the single `SaveOptionsDialog`, whose only control is Embed Color
Profile. Photoshop CS6 opens a format-specific options dialog for each of these
formats — JPEG Options (Matte, Quality 0–12, Baseline/Baseline
Optimized/Progressive, Scans, Preview, file size), PNG Options (Compression,
Interlace), BMP Options (File Format, Depth, RLE, Flip Row Order, Advanced
Modes), and TIFF Options (Image Compression, Pixel Order, Byte Order, Layer
Compression, Save Image Pyramid, Save Transparency) — and remembers the last
settings per format. Without them the user cannot choose JPEG quality or format,
PNG compression, or TIFF compression, and the format options the encoder already
supports (via `QImageWriter::setQuality`, `setCompression`, `setOptimizedWrite`,
`setProgressiveScanWrite`) never reach the file. The parity register records this
as `WF-1` (`docs/10-workflow-io/save-and-save-as.md`).

## What Changes

- **Per-format options dialogs.** Four new dialogs — `JpegOptionsDialog`,
  `PngOptionsDialog`, `BmpOptionsDialog`, `TiffOptionsDialog` — presented by
  `saveAsWithDialog` for their suffix, each laying out the CS6 controls. The
  existing Embed Color Profile dialog is unchanged and still shown for
  PSD/PSB/JPEG/TIFF.
- **Honored options.** The encode edge learns JPEG quality/format (baseline,
  baseline optimized, progressive), PNG compression, TIFF compression, and TIFF
  Save Transparency, all through `QImageWriter`.
- **Explicit ceilings.** Controls the Qt writer cannot honor — PNG Interlace,
  the BMP controls, TIFF Pixel/Byte Order, Save Image Pyramid, and Layer
  Compression — are shown disabled with a tooltip and a `ponytail:` note, so the
  dialog matches CS6 without claiming a write that does not happen. The JPEG
  Matte is shown disabled too, so a transparent pixel flattens onto white.
- **Preview and file-size estimate.** The JPEG dialog encodes the current
  composite to memory on a debounce to show the file-size readout, and its
  Preview control shows the encoded result; neither touches the document.
- **Remembered settings.** The last accepted settings per format persist in the
  session store and seed the next dialog, as CS6 remembers them.

## Capabilities

### New Capabilities

- `interop/save-format-options`: the per-format options dialogs, their CS6
  control sets, the options the encoder honors, the JPEG preview and file-size
  estimate, and the remembered last-used settings.

## Impact

- **App (C++):** four new dialog classes under `crates/pictura-app/cpp/`,
  `saveAsWithDialog` dispatch, `encode_image.{h,cpp}` option parameters, and the
  session store gains a `saveFormatOptions` object.
- **App (Rust):** a new `save_format.rs` bridge storing the chosen options on
  the view, the save path reading them, and a preview/size encode returning
  bytes; `impl_core/save.rs` and `export.rs` thread the options to the boundary.
- **Tests:** a new Qt Test suite `tst_save_format_options` (dialog controls,
  defaults, dispatch, honored options parsed from the written bytes, size
  estimate, preview, session round-trip) and Rust units for the option defaults
  and the boundary mapping.
- **Docs:** the parity register's `WF-1` row narrows to the options still not
  honored, and `ROADMAP.md` links the remaining work.
- **Dependencies/assets:** none. Every honored option goes through the existing
  Qt image writers.

## Deferred

The BMP controls, PNG Interlace, TIFF Pixel Order, Byte Order, Save Image
Pyramid, and Layer Compression remain shown-but-disabled until an encoder
implements them (tracked by the narrowed `WF-1` row). Layered TIFF and PNG
interlace are not attempted here.
