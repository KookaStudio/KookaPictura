# Proposal: App IO, frame, panel and print bugs

## Why

The audit found a crash while a document loads, stale panel readouts, a destructive partial recovery, missing raster paths on Open/Save, and a discarded print page setup. These live in the C++ shell and the save/open/print paths around the codec.

## What changes

- #845 — a Bitmap PSD with an extra channel saves. An unchanged Bitmap keeps its packed channel; an edited one falls back to 8-bit RGB and expands the channel to one sample per pixel.
- #871 — rulers/guides toggles skip loading entries instead of dereferencing a null canvas, and the state applies when the open completes.
- #872 — Info panel physical units use document ppi; Opacity reads the active layer and is blank for the Background.
- #873 — a partial recovery quarantines the snapshots of documents that failed to open; quarantined snapshots are not offered again.
- #874 — opening a raster records its source path, so Save writes in place and Revert stays available.
- #875 — Save As honours the chosen Format (the typed suffix is rewritten) and writes EXIF/IPTC/XMP into JPEG and TIFF.
- #876 — Print Settings page size and paper source reach the preview and the job, and the dialog is testable without a printer.

## Capabilities

- Modified: `codec/psd-codec`
- Modified: `ui/rulers-and-guides`
- Modified: `ui/info-histogram-panel`
- Modified: `document/crash-recovery`
- Modified: `document/document-lifecycle`
- Modified: `interop/raster-export`
- Modified: `interop/print`

## Impact

`crates/pictura-app/cpp/{frame_open,frame_guides,frame_recovery,recovery_store,frame_menus,dialogs,print_dialog}`, `crates/pictura-app/cpp/panels/info_panel.cpp`, `crates/pictura-app/src/cxxqt_object/{open_jobs,impl_core,impl_layers}` and `crates/pictura-codec` for the Bitmap channel normalization and the raster metadata writer.

## Issues

Closes #845, #871, #872, #873, #874, #875, #876.
