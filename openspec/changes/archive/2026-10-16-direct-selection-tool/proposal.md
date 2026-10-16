# Proposal: direct-selection-tool

## Why

The Direct Selection tool (issue #42) was catalogued but disabled. photorust's
`core/src/path.rs` moves one anchor with its handles and drags one handle; the
port follows `docs/03-tools/path-selection-tools.md`.

## What Changes

- `pictura_core::path`: `move_anchor` carries the anchor's handles by the
  same delta. Handle drags reuse `move_handle` (a smooth point's other handle
  keeps its length and swings to stay collinear; Alt breaks the pair).
- `cxxqt_object/paths.rs`: `path_move_anchor`; drags commit through
  `path_commit_drag` as one "Drag Anchor Point" or "Drag Direction Point"
  state.
- `tool_path_selection.cpp`: a handle on the selected component wins over an
  anchor; an anchor drag selects and moves it (drawn solid, its component's
  other anchors hollow); a segment click selects the component; Alt-click
  selects the component whole, which Delete then removes.
- Qt Test `tst_path_selection_tools::directSelectionTool`.

## Capabilities

### New Capabilities

- `tools/direct-selection-tool`: the Direct Selection tool.

## Impact

- `pictura-core` (`path.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/path.rs` and its shell's
`CanvasView::pathSelectPress` (<https://github.com/perfecto25/photorust>).
Behavioural parity only: no CS6 oracle exists for path geometry or history
labels. Ceiling (`ponytail:`): no segment drags, marquee, Shift-click
multi-selection, arrow nudges, or Constrain Path Dragging; Delete does not
remove a lone anchor's segments.
