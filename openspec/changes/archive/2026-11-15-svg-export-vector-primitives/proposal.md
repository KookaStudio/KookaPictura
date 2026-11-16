# Proposal: svg-export-vector-primitives

## Why

Issue #652, the follow-up to #477. SVG export already walks the layer tree,
emits solid shape primitives, and falls back to a base64 PNG for everything
else. Four pieces were split out:

- gradient and pattern shape fills still rasterize instead of using
  `<linearGradient>` / `<radialGradient>` / `<pattern>`;
- a shape's stroke is an `lfx2` effect, so a stroked shape rasterizes whole
  instead of emitting `stroke` attributes;
- the `<path>` fill rule and the `mix-blend-mode` value set have never been
  confirmed against a CC sample;
- `File ▸ Export As…` ignores the checked rows for SVG and writes one
  document-level file.

The last item is the largest parity gap: every other Export As format writes one
file per checked row, but SVG silently exports the whole document.

## What Changes

- New SVG paint-server output: a shape or fill layer with a decoded linear or
  radial gradient emits a `<linearGradient>` / `<radialGradient>` in `<defs>`
  and a `fill="url(#…)"`; a pattern fill emits a `<pattern>` whose tile is the
  pattern pixels nearest-neighbour scaled to the fill's tile size. Unsupported
  gradient kinds (angle, reflected, diamond) and degenerate layer rectangles
  keep the raster fallback.
- Solid vector strokes: a shape whose only active effect is an enabled,
  Normal-blend, solid-colour stroke emits `stroke`, `stroke-width`, and
  `stroke-opacity`. Center strokes use the centerline; inside/outside strokes
  double the width and mask the result to the shape's interior/exterior.
  Gradient/pattern strokes, non-Normal stroke blends, and any other active
  effect keep the raster fallback.
- Per-layer SVG export: each checked Export As row writes its own
  document-sized `.svg`. The document row is the whole document; a layer row is
  that top-level layer alone (including its group subtree). One checked row
  prompts for a file; several prompt for a folder and write one file each.
- The fill-rule and `mix-blend-mode` confirmation is recorded as a blocked
  research item: the repository still has no CC sample with vector content, so
  the existing inferred even-odd rule and CSS blend map are kept and marked.

## Capabilities

### Modified Capabilities

- `interop/svg-export`: add gradient/pattern paint servers, vector strokes, the
  per-layer Export As file flow, and the matching oracle coverage.

## Impact

- `crates/pictura-codec/src/svg.rs`: paint-server and stroke types, `<defs>`
  emission, stroke masks, and shape serialization.
- `crates/pictura-render/src/svg.rs` and `svg/tests.rs`: decode the fill and
  stroke into the new codec types, including the compositor's gradient and
  pattern geometry.
- `crates/pictura-app/src/cxxqt_object/svg_export/imp.rs` and
  `decode_bridge.rs`: a `svg_from_layer` bridge for a top-level layer.
- `crates/pictura-app/cpp/export_as_dialog.{h,cpp}`: per-row SVG target
  selection and writing.
- `crates/pictura-app/cpp/tests/tst_export_as.cpp`: per-layer SVG coverage.
- `crates/pictura-render/tests/svg_oracle.rs`: librsvg oracle coverage for
  gradients, patterns, and strokes.
- `docs/10-workflow-io/export-formats.md` and `docs/dev/STATE.md`
  (`TASK-ALLOWS-DOCS`).
- `ROADMAP.md` is already ticked for SVG export; no shipped/removed feature
  change.

## Provenance

No Adobe source. There is still no CC sample with vector content, so the paint
server geometry, the stroke band model, and the per-layer canvas choice are
inferred from the engine's compositor and public SVG 1.1 semantics. The librsvg
oracle compares pixels, never bytes, and is the only parity claim. The
`<path>` fill rule stays even-odd and the `mix-blend-mode` map stays as shipped
until a CC vector sample is available.
