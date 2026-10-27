# Proposal: rasterize-type-on-canvas

## Why

Issue #962. Rasterize Type painted the text a second time through a Qt
`QFont`/`QPainter` helper rather than keeping the render the canvas shows. That
second render:

- ignored the type transform, so a Free Transformed or moved layer came out at
  its original size in the corner of its rect;
- looked the font up by its PostScript name;
- word-wrapped inside the tight point-type rect;
- painted COLR colour layers. A colour font such as Bitcount Prop Single Ink
  became rainbow tiles and checkerboards.

The bundled fallback used by Rasterize Layer and Rasterize All Layers also laid
the text out at the identity matrix.

The font family pickers skip the per-font preview only for emoji faces (no `A`,
has U+1F600). A colour font with Latin letters is still previewed in its own
face. That costs about 400 ms per repaint and leaves the popup blank, so the
dropdown is slow and empty when the list sits near such a face.

## What Changes

- **Rasterize keeps the on-canvas render.** A type layer that carries its
  rendered planes (every set or re-set type layer, and an opened layer with
  pixels) keeps its rect and colour planes. Only the composite-time text gamma
  is baked into the alpha. A layer without pixels is still painted over its rect
  by the bundled backend, as the compositor draws it.
- **The result paints like any pixel layer.** Rasterizing also clears the type
  layer's transparency and pixel locks and its applied style names and
  overrides.
- **One rasterize path.** Rasterize Type, Rasterize Layer and Rasterize All
  Layers all go through `render_text_layer`. The Qt text helper
  (`render_text_rgba`, `render_text.{cpp,h}`) is deleted, and its self-test
  check `lpr_qt_text` (exit code 525) is retired.
- **Colour faces list without a preview.** The font family pickers detect
  colour faces by their `COLR`, `CBDT`, `sbix` or `SVG ` table, so emoji and
  Latin colour fonts alike list their name in the UI font.
- **Names read in the UI font.** As in CS6's font menu, each row draws the
  family name in the UI font and then a short sample in the face. Thin and
  dotted weights (Bitcount Thin, ExtraLight) no longer list as faint, unreadable
  names.

## Capabilities

### New Capabilities

- `ui/font-family-picker`: names in the UI font with a face sample; colour
  faces list by name without a preview.

### Modified Capabilities

- `tools/text-rasterize-bundled`: the Qt-backend Rasterize Type requirement is
  removed; rasterizing type keeps its on-canvas render.

## Impact

- `crates/pictura-render/src/text_render.rs` and its tests.
- `crates/pictura-app/src/cxxqt_object/impl_layers_rasterize.rs` and
  `cxxqt_object.rs`; `crates/pictura-app/build.rs`; `CMakeLists.txt`.
- `crates/pictura-app/cpp/render_text.{cpp,h}` (deleted),
  `selftest_layers_controls.cpp`, `scripts/selftest-budget.txt` (507).
- `crates/pictura-app/cpp/font_combo.{cpp,h}`.
- Tests in `tst_type_tools` and `tst_paragraph_styles_panel`.
- No dependency or `ROADMAP.md` change.
