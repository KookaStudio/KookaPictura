# Proposal: convert-point-tool

## Why

The Convert Point tool (issue #36) was catalogued but disabled. photorust's
`core/src/path.rs` converts between corner and smooth points; the port follows
`docs/03-tools/pen-and-path-tools.md`.

## What Changes

- `pictura_core::path`: `drag_new_handles` (corner to smooth), `set_corner`
  (smooth to corner; false when already one), and `move_handle` with
  `independent` breaking the pair.
- `cxxqt_object/paths.rs`: `path_set_corner`, `path_drag_new_handles`,
  `path_move_handle`, `path_commit_convert`; one "Convert Point" state per
  click or drag.
- `tool_pen.cpp`: a handle is hit before its anchor.
- Qt Test `tst_pen_tools::convertPointTool`.

## Capabilities

### New Capabilities

- `tools/convert-point-tool`: the Convert Point tool.

## Impact

- `pictura-core` (`path.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/path.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only: no CS6 oracle exists for path geometry or
history labels. Ceiling (`ponytail:`): no Alt-from-Pen or Ctrl+Alt-from-Direct-Selection
shortcut to the tool.
