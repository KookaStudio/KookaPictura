# Design

## Context

See `proposal.md` for motivation. The implementation sits on five existing
seams:

- **Menu and command plumbing.** `crates/pictura-script/src/command_catalogue/entries.rs`
  is the single menu table; `commands.cpp` builds actions from it and enables an
  action only when `implemented` is set, a handler is registered, and any
  enabled provider passes. The Zoomify, Data Sets, Render Video, and Export
  Layers rows already exist and are disabled; Paths to Illustrator has no row.
- **Composite and encoders.** `web_export.rs` renders one top-level layer alone
  (cropped and sRGB-normalized), `encode_image.cpp` encodes RGBA through Qt and
  writes atomically with `QSaveFile`, and `pictura-codec` owns the PSD and PDF
  writers.
- **Path model.** `pictura_core::Document` carries `work_path` and
  `saved_paths`; `crates/pictura-codec/src/path_resources.rs` already reads and
  writes the PSD resources, and `svg::path_data` walks the same `Subpath`
  geometry.
- **Progress.** `crates/pictura-app/cpp/busy_job.h` exposes
  `runCancellableJob`, which pumps the event loop, delays the progress window
  250 ms, reports cached progress, and lets Cancel/Esc stop the job between
  units. The change modifies `document/long-operation-progress` so the
  worker-thread rule applies only to jobs that mutate the document.
- **Qt boundary rule.** `interop/raster-export` keeps raster encoding in the
  app; engine crates stay Qt-free.

The Zoomify container and tile naming are an external published format, not a
CS6 Help topic: 256-pixel tiles, 256 tiles per `TileGroup`, `{z}-{x}-{y}.jpg`
names, and an `ImageProperties.xml` descriptor. The format is the oracle for
the layout.

## Goals / Non-Goals

**Goals:**

- Ship the three export commands as one reviewable change without touching the
  compositor, the save path, or the document model.
- Keep format logic pure and unit-testable in Rust, and pixel encoding at the
  Qt boundary.
- Make every export non-mutating, and atomic where a single file is written.
- Bound memory to one base image, one tier, and one output at a time.

**Non-Goals:**

- Data Sets As Files (#622) and Render Video (deferred Extended-only; needs a
  codec backend).
- A native Illustrator 8 object stream with AI private operators; the output is
  a valid EPS or PDF that Illustrator imports as editable stroked paths.
