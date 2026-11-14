# Proposal

## Why

Issue #1167, a child of #1159. The Layers panel is the only seam for its row
behaviour: 175 of 213 public declarations are `*ForTest` hooks and every Qt
test reaches behaviour through `PicturaMainWindow`. The row context menu has
the same shape in miniature. `layers_panel_menu.cpp` holds 50 `RowSpec`
entries and a 134-line `commandForRow()` chain that links each row id to a
`command_ids` constant by string, and menu applicability still asks the live
`PictureView` (`layer_is_fill_content`, `layer_can_rasterize_smart_object`,
`clipping_can_release`). No test can build the menu without a document and a
frame.

## What Changes

- Rename `LayersModel` to `LayerRowModel` and give it
  `actionsFor(path, facts)`: the resolved row-menu actions (id, label, command
  id, block, enabled, submenu) for one row, derived only from the row's
  snapshot fields and three command-availability facts.
- Replace the parallel `kRowSpecs` + `commandForRow()` lists with one
  `RowAction` table carrying the command id. Keep a small resolver for the
  five dynamic rows (rasterize target, clipping create/release, link/unlink,
  and the two mask submenus).
- Keep the bespoke parts: the Background brief menu, the type-row submenus, the
  mask child menus, Rename, and Color Label. The panel still anchors the row
  and dispatches through `CommandRegistry`.
- Route `populateRowMenu` through `actionsFor`; the panel emits a separator
  when the non-empty block changes, so the menu stays byte-identical.
- Add `tst_layers_row_model`, constructing `LayerRowModel` from hand-built rows
  and facts with no `PicturaMainWindow` and no `PictureView`, plus a check that
  the static rows carry their command ids.
- No behavior change. `tst_layers_row_menu` remains the oracle and is not
  edited.

Out of scope, as separate follow-ups:

- Moving the 175 `*ForTest` hooks to a test adapter (937 call sites across 40
  files; the measured bulk of #1167).
- A behaviour-owning `LayerCommandTarget`: `CommandRegistry` already dispatches
  by id and the anchor write stays in the panel (#829 owns target ownership).
- The panel-flyout `rowsForPanel` / `panelCommandId` table (a second string
  join).

## Capabilities

No spec delta: this is an internal refactor with no requirement change
(`skip_specs: true`).

### New Capabilities

None.

### Modified Capabilities

None.

## Impact

- `crates/pictura-app/cpp/panels/layers_row_actions.{h,cpp}` (new) —
  `RowFacts`, `ResolvedRowAction`, the `RowAction` table, and
  `resolveRowActions`.
- `crates/pictura-app/cpp/panels/layers_panel_model.h` — `LayersModel` renamed
  to `LayerRowModel`; `actionsFor`.
- `crates/pictura-app/cpp/panels/layers_panel_menu.cpp` — delete `kRowSpecs`
  and `commandForRow()`; consume resolved actions; fetch `RowFacts` once per
  menu.
- `crates/pictura-app/cpp/panels/layers_panel.h` and `layers_panel.cpp` — the
  forward declaration and construction rename.
- `cmake/sources/panels.cmake` — register the new source.
- `crates/pictura-app/cpp/tests/tst_layers_row_model.cpp` (new) and
  `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- No Rust bridge change, no new dependency, no command-id change.

## Provenance

No Adobe behavior is involved; the existing `tst_layers_row_menu` is the CS6
behavior oracle.
