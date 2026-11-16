# Proposal

## Why

Issue #630 tracks the export commands that are still inert: the catalogue rows
for `File ▸ Export ▸ Zoomify…` and `File ▸ Scripts ▸ Export Layers to Files…`
exist but are disabled, and `File ▸ Export ▸ Paths to Illustrator…` is absent
from the catalogue entirely. The parity register records the same gaps as
`WF-6` and `LAY-9`. These three commands are independent, self-contained
features that reuse the existing composite, encoder, PSD/PDF writer, and path
resource surfaces. The other two commands bundled into #630 are not ready:
`Data Sets As Files` needs the Variables engine from #622, and `Render Video`
is Extended-only and needs a codec backend before it can be planned.

## What Changes

- **Zoomify export.** `File ▸ Export ▸ Zoomify…` opens a dialog for output
  folder, browser base size, and tile JPEG quality, then writes a Zoomify tile
  pyramid (`TileGroup{N}/{z}-{x}-{y}.jpg`, 256-pixel tiles, 256 tiles per
  group), `ImageProperties.xml`, and a self-contained HTML viewer page.
- **Paths to Illustrator.** `File ▸ Export ▸ Paths to Illustrator…` writes the
  document's Work Path and saved paths to a `.ai` file. The dialog offers two
  serializers: the CS6-era Illustrator 8 EPS-based file (default) and a
  PDF-based `.ai` extension. Both preserve subpaths, open/closed state,
  anchors, and cubic handles, and record the saved path names in an ASCII-safe
  comment.
- **Export Layers to Files.** `File ▸ Scripts ▸ Export Layers to Files…` opens
  a batch dialog (destination, prefix, visible-only, file type, ICC) and writes
  one file per layer, each layer rendered alone on a document-sized
  transparency canvas, in PSD, JPEG, TIFF, BMP, or PDF. This ships the core of
  #587; #587 stays open for the shared Save As format-option surface and Targa
  (#625).
- **Progress.** Zoomify and Export Layers use the existing modal progress
  window; Paths is a synchronous single-file write. `document/long-operation-progress`
  is modified to scope its worker-thread rule to document-mutating jobs, because
  an export has no document state to roll back.
- **Catalogue wiring.** The existing disabled rows become implemented; a new
  `File ▸ Export ▸ Paths to Illustrator…` row is added and the generated
  `command_ids.h` and catalogue golden are updated.
- **Roadmap and parity record.** `ROADMAP.md` and
  `docs/dev/doc-code-parity-gaps.md` mark Zoomify and Paths to Illustrator as
  shipped and Export Layers as PARTIAL; `docs/02-ui-ux/menus.md` lists the new
  Paths entry. `Data Sets As Files` stays on #622 and `Render Video` is
  recorded as deferred Extended-only.

No **BREAKING** changes: all three commands are new surfaces and leave the open
document's pixels, path, modified state, and history untouched.

## Capabilities

### New Capabilities

- `interop/export-layers-to-files`: the batch dialog and per-layer export to
  PSD/JPEG/TIFF/BMP/PDF, including layer traversal, naming, visibility,
  cancellation, and non-mutation.
- `interop/zoomify-export`: the Zoomify dialog, tile-pyramid geometry,
  `ImageProperties.xml`, JPEG tile groups, and the HTML viewer page.
- `interop/paths-to-illustrator`: the Export Paths command, the EPS-based
  Illustrator 8 serializer, the PDF-based serializer, coordinate preservation,
  ASCII-safe path names, and the no-path refusal.

### Modified Capabilities

- `document/long-operation-progress`: the worker-thread rule applies to
  document-mutating jobs; a non-mutating export may report progress through
  the modal progress window while pumping the event loop.

## Impact

- `pictura-script`: three catalogue rows become implemented, one row is added,
  and `tests/golden/catalogue.tsv` is regenerated.
- `pictura-codec`: a Zoomify layout/XML/HTML module, an EPS-based AI writer, a
  minimal PDF-based AI writer, and a single-layer document projection helper.
- `pictura-app`: three dialogs, command handlers, generated `command_ids.h`,
  menu wiring, a cancellable layer batch, the modal ids and enabled providers,
  and Qt Test suites.
- `docs/`: `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md` rows WF-6/LAY-9,
  and `docs/02-ui-ux/menus.md` (the docs commit carries `TASK-ALLOWS-DOCS`).
- No new dependencies. No engine renders differently; the composite and save
  paths are reused as-is.
- Out of scope and recorded in `design.md`: `Data Sets As Files` (#622),
  `Render Video` (deferred Extended-only, codec backend needed), Targa and the
  shared Save As option surface in Export Layers (#587/#625), custom Zoomify
  templates, and importing `.ai` files.
- #587 stays open for the shared option surface and Targa. #630 stays open for
  Data Sets As Files and the Render Video disposition.