- Custom Zoomify templates or remote viewer assets.
- The shared Save As format-option controls and Targa in Export Layers
  (#587/#625); the change ships the core and marks LAY-9 PARTIAL.
- Importing `.ai` files.

## Decisions

### D1. Three new capabilities, one modified, one change

Each command gets its own capability under `interop/` because each has an
independent external contract:
`interop/export-layers-to-files`, `interop/zoomify-export`, and
`interop/paths-to-illustrator`. They share the command plumbing, the dialog
pattern, and the job helper, so one change and one PR keep the wiring and its
tests together. `document/long-operation-progress` is modified because the
exports are non-mutating and use the pumped progress window instead of a worker
thread. The alternative - three changes plus a separate progress change -
triples the OpenSpec bookkeeping for roughly 100 lines of shared plumbing.

### D2. Zoomify layout in Rust, JPEG encoding in Qt

Add `pictura-codec/src/zoomify.rs`:

- `plan(base_width, base_height) -> Plan` computes the tier count, each tier's
  dimensions, each tile's cell rectangle, and the total tile count.
- `image_properties(&Plan) -> String` emits the `IMAGE_PROPERTIES` element.
- `viewer_html(&Plan) -> String` emits a self-contained page with inline
  dimensions and a small dependency-free tile viewer. The page uses `<img>` tile
  references only; it contains no `fetch`, XHR, or module import, so it works
  from `file://`.

The app renders the composite once at the base size through the Qt boundary,
downscales tier by tier with `QImage::scaled` (`SmoothTransformation`), crops
each planned tile, flattens it onto white, and encodes it with `QImageWriter`
at the chosen quality. The alternative, a whole C++ Zoomify implementation,
would move the tier arithmetic and markup out of unit-test reach; a whole Rust
implementation would put JPEG encoding in an engine crate, against the
raster-export boundary rule.

The app writes directly into the chosen folder and replaces files with the
names it writes; unrelated files are untouched. A destination is probed before
work starts; a mid-run failure stops the export and leaves already-written
tiles in place, which the capability allows. Tiles are generated tier by tier,
and each tile is encoded and dropped before the next, so peak memory is the
base image plus one smaller tier plus one tile.

### D3. A shared pumped progress job for non-mutating exports

Zoomify and Export Layers run under `runCancellableJob`: progress is file or
tile count, Cancel sets a flag checked between units, and the document is never
touched. Paths is a synchronous single-file write with no progress window. The modified `document/long-operation-progress` requirement scopes the
worker-thread rule to jobs that mutate the document; an export has no document
state to roll back, so pumping the event loop is the smaller correct model. The
trade-off is that each individual file is still encoded on the GUI thread; a
batch of very large layers will paint between files, not within one. The
`document/long-operation-progress` worker model stays for document-mutating
commands.

The Zoomify and Export Layers cancel tests need a deterministic seam like the
existing `save_set_delay`: add a test-only export delay/gate so a batch can be
held past 250 ms without a giant fixture.

### D4. Paths to Illustrator: two serializers, stroked geometry

Add `pictura-codec/src/illustrator.rs` with two pure writers:

- `write_eps(width, height, paths) -> Vec<u8>` emits
  `%!PS-Adobe-3.0 EPSF-3.0`, `%%BoundingBox`, `%%Creator`, `%%Title`,
  `%%Pages: 1`, one `newpath ... stroke` block per path, `showpage`, `%%EOF`.
- `write_pdf(width, height, paths) -> Vec<u8>` emits a minimal one-page PDF
  with an uncompressed content stream of `m`, `l`, and `c` operators and, for a
  closed subpath, `h`, followed by `S`.

Both take the Work Path and saved paths with their names, flip y
(`page_height - y`), and format coordinates to three decimals with trailing
zeros trimmed. Path names are recorded as `%%Path: <name>` comments in EPS and
as content-stream comments in PDF; they are not Illustrator layer names. A name
with non-ASCII or control characters is escaped (non-ASCII bytes as `\ooo`
octal, newline/CR/tab as their escapes) so the output stays ASCII and cannot
inject a line. No timestamp or locale is written, so the same document produces
identical bytes.

The paths are **stroked with a 0.25-point hairline** in both formats. An
unpainted PostScript or PDF path is discarded by the interpreter, so a native
AI object stream would be required to keep unpainted objects; that private
format is undocumented and has no oracle. A hairline stroke produces editable
paths in Illustrator and is visible in any renderer. This is a deliberate
behavioral divergence from the CS6 file, which appears empty until Illustrator
switches to outline view; the design records the ceiling rather than claiming
native-AI parity.

The PDF flavour is an extension: CS6 writes one `.ai` flavour, and the docs
leave its encoding unverified. Both writers share the path walk and coordinate
formatter.

Atomicity belongs to the app: the writer returns bytes, and the handler writes
a temporary file with `QSaveFile` and renames it. A path-less document is
refused before the writer runs. The `.ai` suffix is appended when the chosen
destination lacks it.

### D5. Export Layers: a target list, a scratch document, one file at a time

Add a pure target enumerator in the app bridge:

- It walks the document tree depth-first in stored order, skips groups as
  targets, and emits one descriptor per non-group node: path, display name,
  effective visibility (self and ancestors), and kind.
- With Visible Layers Only off, every node is a target; with it on, only nodes
  whose whole ancestor chain is visible are targets.
- Adjustment layers stay targets because the docs say every layer; a raster
  format renders them as a transparent canvas, while PSD and PDF keep the
  adjustment layer in the single-layer document / embedded editing PSD.

For each target the bridge builds an 8-bit scratch `Document` with the source's
`width`, `height`, `mode`, `source_mode`, `document_icc`, and `image_resources`
(the last carries the resolution, resource 1005). Native 16/32-bit stores are
not carried; a 16/32-bit source exports an 8-bit working projection. It holds
only that layer at its original position. The scratch layer is forced
`visible=true` for rendering, even when it was hidden and the target filter
allowed it. The scratch's `composite` is rebuilt from the single layer and
`merged_composite_present=true`; the source composite is never carried, so a
per-layer PSD or embedded PDF editing PSD cannot contain a stale merged image.
A PSD reopen test verifies the one layer and the merged composite. The scratch
document is dropped after the file is written.

Raster and PDF output use a document-sized working-space composite of the
scratch document (not `layer_flattened`, which crops and converts to sRGB),
written through the Save As raster edge with `metadata.icc` set from the
document profile when the option is on, as `impl_core/save.rs` does. A CMYK
source keeps its profile only in PSD/PSB; JPEG, TIFF, and PDF carry the sRGB
working profile rather than the CMYK source profile. For PSD, the writer emits
the scratch document; when ICC is off, the scratch's `document_icc` and ICC
image resource are cleared so the PSD is untagged.

PDF is written from the scratch document and a `PdfEnv`, so Preserve Photoshop
Editing Capabilities embeds exactly the exported layer (and its live text when
present). The page is the document's physical size, pixels × 72 / ppi, matching
the PDF writer's existing convention.

Format defaults are documented and inferred: PSD; JPEG quality 90; TIFF
uncompressed; BMP 24-bit; PDF with the writer's defaults. Include ICC Profile
is offered for PSD, JPEG, TIFF, and PDF and disabled for BMP. The PDF writer's
`icc_profile` option follows the dialog; when off, the PDF is untagged.

Names are assembled as prefix plus sanitized layer name, with collisions
suffixed ` 2`, ` 3`, and so on in traversal order. An existing file with the
computed name is replaced; a directory at that path is a write failure that
stops the batch. The sanitizer and the target walk are pure functions with Rust
unit tests.

The shared Save As format-option controls and Targa are not in this change.
`#587` stays open for that surface, and the parity register marks LAY-9 PARTIAL
rather than SHIPPED.

### D6. Dialog, catalogue, and command wiring

One dialog per command, following the existing `QDialog` pattern:
`ZoomifyDialog`, `ExportPathsDialog`, and `ExportLayersDialog`. Each is
registered in the matching `cmake/sources/*.cmake` fragment. The handlers live
with the other File commands. The catalogue changes are:

- flip `implemented` for `file.export.zoomify` and
  `file.scripts.export.layers.to.files`, and add the `…` to the Export Layers
  label and path leaf;
- add `file.export.paths.to.illustrator` at `File ▸ Export ▸ Paths to
  Illustrator…`;
- add the constants the handlers use to `CONST_NAMES`, update the hard-coded
  catalogue counts in `crates/pictura-script/tests/catalogue.rs`, regenerate
  `command_ids.h` and `tests/golden/catalogue.tsv`;
- add the three ids to `modal_ids` and register `setEnabledProvider` handlers
  so each action is disabled without a document.

`Data Sets As Files` and `Render Video` stay disabled and out of this change.

### D7. Tests

- Rust unit tests: Zoomify tier/tile geometry, group rollover, edge tiles, XML
  attributes, viewer self-containment (including no `fetch`/XHR/import);
  EPS/PDF headers (`%%Pages: 1`), operators, coordinate round-trip,
  open/closed subpaths, ASCII name escaping, determinism; target enumeration,
  visibility, name sanitization, collision suffixes, UTF-8 truncation,
  scratch-document geometry.
- Qt Test suites:
  - `tst_file_export_tools`: Zoomify and Paths dialogs, enablement, Zoomify
    output tree/tile dimensions/XML, EPS/PDF output, the exported job-window
    integration, cancellation, bad destinations, unchanged documents.
  - `tst_export_layers`: dialog defaults, format menu, ICC control, per-layer
    output in every format, hidden-layer rendering, group and adjustment
    behavior, existing-file replacement, directory failure, cancel, unchanged
    document.
  - Run offscreen; outputs go to a temporary directory.
- Manual: open both `.ai` flavours in Illustrator and confirm the paths are
  editable; open the Zoomify `index.html` in a browser and confirm pan/zoom.
  Illustrator is not available in CI, so those checks stay reviewer items.
- A Ghostscript smoke check for the EPS/PDF files may self-skip when
  Ghostscript is absent; it is a parse check, not a golden render.
- JPEG white assertions sample a flat region or use a tolerance; they do not
  require an exact 255 at quality 75.

### D8. Disposition of the bundled features

`Data Sets As Files` remains disabled and blocked on #622; its model, CSV
parser, PSD resources 7000/7001, and apply pipeline are not part of this
change. `Render Video` remains disabled and is recorded as a deferred
Extended-only feature that needs a codec backend (the docs' Linux substitute is
an FFmpeg-style backend or image-sequence output), not as a blanket Linux
non-goal. Targa and the shared Save As option surface in Export Layers are
tracked by #625/#587, and the change marks LAY-9 PARTIAL.

## Risks / Trade-offs

- **Zoomify HTML is our viewer, not Adobe's template** -> the page is
  dependency-free and tested for self-containment; browser pan/zoom is a manual
  check, and custom template loading is explicitly out of scope.
- **AI output is stroked, not a native AI object stream** -> Illustrator treats
  the output as editable stroked paths; the divergence is documented and the
  manual check covers it. No claim of byte parity with CS6.
- **PDF-based `.ai` is an extension** -> it is structurally tested and manually
  opened in Illustrator; the EPS flavour remains the default.
- **Export Layers semantics are partly inferred** (naming, group handling,
  format defaults) -> the target walk, naming, and formats are pinned by tests
  and recorded in `design.md`; changing them later is a spec change.
- **Scratch-document clone and composite cost** -> one layer clone plus one
  document-sized composite is the peak; layers are processed one at a time and
  the scratch document is dropped before the next. A PSB-sized layer is bounded
  by the existing memory governor, not duplicated across the whole batch.
- **GUI-thread encoding between files** -> the modified long-operation
  requirement permits this for non-mutating exports; Cancel is honoured between
  files, and progress is reported per file. A worker-thread batch is a future
  performance change, not a behavioral one.
- **Qt version differences in smooth scaling** -> geometry and dimensions are
  asserted, not tile pixels; no golden image depends on the scaler.
- **Export Layers is PARTIAL** -> the shared option controls and Targa remain
  on #587/#625; the roadmap and parity register say so.

## Migration Plan

No data migration. The change adds commands and files. The implementation PR
updates `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md`, and
`docs/02-ui-ux/menus.md`; closes the core of #587 but leaves #587 open for the
shared option surface and Targa; and leaves #630 open for Data Sets As Files
and the Render Video disposition. The OpenSpec change is archived in the same
PR.
