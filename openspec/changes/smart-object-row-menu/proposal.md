# Proposal

## Why

The Layers row context menu cannot reach the Smart Object commands from a normal
layer: there is no `Convert to Smart Object` row, so a plain layer can never be
converted from the row menu, and the `Edit Contents`, `Replace Contents…`, and
`Export Contents…` rows are missing even though their commands and bridge
operations already exist (#114, #115). The linked-object rows (`Relink to
File…`, `Convert to Linked…`) are absent too; their implementation is tracked
separately (#113), so they SHALL appear disabled under the `— not implemented
yet` convention until that lands.

## What Changes

- Add a **Convert to Smart Object** row to a raster pixel row, enabled only when
  the row is convertible, wired to the existing bridge, and selecting the new
  smart object afterwards.
- Add **Edit Contents**, **Replace Contents…**, and **Export Contents…** rows to
  a smart-object row, enabled by the existing eligibility predicates and wired
  to the existing commands.
- Add **Relink to File…** and **Convert to Linked…** rows to a smart-object row,
  rendered disabled with the `— not implemented yet` tooltip.
- Route the three contents actions through a new Layers-panel signal that the
  frame handles on the row's path, so the row menu acts on the clicked row rather
  than only the current one.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: the row context menu offers the new smart-object rows.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_menu.cpp` (rows, enablement,
  dispatch) and `panels/layers_panel.h` (signal).
- `crates/pictura-app/cpp/frame_menus_smart_object.cpp` (path-aware Replace /
  Export methods and the row-menu signal wiring) and `frame_menus.cpp` (the
  command handlers delegate to the new methods).
- `crates/pictura-app/cpp/tests/tst_command_tree.cpp` and
  `tests/tst_smart_object_actions.cpp`.
- No new dependencies. No `docs/` change.
