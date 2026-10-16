# Proposal: horizontal-type-tool

## Why

The Horizontal Type tool (issue #37) was catalogued but disabled, and nothing
could create a type layer: Kooka read and rendered `TySh` but never authored
one. photorust's `core/src/psd/text_write.rs` writes the block and its shell's
type entry drives it; the port follows `docs/03-tools/type-tools.md`.

## What Changes

- `pictura_core::TypeSpec` (new): a point-type string (`\r` between lines),
  family, pixel size, RGBA colour, justification, orientation, anti-aliasing,
  and the click. `TypeTool` gains `vertical` (the text descriptor's `Ornt`).
- `pictura_codec::author_type_tool` (new module `type_write.rs`): the `TySh`
  view for a spec — transform at the click, a `TxLr` text descriptor (`Txt `,
  `Ornt`, `AntA`, bounds, a complete `EngineData` dump with run lengths that
  cover the string), and a "no warp" descriptor. `Txt ` is read back without
  Photoshop's trailing NUL.
- `pictura_render` (new module `type_layer.rs`): `type_placement` (the layer
  rect from the click; lines `1.2 × size` apart, the click on the first
  baseline, moved by Left / Center / Right), `render_type`, and
  `add_type_layer` (rendered pixels plus the authored `TySh`, CS6's type locks,
  named after its first line; 8-bit RGB only). `type_layer_at`,
  `type_layer_spec` (the origin follows a moved layer), and
  `replace_type_layer` reopen and re-set an existing type layer in place.
  `TypeSpec::matrix` carries the `TySh` transform's linear part: Free
  Transform and Skew of a type layer (`transform_layer` /
  `transform_layer_quad`, affine quads only) fold the map into it through
  `transform_type_layer` and re-set the text, each glyph's outline mapped by
  the matrix (`BundledRasterizer::rasterize_mapped`), so transformed type stays
  sharp and editable instead of being resampled.
- Fixes the read side needed: `TextStyle::fill_color` is EngineData's `Values`
  order (alpha, red, green, blue) and is now rendered as such (`rgba()`;
  previously black text rendered red), and multi-line `TySh` text is split at
  `\r` rather than drawn as one line.
- `cxxqt_object/type_tools.rs` (new bridge): `type_preview_rect` /
  `type_preview_rgba` (the engine's own render, so the preview is the commit)
  and `type_commit_layer` — one "Horizontal Type" state, the new layer active
  and selected in the Layers panel; `type_edit_begin` / `type_edit_cancel` /
  `type_commit_edit` reopen a type layer (hidden while retyped, one "Edit Type
  Layer" state).
- `tool_type.cpp`: the handler for all four Type tools. A click opens a session
  that edits like a text field (`type_text_edit.cpp`, photorust's caret /
  anchor model): click to place the caret, drag or Shift-click to select,
  double-click a word, arrows / Home / End (Ctrl by word or whole text, Shift
  extends), Backspace / Delete, Ctrl+A / C / X / V, Enter for a new line;
  Ctrl+Enter or keypad Enter commits, Esc cancels; a click away or a tool
  switch commits. The caret stops come from the engine's layout
  (`pictura_render::type_caret_stops`, new module `type_caret.rs`). A click on an
  existing type layer reopens it in its own orientation and settings. An application
  key filter quiets the window's letter shortcuts while typing.
  `image_view_type.cpp` draws the preview and caret.
- The Layers panel draws a type row's thumbnail as a dark T on a white card,
  as CS6 does, instead of its pixels (opened type layers included).
- Host fonts: `type_fonts.cpp` rebuilds the sfnt of the face Qt resolves for
  the bar's family from `QRawFont` tables and registers it with the engine
  (`pictura_render::register_font`, new module `fonts.rs`); layout, caret stops,
  and rendering use that face (bundled Liberation Sans otherwise), and `TySh`
  records its PostScript name. With a type layer selected and no text being
  typed, a bar change (family, size, alignment, anti-aliasing) re-sets the
  layer — only the changed field — as one "Edit Type Layer" state
  (`type_update_layer`); selecting a type layer shows its family, size, and
  alignment in the bar.
- `options_bar_type.cpp`: Toggle Text Orientation, font family, size (px),
  anti-aliasing (None / Sharp), alignment, Cancel / Commit.
- Qt Test `tst_type_tools::horizontalTypeTool`, `typeKeysAndCancel`,
  `typeOptionsBar`, `reopenTypeLayer`, `freeTransformKeepsType`,
  `textEditModel`, `editLikeATextField`, `barRestylesTheSelectedLayer`. The guard (98) now probes Path Selection and `shift_plain`
  (117) presses A.

## Capabilities

### New Capabilities

- `tools/horizontal-type-tool`: the Horizontal Type tool.

## Impact

- `pictura-core`, `pictura-codec`, `pictura-render`, `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/psd/text_write.rs` and its shell's type entry
(<https://github.com/perfecto25/photorust>). Behavioural parity only: no CS6
oracle exists for type layout, the `TySh` bytes Photoshop accepts, or history
labels; the authored block is checked by Kooka's own reader. Ceiling
(`ponytail:`): a variable font's named instance renders as its default instance
and a registered face stays loaded for the process; point type only (no paragraph, on-path, or Warp Text); no input-method composition; a
reopened layer is retyped in one style and its own `TySh` replaced by the
authored one, and its Layers row shows it hidden while it is retyped; Distort /
Perspective and a type layer with a layer mask still resample its pixels; no font style, colour swatch,
or Crisp / Strong / Smooth; 8-bit RGB documents only.
