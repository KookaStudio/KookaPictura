# Proposal: art-history-brush-tool

## Why

The Art History Brush (issue #20) was catalogued but disabled. photorust has
no Art History Brush, so it is implemented fresh from
`docs/03-tools/art-history-brush.md` (TOOL-034) on the History Brush's source.

## What Changes

- `pictura_paint::art_history`: `ArtStyle` (CS6's ten styles, Tight Short …
  Loose Curl Long), `ArtHistoryOptions` (Style, Area, Tolerance, seed), and a
  per-dab engine. Each dab scatters strokes over the Area; a stroke takes its
  colour from the source at its start, runs along the source's edges (across
  its luminance gradient) for the style's length with its wander and curl, and
  is skipped where the layer is within Tolerance of the source.
  `Stroke::begin_art_history` runs it; 16/32-bit documents are refused.
- `cxxqt_object/paint_tools.rs`: `begin_art_history_brush` (one `"Art History
  Brush"` state per stroke, seeded by the history index so strokes differ but
  replay), `art_history_style_count` / `_name`.
- `tool_stamps.cpp` handler; the bar has the tip, Mode (Normal only, greyed),
  Opacity, Style, Area, and Tolerance. Catalog row enabled; the Y group is
  complete.
- C++ self-test `art_history_brush_tool` (548); `shift_plain` (117) asserts
  Shift+Y reaches it.

## Capabilities

### New Capabilities

- `tools/art-history-brush-tool`: the Art History Brush.

## Impact

- `pictura-paint` (`art_history.rs`, `Stroke`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Original to Kooka (no photorust source). Behavioural parity only: the style
table is a design choice, not Adobe's kernels. Tolerance starts at 0 % — the
spec's unverified 100 % default would paint almost nowhere. Blend modes other
than Normal, and painting while the pointer is held still, are `ponytail:`
ceilings.
