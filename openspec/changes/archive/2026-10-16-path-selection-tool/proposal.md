# Proposal: path-selection-tool

## Why

The Path Selection tool (issue #41) was catalogued but disabled. photorust's
`core/src/path.rs` moves a whole subpath and hit-tests one from inside; the
port follows `docs/03-tools/path-selection-tools.md`.

## What Changes

- `pictura_core::path`: `move_subpath`, `duplicate_subpath`, `remove_subpath`
  (`delete_anchor` now uses it), `hit_subpath` (a segment or lone anchor
  within the radius, else inside a closed subpath, even-odd over the flattened
  curve), and `subpath_bounds` (the flattened curve's bounds).
- `cxxqt_object/paths.rs`: `path_hit_subpath`, `path_subpath_bounds`,
  `path_move_subpath`, `path_duplicate_subpath`, `path_commit_drag`, and
  `path_remove_subpath`. A drag records one "Drag Path" state, an Alt-drag one
  "Duplicate Path Component", Delete one "Delete Path".
- `tool_path_selection.cpp` (new, shared with Direct Selection): click a
  component to select it (anchors drawn solid), drag to move it, Alt-drag to
  drag a copy, Delete / Backspace to remove it, empty canvas to deselect.
  `workPathOverlay` (`path_overlay.h`, defined in `tool_pen.cpp`) builds the
  overlay for both tool groups.
- `options_bar_pen.cpp`: Show Bounding Box (off) frames the selected
  component (`PenOptions::showBoundingBox`).
- Qt Test `tst_path_selection_tools::pathSelectionTool`. The guard (98) now
  probes Rectangle and `shift_plain` (117) presses U.

## Capabilities

### New Capabilities

- `tools/path-selection-tool`: the Path Selection tool.

## Impact

- `pictura-core` (`path.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/path.rs` and its shell's
`CanvasView::pathSelectPress` (<https://github.com/perfecto25/photorust>).
Behavioural parity only: no CS6 oracle exists for path geometry, pick radius,
or history labels. Ceiling (`ponytail:`): no Shift-click multi-selection,
arrow nudges, path operations, alignment / arrangement, or drag to another
document; the selection is dropped on a tool switch.
