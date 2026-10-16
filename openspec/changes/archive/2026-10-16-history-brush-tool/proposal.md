# Proposal: history-brush-tool

## Why

The History Brush (issue #19) was catalogued but disabled. photorust has no
History Brush, so it is implemented fresh as the source stroke added by
`clone-stamp-tool` reading an earlier history state
(`docs/03-tools/history-brush.md`).

## What Changes

- `History` (`pictura-app/src/history.rs`) gains a brush source: the oldest
  state by default (CS6's opening snapshot), a chosen state, or a named
  snapshot. The index follows its state as the depth limit drops older ones,
  and a source that pruning or a post-undo capture would discard is pinned as a
  copy, so the brush never reads a discarded state.
- `pictura_paint::stamp::layer_surface` reads the source state's layer at the
  active layer's panel path, in document space.
- `cxxqt_object/paint_tools.rs`: `begin_history_brush`, one `"History Brush"` state
  per stroke; `history_brush_source_state` / `_snapshot` and
  `set_history_brush_source` for the panel.
- History panel: a History Brush icon marks the source row; a press in the
  left (icon) column chooses the source instead of jumping to the state.
- `tool_stamps.cpp` History Brush handler; the bar carries the paint fields.
  Catalog row enabled.
- C++ self-test `history_brush_tool` (544); `shift_plain` (117) asserts the Y
  group's single implemented member.

## Capabilities

### New Capabilities

- `tools/history-brush-tool`: the History Brush and its source.

## Impact

- `pictura-app` (`history.rs`, bridge, History panel, C++).
- No new dependency.

## Provenance

Original to Kooka (no photorust source); it reuses the ported source stroke.
Behavioural parity only. The source layer is matched by panel path (layers carry
no stable id), and merged snapshots, Erase to History, and `Edit > Fill >
History` are `ponytail:` ceilings.
