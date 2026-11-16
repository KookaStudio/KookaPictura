# Proposal

## Why

Issue #628 (parity gap WF-4): the docs promise GIF saving through the Indexed
Color dialog and a non-destructive `File > Save a Copy`, but the app offers
neither. Worse, a `.gif` path is silently routed to the PSD codec, so Ctrl+S on
an opened GIF — or typing `cat.gif` into the Save As "All Formats" filter —
rewrites the file with PSD bytes. The seven hand-maintained suffix whitelists
that caused this will break again for the remaining formats tracked by #625.

## What Changes

- Add CompuServe GIF to the Save As / Save a Copy format matrix. A non-Indexed
  document passes through the existing Indexed Color dialog first; a document
  whose retained Indexed source still matches its current pixels writes that
  palette and index plane without re-quantizing; every other document quantizes
  an 8-bit sRGB composite. Bytes come from the existing engine encoder
  (`pictura_codec::encode_gif`) and are written atomically.
- Saving GIF writes a snapshot and never mutates the open document, matching the
  documented save pipeline. The document keeps its mode and depth; only its
  recorded path changes. Palette, colors, dither, and amount follow the existing
  per-format options and session memory. Row order defaults to Normal; the GIF
  Options dialog remains #625.
- Add `File > Save a Copy…`: the same path/format matrix, per-format dialogs, and
  Embed Color Profile dialog as Save As, writing atomically, but leaving the
  document's path, output format, modified state, history, recovery snapshot,
  recent list, tab/window title, and action recorder unchanged. This is the
  documented way to write a GIF without turning the open document into an
  indexed file.
- Replace the duplicated suffix whitelists with one C++ save-format descriptor
  table and one Rust writer dispatch, plus a parity test that fails when a
  filter has no writer. Thread a save intent (`Save`, `SaveAs`, `Copy`) through
  the native, Qt-raster, engine-GIF, and PDF writers; `Copy` skips the
  bookkeeping step.
- Fail closed: a known image extension either has a declared writer or is
  refused, and GIF destinations over 65,535 pixels are refused. `.gif` can no
  longer fall through to the PSD codec.

## Capabilities

### New Capabilities

- `interop/gif-save`: GIF as a Save As and Save a Copy destination, the Indexed
  Color hand-off, source-palette preservation, and engine GIF89a encoding.
- `interop/save-a-copy`: the `File > Save a Copy` command and its non-mutating
  write guarantee across the existing writer families.

### Modified Capabilities

- `document/document-lifecycle`: the Save and Save As requirement admits GIF
  through the engine GIF89a encoder, and the Save As format-choice requirement
  adds `.gif` to the canonical suffix set.
- `interop/raster-export`: the Qt-boundary requirement names GIF Save As as the
  second engine-encoded format alongside WebP, leaving the Qt edge and the
  export formats otherwise unchanged.
- `interop/save-format-options`: the Save As options-dialog requirement adds the
  GIF Options step, which is the Indexed Color dialog specified by
  `interop/gif-save`.

## Impact

- `crates/pictura-app/src/cxxqt_object/impl_core/{save,save_jobs}` and
  `crates/pictura-app/src/cxxqt_object/pdf_export`: writer dispatch, save
  intent, GIF branch, atomic GIF write.
- `crates/pictura-app/cpp`: `import_save_save.cpp`, `import_save_coordinator.*`,
  `save_format_options.*`, `dialogs.*`, `frame_commands.cpp`, `frame.h`, and the
  CMake source and test lists.
- `crates/pictura-script`: the command catalogue row, classification, and modal
  ids; regenerate `command_ids.h` and update the catalogue goldens.
- No new dependency: reuse `pictura_codec::encode_gif` / `quantize` and the
  existing `IndexedColorDialog`.
- `docs/dev/doc-code-parity-gaps.md` WF-4 and the `ROADMAP.md` entry for #628
  move with the change.
