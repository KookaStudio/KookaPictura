# Proposal: pattern-stamp-tool

## Why

The Pattern Stamp (issue #18) was catalogued but disabled. photorust ships it
as the clone stroke over a tiled pattern, with a generated, seamless pattern
set. This change ports both onto the source stroke added by `clone-stamp-tool`
(`docs/03-tools/clone-stamp-and-pattern-stamp.md`), completing the `S` group.

## What Changes

- `pictura_paint::pattern`: eight generated greyscale, seamless 64 px tiles
  (Checkerboard, Grid, Diagonal Stripes, Horizontal Lines, Polka Dots, Woven,
  Bricks, Grain).
- `pictura_paint::stamp::tiled`: a tile repeated over the document from an
  origin, wrapping negative offsets.
- `cxxqt_object/paint_tools.rs`: `begin_pattern_stamp` and the pattern list
  (`stamp_pattern_count` / `_name` / `_tile` / `_side`); one `"Pattern Stamp"`
  state per stroke.
- `tool_stamps.cpp`: Aligned pins the tile to the document origin; unchecked,
  to each stroke's start.
- Options-bar row: the paint fields, a pattern picker with swatches, Aligned,
  and a disabled Impressionist. Catalog row enabled.
- C++ self-test `pattern_stamp_tool` (543).

## Capabilities

### New Capabilities

- `tools/pattern-stamp-tool`: the Pattern Stamp and the built-in patterns.

## Impact

- `pictura-paint` (`pattern.rs`, `stamp.rs`), `pictura-app` bridge and C++.
- No new dependency.

## Provenance

Ported from photorust's `core/src/pattern.rs`, `core/src/document.rs`
(`begin_pattern_stroke`), and `shell/src/MainWindow.cpp` (the pattern options)
(<https://github.com/perfecto25/photorust>). The tiles are generated, not
Adobe's artwork. Behavioural parity only. Impressionist, document / `.pat`
patterns, and a pattern library are `ponytail:` ceilings.
