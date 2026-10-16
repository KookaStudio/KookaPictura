# Proposal: magic-eraser-tool

## Why

The Magic Eraser (issue #23) was catalogued but disabled. photorust's
`core/src/erase.rs` erases the Magic Wand's flood; the port follows
`docs/03-tools/eraser-tools.md` (TOOL-023).

## What Changes

- `pictura_paint::eraser`: `magic_erase` erases a layer through a
  document-sized coverage mask scaled by Opacity (alpha multiplied down), or
  blends toward the background colour on a layer without alpha or with Lock
  Transparency; `antialias_mask` softens a binary mask's edge.
- `cxxqt_object/paint_tools.rs`: `magic_erase_at` floods with
  `pictura_select::magic_wand` over the active layer (or, with Sample All
  Layers, the composite), turns the Background into a layer first, and records
  one `"Magic Eraser"` state.
- `tool_eraser.cpp` click handler; the bar (`options_bar_erase.cpp`) has
  Tolerance 32, Anti-alias, Contiguous, Sample All Layers, and Opacity. Catalog
  row enabled; the E group is complete.
- C++ self-test `magic_eraser_tool` (551). The eraser checks move to
  `selftest_erasers.cpp`, sharing the paint fixture through
  `selftest_paint_fixture.h` (pure moves).

## Capabilities

### New Capabilities

- `tools/magic-eraser-tool`: the Magic Eraser.

## Impact

- `pictura-paint` (`eraser.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/erase.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only. The active
selection does not limit the erase — a `ponytail:` ceiling.
