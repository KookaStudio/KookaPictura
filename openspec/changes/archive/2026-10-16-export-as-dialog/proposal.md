# Proposal: export-as-dialog

## Why

Issue #457: `File ▸ Export As…` currently opens a bare format / quality / scale
dialog and then a file picker (`crates/pictura-app/cpp/export_as_dialog.cpp`).
Photoshop CC replaced this with a full **Export As** dialog: a layer list, a live
preview with zoom and an output-size readout, **File Settings** (Format +
Transparency / Quality), **Image Size** (width / height / scale + a **Resample**
menu), **Canvas Size** (width / height + **Reset**), and `Cancel` / `Export…`.
The repo's contract (`interop/raster-export`) predates that dialog, and the CS6
corpus has no "Export As" surface at all (CS6 used Save for Web), so this change
brings the CC dialog into scope as an explicit CC-only extension and wires the
PNG-8 and GIF encoders that already exist (`pictura_codec::web`) into it.

**SVG is deferred** to issue #459; the Format list here is PNG, JPG, PNG-8, GIF.

## What Changes

- `ExportAsDialog` SHALL be the CC-style dialog: a left **layer list** with a
  checkbox per row (the document plus each top-level layer), a centered preview
  with zoom controls, a **File Settings** section (Format, plus Transparency for
  the alpha formats and Quality for JPEG), an **Image Size** section (width,
  height, scale, Resample), a **Canvas Size** section (width, height, Reset), and
  `Cancel` / `Export…`.
- Selecting a list row SHALL preview that row; the checked rows SHALL be the
  export targets. One checked row SHALL prompt for a file; several SHALL prompt
  for a folder and write one file per row (the document as `name.ext`, a layer as
  `name-layer.ext`).
- The Format list SHALL be PNG, JPG, PNG-8, GIF. PNG-8 and GIF SHALL reuse the
  engine `pictura_codec::web` quantizer and GIF encoder through the existing
  `web_export` bridge; the rest SHALL use the Qt encode edge.
- **Image Size** SHALL resample the target with the chosen Resample method
  (Bicubic, Bicubic Automatic, Bicubic Sharper, Bicubic Smoother, Bilinear,
  Nearest Neighbor, Preserve Details). **Canvas Size** SHALL crop or pad the
  output canvas; **Reset** SHALL restore it to the image size.
- The dialog SHALL show an output-size estimate for a small document and SHALL
  omit it above a pixel cap (a full-resolution encode is too costly to run on
  every change; measured ~1.2 s for 12 MP).
- `interop/raster-export` SHALL be amended: the Export As requirement is
  rewritten around the dialog and the per-layer targets; a requirement for the
  PNG-8 / GIF formats is added.

## Capabilities

### New Capabilities

- `ui/export-as-dialog`: the CC Export As dialog surface — the layer list, the
  preview and zoom, File Settings, Image Size and Resample, Canvas Size, and the
  capped output-size estimate.

### Modified Capabilities

- `interop/raster-export`: Export As is rewritten from a single format/quality/
  scale dialog to the CC dialog with per-layer targets, and gains PNG-8 / GIF.

## Impact

- `crates/pictura-app/cpp/export_as_dialog.{h,cpp}` — the dialog rewrite and the
  per-layer export flow.
- `crates/pictura-app/cpp/encode_image.{h,cpp}` — no change; the Qt edge stays.
- `crates/pictura-app/src/cxxqt_object/export.rs` and `web_export.rs` — a
  per-entry flatten edge for a single layer, and the reuse of the web encoders.
- `crates/pictura-app/cpp/panels/layers_panel_menu.cpp` — unchanged (still calls
  `exportAsFromView`).
- `docs/10-workflow-io/export-formats.md` — an Export As (CC) section and
  provenance (`TASK-ALLOWS-DOCS`).
- Tests: retire the `ExportAsDialog` assertions from the C++ self-test check
  `lpr_raster_export` (code 532) and add the Qt Test suite `tst_export_as`.
  No new dependency.

## Provenance

Photoshop CC's Export As dialog is a CC-era surface (introduced after CS6); the
CS6 Help corpus does not document it. Behavior is taken from the shipped dialog
(the issue's screenshots) and is otherwise inferred; it is a CC-only scope
extension recorded in `docs/10-workflow-io/export-formats.md`. No pixel oracle.
