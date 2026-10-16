# Proposal: vertical-type-tool

## Why

The Vertical Type tool (issue #38) was catalogued but disabled. It is the
Horizontal Type tool's session and commit with the text set in columns; the
port follows `docs/03-tools/type-tools.md` and photorust's type entry.

## What Changes

- `pictura_render::type_layer`: `vertical_layout` stacks each line's characters
  top to bottom in a column `1.2 × size` wide, columns running right to left;
  the click is the first column's top centre, moved by Top / Center / Bottom.
  `render_text_buffer` draws a `TypeTool` with `vertical` this way, so an opened
  vertical type layer renders in columns too.
- `pictura_codec::author_type_tool` writes `Ornt` `Vrtc`; the reader sets
  `TypeTool::vertical` from it.
- `type_commit_layer` records one "Vertical Type" state for vertical text.
- `tool_type.cpp` / `options_bar_type.cpp`: Enter starts a new column; the
  alignment buttons read Top / Center / Bottom.
- Qt Test `tst_type_tools::verticalTypeTool`.

## Capabilities

### New Capabilities

- `tools/vertical-type-tool`: the Vertical Type tool.

## Impact

- `pictura-core`, `pictura-codec`, `pictura-render`, `pictura-app` (C++).
- No new dependency.

## Provenance

Ported from photorust's type entry and `core/src/psd/text_write.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only: no CS6
oracle exists for vertical layout. Ceiling (`ponytail:`): every character is set
upright on a fixed `ascent + descent` cell — no `vert` alternates, rotated
Latin, or tate-chu-yoko — plus the Horizontal Type tool's ceilings.
