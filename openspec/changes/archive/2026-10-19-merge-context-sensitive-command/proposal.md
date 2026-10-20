# Proposal

## Why

Issue #744 reports two CS6 parity defects around merging. The Layer menu exposes
`Merge Down` and `Merge Layers` as separate entries, and `Merge Down` stays
enabled with a multi-selection, where it merges only the current row. CS6 has one
context-sensitive item: `Merge Down` with exactly one selected layer, `Merge
Layers` with two or more, both on `Ctrl+E`. The current split also contradicts
the original `D4` design (`layer.merge.layers`, one id) and the command-registry
and Layers-panel specs, which already describe a single selection-dependent
command. The main `layer-management` spec carries a second, conflicting
requirement, `Merge Down as a distinct command`.

## What Changes

- Remove the separate `layer.merge.down` command, its handler, and its bridge
  helper. `layer.merge.layers` remains the only merge command; no second
  `Merge Down` item exists.
- Make the Layer menu label follow the selection: `Merge Down` for exactly one
  selected layer, `Merge Layers` for two or more, and `Merge Layers` (disabled)
  with no selection.
- Keep `Ctrl+E` on that command and let the existing bridge choose
  `MergeScope::Down` for one path and `MergeScope::Selected` for several.
- Make the Layers panel flyout label follow the same rule and disable the entry
  when the current selection cannot merge; its dispatch already uses
  `selectedPaths()`.
- Correct the two conflicting `compositing/layer-management` requirements and
  `docs/05-layers/merge-and-flatten.md`, which still describe two menu entries.
- Add Qt coverage for both selection states, the label swap, `Ctrl+E` routing,
  the disabled states, and the retired `layer.merge.down` id, plus a Rust test
  for masks and union alpha on Merge Down.
- Non-menu command listings (`CommandRegistry::describe()`, the Keyboard
  Shortcuts dialog) keep the declarative `Merge Layers` label; only the Layer
  menu and the panel flyout are selection-aware.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-management`: both existing merge requirements are replaced
  by one context-sensitive command requirement whose label, scope, and
  enablement follow the selection. The `document/command-registry` and
  `ui/layers-panel` specs already describe the single `layer.merge.layers`
  command and need no requirement change.

## Impact

- `crates/pictura-app/cpp/commands.h`, `command_tree.cpp`,
  `frame_menus.cpp`, `frame_menus_layer_ops.cpp`, `frame.h`
- `crates/pictura-app/src/cxxqt_object/layer_arrange.rs`
- `crates/pictura-app/cpp/panels/layers_panel.{h,cpp}` (a
  `canMergeSelection()` helper for the flyout) and `panel_group_menu.cpp`
- `crates/pictura-render/src/document_ops/layer_ops/merge_tests.rs` (test-only:
  mask and union-alpha coverage)
- Qt tests (`tst_command_tree.cpp`, a new `tst_merge_commands.cpp`), and the
  C++ test list in `crates/pictura-app/cpp/tests/CMakeLists.txt`
- `openspec/specs/compositing/layer-management/spec.md` via this change's delta
- `docs/05-layers/merge-and-flatten.md`, `docs/02-ui-ux/panels/layers-panel.md`,
  and `docs/02-ui-ux/menus.md` (docs-only commit with `TASK-ALLOWS-DOCS`)
- No Rust engine behavior or dependency changes; the merge engine already
  supports both scopes.
