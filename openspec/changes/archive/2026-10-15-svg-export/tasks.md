# Tasks: svg-export

## 0. Decisions (blockers on #459)

- [x] 0.1 Resolve the `vscg` fill gap: the decoder already shipped in
  `pictura-render`; wire it into the shape model and surface it (D1b).
- [x] 0.2 Confirm the writer split: pure serializer in `pictura-codec::svg`, the
  layer→content decode in `pictura-render::layer_content`, the compositor and
  `vscg` decode in render (D1, avoids the `codec → render` cycle).
- [x] 0.3 Confirm text handling: the raster fallback (D6, decided on #477).
- [x] 0.4 Pick the oracle renderer for CI: ImageMagick's `RSVG:` coder
  (librsvg), already in the `oracles` job; self-skip when absent (D8).
- [x] 0.5 Raster-fallback PNG encoding: a minimal Rust PNG writer in
  `pictura-codec::png` (D1c); no Qt round-trip, no new dependency.

## 1. Contract

- [x] 1.1 `docs/10-workflow-io/export-formats.md`: add an **SVG (CC)** section —
  the header, the vector-vs-raster decision, the raster fallback, and provenance
  (`TASK-ALLOWS-DOCS`).

## 2. Engine

- [x] 2.1 `crates/pictura-codec/src/svg.rs`: `SvgOptions` / `SvgContent`,
  `encode_svg`, the CC header, XML escaping, and the element builders.
- [x] 2.2 Path serializer: `Subpath` → `d` (`M` / `C` / `Z`), with fixed-precision
  coordinate formatting.
- [x] 2.3 Primitives: `rect`, rounded `rect`, `ellipse`, `path`, `image`,
  `text`, `group` (opacity / `mix-blend-mode`), and the fill attributes.
- [x] 2.4 Layer walk: top-level bottom-to-top, skip hidden / zero-opacity, shape
  primitive vs supplied raster fallback; a vector mask emits a `<clipPath>`.
- [x] 2.5 `lib.rs`: `mod svg;` and the re-exports.
- [x] 2.6 Rust `#[test]`: `d` serializer, each primitive, XML escaping, the
  raster-only document byte-for-byte against the CC header, layer skipping, the
  opacity `<g>`, the vector-mask clip path, a filled shape path, and the live
  primitives.
- [x] 2.7 `pictura-render`: broaden `is_shape_layer` / `shape_fill_color` to the
  `vscg` decode (D1b); surface `decode_layer_fill` / `decode_vector_fill`; add
  `pictura_render::layer_content` and its tests.
- [x] 2.8 `pictura-codec::png`: `encode_rgba` / `encode_base64` (CRC-32, base64,
  `flate2` zlib) with tests (header, IDAT round-trip, base64 vectors, CRC check).

## 3. Bridge

- [x] 3.1 `crates/pictura-app/src/cxxqt_object/svg_export.rs`: `svg_from_view`
  builds each top-level layer's content (composite each raster to document-sized
  RGBA, then `pictura_render::layer_content`) and calls `encode_svg`; registered
  in `cxxqt_object.rs` and `build.rs`.
- [x] 3.2 Reuse `pictura_render::composite_active` for the raster bytes; no new
  compositor.

## 4. Dialog

- [x] 4.1 `export_as_dialog.{h,cpp}`: `SVG` format entry (`Format::Svg`), `.svg`
  suffix, `formatForPath`/`retargetPath` accept `.svg`, `refreshFormatControls`
  disables Transparency/Quality, and `exportAsFromView` writes the document SVG
  through `svg_from_view` with `QSaveFile` (SVG is document-level, so it does not
  use the per-entry `QImage` encode path).
- [x] 4.2 `tst_export_as`: the format list has 5 entries and SVG gates the raster
  controls; `svgExport` writes a well-formed, non-empty `.svg` for a document.

## 5. Oracle

- [x] 5.1 `crates/pictura-render/tests/svg_oracle.rs`: librsvg renders a
  vector-only document's SVG and it matches the CPU composite within tolerance;
  self-skips when the renderer is absent.

## 6. Structural parity (#477, D10)

- [x] 6.1 Codec: `SvgNode` tree with `SvgContent::Group`; document-unique clip
  ids; isolated groups carry `isolation:isolate`.
- [x] 6.2 Render: `svg_nodes` / `encode_document_svg` walk groups, clipping
  stacks, adjustment layers, and non-CSS blends; a raster fallback no longer
  bakes in its opacity or blend; shapes honor fill opacity and rasterize with
  effects or a pixel mask. Unit tests per rule.
- [x] 6.3 Bridge: `svg_from_view` calls `encode_document_svg` with the active
  compositor.
- [x] 6.4 Follow-up issue (#652) for gradient/pattern `<defs>`, `vstk` strokes, and
  per-layer SVG export.

## 7. Verification

- [x] 7.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo nextest run -p pictura-codec` (or workspace), and
  `cargo test --workspace --doc`.
- [x] 7.2 CMake build + `ctest --test-dir build -R '^tst_'`.
- [x] 7.3 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh`; `openspec validate
  --all --strict`.
