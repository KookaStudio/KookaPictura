# Proposal: save-for-web-slices

## Why

Issue #629 (WF-5 in the code-parity register): Save for Web ships palette
reduction, GIF/WBMP encoders, and Original / Optimized / 2-Up previews, but has
no slice export, no HTML/CSS emitter, no 4-Up comparison, no colour-table
editor, and no metadata choice. A sliced document cannot be reassembled on the
web, and users cannot inspect or preserve what the optimiser does.

## What Changes

- **Slice export**: when the document has one or more user slices, Save writes
  one image per export cell (user slices clipped into a non-overlapping grid,
  filled out by auto slices) into an `images/` subfolder as `<document>_NN.<ext>`,
  cropped from the Image Size-scaled composite.
- **HTML/CSS emitter**: a Qt-free engine writer emits a table that reassembles
  the slice grid (`rowspan`/`colspan`, image paths) or the equivalent absolutely
  positioned CSS layout.
- **Save Optimized As**: a small dialog after Save with Format = HTML and
  Images / Images Only / HTML Only, Slices = All Slices, and a Table/CSS layout
  choice. Without user slices the existing single-image save is unchanged.
- **4-Up**: the disabled tab becomes a 2x2 comparison: top-left original, three
  optimised panes, each with its own settings; clicking a pane targets the
  options; Repopulate Views regenerates the other optimised panes. 2-Up already
  ships and stays.
- **Colour-table editor** for GIF/PNG-8: swatch display and selection, shift
  (edit) a colour, add a locked colour, delete, sort by hue/luminance/
  popularity, map/unmap a colour to transparency, and load/save `.act`.
- **Metadata menu**: None (default) / Copyright / Copyright and Contact Info /
  All Except Camera Info / All, remembered for the session. JPEG carries the
  selected blocks (None keeps only a minimal EXIF copyright notice when one
  exists); PNG-8 and PNG-24 carry XMP in an `iTXt` chunk; GIF and WBMP carry
  none.
- **Engine support**: custom and locked palettes in `quantize`, the `.act` file
  format, metadata level filtering, and PNG XMP insertion.

Explicit non-goals (tracked by #543 unless noted): slice names, URLs, target,
alt text, Slice Options, per-slice optimization settings, Selected Slices,
link/unlink slices, Divide/Combine, `.aco` tables, Web Snap, and saved output
settings presets.

## Capabilities

### New Capabilities

- `interop/web-slice-export`: writing one image per resolved slice and emitting
  the HTML table or CSS layout that reassembles them, plus the Save Optimized
  As format choice.

### Modified Capabilities

- `interop/save-for-web`: the dialog now offers 4-Up with per-pane settings, a
  GIF/PNG-8 colour-table editor, and a metadata menu; the encoders accept custom
  and locked palettes. The existing dialog and encoder requirements change, so
  the capability needs a delta.

## Impact

- `crates/pictura-codec`: new `web::html` and `web::act` modules; palette
  options in `web::quantize`; metadata level filtering in `web::sfw_metadata`;
  PNG `iTXt` insertion; Rust unit tests.
- `crates/pictura-render`: `resolve_export_slices` clips overlapping user slices
  into disjoint export cells for export only; `resolve_slices` keeps its
  overlap behaviour for the canvas overlay.
- `crates/pictura-app`: `web_export` bridge additions; the dialog splits into
  settings, colour-table, and save-optimized units; new sources registered in
  `cmake/sources/documents.cmake`; extended `tst_save_for_web`.
- `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md` (WF-5) and
  `docs/10-workflow-io/web-export-and-slices.md` updated in a separate `docs:`
  commit with `TASK-ALLOWS-DOCS`.
- No new dependencies. No public PSD or project-file format change; slices stay
  rect-only in `Document::slices`, and resource 1050 remains preserved verbatim.
