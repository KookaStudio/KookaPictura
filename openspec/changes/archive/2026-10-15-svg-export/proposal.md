# Proposal: svg-export

## Why

Issue #459. `File ▸ Export As…` (#457) offers PNG / JPG / PNG-8 / GIF; Photoshop
CC also offers **SVG**. No SVG writer exists in the tree. CC's SVG export is
**not** a traced raster: it walks the document's vector content and emits real
SVG primitives, embedding raster and effect content as a base64 PNG fallback.

A shipped CC sample (`~/Downloads/horse-3svg.svg`, a 1000×652 raster document)
establishes the raster case exactly: one `<image>` whose `xlink:href` is a
`data:img/png;base64,…` PNG at the document size, under a header of
`xmlns` / `xmlns:xlink` / `preserveAspectRatio="xMidYMid"` / `width` / `height` /
`viewBox`, two-space indentation, and a trailing newline. This change makes that
exact shape the baseline and adds vector primitives on top.

## What Changes

- New capability `interop/svg-export`: `File ▸ Export As…` gains `SVG` in its
  Format list. File Settings shows no raster-only controls for SVG (no
  Transparency / Quality / PNG-8 / GIF settings), matching CC.
- `pictura-codec::svg` (new engine module, Qt-free): serializes a document to
  SVG text. It emits the CC header verbatim, walks top-level layers bottom-to-top
  (skipping hidden and zero-opacity layers), and for each layer emits either an
  SVG primitive (vector content) or an embedded base64 PNG `<image>` (pixel,
  smart-object, effect, and adjustment content).
- A pure path serializer turns a `pictura_core::path::Subpath` into an SVG `d`
  string (`M` / `L` / `C` / `Z`), and live-shape `LiveShape`s into `<rect>` /
  `<rect rx ry>` / `<ellipse>`. `Subpath::segment` already yields the cubic
  control points.
- Layer `opacity` and the CSS-representable blend modes map to a wrapping `<g>`
  (`opacity`, `mix-blend-mode`); the rest fall back to a rasterized layer.
- The app bridge (`crates/pictura-app/src/cxxqt_object/`) gains an `svg_*` edge;
  `cpp/export_as_dialog.cpp` dispatches the `SVG` format to it (SVG is text, so
  it cannot use the `QImageWriter` path).

## Capabilities

### New Capabilities

- `interop/svg-export`: the SVG output format — its CC header, the per-layer
  vector-vs-raster decision, the path / primitive serialization, the raster
  fallback, and its Export As integration.

## Impact

- `crates/pictura-codec/src/svg.rs` (new) and `lib.rs` (module + re-exports).
- `crates/pictura-app/src/cxxqt_object/` — a new `svg_export.rs` bridge that
  supplies per-layer rasters (composited with `pictura-render`) and calls the
  codec writer.
- `crates/pictura-app/cpp/export_as_dialog.{h,cpp}` — `SVG` format entry, its
  `.svg` suffix, a text write path (not `QImageWriter`), and the File Settings
  gating.
- `docs/10-workflow-io/export-formats.md` — an SVG section and provenance
  (`TASK-ALLOWS-DOCS`).
- Tests: Rust unit tests for the `d` serializer and each primitive; a Qt Test
  (`tst_export_as`) case for a well-formed `.svg`; an oracle that renders the
  SVG and compares pixels against the CPU composite, self-skipping when no
  renderer is installed. No new engine dependency.

## Provenance

No Adobe source. The raster case is taken from the shipped CC sample named
above; the vector primitives, blend mapping, and text handling are inferred and
marked with `ponytail:` ceilings. Any SVG renderer used in tests is an oracle
only, never in the shipped path.
