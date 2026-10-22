# Rust decides the layer target; the Layers panel only shows it

## Why

#741 moved the layer target into Rust, and almost every structural operation
now sets its own post-op target there. Several C++ paths still choose or
rewrite the selection after an operation anyway, and one of them breaks the
spec: duplicating two or more layers leaves a single layer selected, where
`document/layer-target` requires every copy to be selected.

**The live bug.** `duplicate_layers`
(`crates/pictura-app/src/cxxqt_object/impl_layers.rs:810`) sets the Rust target
to every copy it created. `LayersPanel::duplicateSelection`
(`crates/pictura-app/cpp/panels/layers_panel_actions.cpp:138`) then calls
`selectPath(created.last())` at line 151, which does a `ClearAndSelect` on the
tree (`layers_panel.cpp:679`) without the `applyingTarget_` guard that
`selectPaths` uses (`layers_panel.cpp:691`). The tree's `currentChanged`
handler (`layers_panel.cpp:357`) calls `writeTarget()` (`layers_panel.cpp:767`),
which writes the single tree row back into Rust and collapses the multi-layer
target. `tst_layer_commands.cpp`'s `duplicateMultiSelection` checks row counts
and order and never the target, so the bug passes it.

**The engine reports the wrong copies.** `pictura_render::duplicate_paths`
(`crates/pictura-render/src/document_ops/layer_ops/properties.rs:276`) computes
each copy's path at insert time. The stack is bottom-first (`doc.layers[0]` is
the bottom layer) and the engine processes the selection in its removal order:
higher-index siblings first, which can also place a lower-index root layer
before a higher group's child. A later insert at a lower index in the same
parent pushes an earlier copy up one, so only the last copy's path is still
right. Duplicating a two-layer document yields the paths `["2", "1"]` for a
stack that ends up as `[bottom, bottom copy, top, top copy]`: `"2"` names the
original top layer and `"3"`, the actual top copy, is not named at all. The Rust
target therefore selects `{top, bottom copy}` instead of `{top copy, bottom
copy}`. The engine test `duplicate_paths_returns_new_paths`
(`document_ops/layer_ops/tests.rs:714`) asserts exactly those stale paths, so a
green suite locks the bug in.

Two further engine defects sit behind that one. A copy nested in a group is
reported at a path that a later copy shifts: duplicating `["0", "1/0"]` in a
document of `[group, group(child)]` reports `"1/1"`, but the copy sits at
`"2/1"`. And a source whose path an earlier copy shifts is dropped before its
turn: duplicating `["1", "2/0"]` in a document of `[layer, layer,
group(child)]` reports only `["2"]` and never copies the child, because the
source path is resolved against the document the copies are already changing.

**The deletion is not enough.** `LayersPanel::refresh()`
(`layers_panel.cpp:487`) restores the pre-refresh current row whenever that row
is not in the target's selection (`layers_panel.cpp:634`), so that a current
row moved with Ctrl+arrow survives a repaint. After a structural operation the
pre-refresh current row can be outside the new target — for a single-layer
duplicate it is the source path, which the target does not contain — so
deleting the C++ `selectPath` would snap the current row back onto the old row
instead of the operation's result.

**Two owners, one decision.** Every other structural path re-selects after the
operation even though Rust already set the target: `frame_menus.cpp:361` and
`frame_menus.cpp:384` after a merge, `frame_menus.cpp:1117` writes the target a
second time, `layers_panel_menu.cpp:665` after a merge, and
`frame_menus_smart_object.cpp:82` after a smart-object copy. The reads have the
same split: the Duplicate enabled-provider (`frame_menus.cpp:340`) reads the
tree's rows, while every other provider reads the Rust target, the Align and
Distribute providers included (`frame_menus_align.cpp:24`).

**Per-document state lives in a shared panel.** `LayersPanel` is one widget
shared by every document. Its solo snapshot (`soloSnapshot_`) and its expanded
groups (`expandedPaths_`) are panel members. The expansion leaks: a group
expanded in one document stays expanded in the next one, by index path. The
solo snapshot is dropped: `setView` (`layers_panel.cpp:448`) clears it on a
document switch but leaves the document soloed, so nothing can restore the
visibility. (The isolated set is cleared on a switch too, which
`ui/isolation-mode` requires.)

## What Changes

- **The engine reports the copies it made.** `duplicate_paths` resolves every
  source before the first insert, inserts the copies in panel order, and
  reports each copy at its final path, so the engine test's golden moves to the
  real paths.
- **Rust decides the post-op target for every structural operation**, and the
  decision moves onto `PictureViewRust` so that a plain Rust test can drive it:
  duplicate, merge, merge visible, group and ungroup each get a named target
  rule, and the operations that had no Rust-side target, Select Similar, Select
  Linked and the adjustment layer creation, gain one.
- **The C++ side stops choosing.** Every `selectPath` and `selectPaths` call
  that follows a structural operation is deleted from `frame_menus*.cpp`,
  `layers_panel_actions.cpp`, `layers_panel_menu.cpp`, `frame_build.cpp` and
  `layers_panel_events.cpp`, and the Select menu's `Select Similar` target
  write moves into the engine as well. `Select All Layers` and `Deselect
  Layers` keep writing the target through the bridge: changing the selection
  is their purpose.
- **The panel's current row follows the target.** The panel keeps the target it
  last agreed with. A refresh restores the previous current row only when the
  target did not move, so a Ctrl+arrow current row survives a repaint while a
  structural operation moves the current row onto the new anchor. A stray
  current row no longer rewrites the target's active path to the first selected
  layer. The remembered target is reset on a document switch, so the new
  document's current row is its own active layer.
- **The Duplicate enabled-provider reads the target**, like every other
  provider.
- **The panel's per-document state moves into Rust.** The solo snapshot and the
  expanded groups become per-document Rust state beside the existing
  `isolation`, so the expansion no longer leaks across documents and the solo's
  snapshot survives a document switch. The group expansion's name-chain re-find
  stays in the panel, which is where the model lives, and writes the remapped
  set back to Rust.

## Capabilities

- Modified: `document/layer-target`: the engine owns the target and the user
  interface projects it; a multi-layer duplicate selects every copy; an
  adjustment layer creation and Select Similar/Select Linked select their
  result through the engine.
- Modified: `ui/layers-panel`: the panel's current row follows the target, the
  Duplicate command acts on the target, and the solo snapshot and the group
  expansion belong to the document.

## Impact

- Code:
  - `pictura-render` - `document_ops/layer_ops/properties.rs`: `duplicate_paths`
    resolves the sources first, inserts in panel order, and reports the copies'
    final paths.
  - `pictura-app` (Rust) - `cxxqt_object/impl_layers/target_transitions.rs`
    grows the named post-op target rules on `PictureViewRust`;
    `cxxqt_object/impl_layers.rs`, `impl_layers_merge.rs`,
    `impl_layers_select.rs` and `impl_filters.rs` call them;
    `cxxqt_object/state.rs` grows the per-document solo and expansion state;
    `cxxqt_object/layer_target_bridge.rs` grows the bridge functions the panel
    reads.
  - `pictura-app` (C++) - the target-moved rule and the remembered target move
    into a new `panels/layers_panel_target.cpp`; `panels/layers_panel.cpp`
    keeps the model and projection; `panels/layers_panel_actions.cpp`,
    `panels/layers_panel_menu.cpp`, `frame_menus.cpp`,
    `frame_menus_smart_object.cpp` and `frame_build.cpp` lose their post-op
    selections, and `frame_menus_select.cpp` loses the Select Similar target
    write; the solo and expansion members leave the panel.
- Tests: `pictura-render` unit tests for the copy paths, including the nested
  and shifted-source cases (`tests.rs` is at its cap, so the duplicate tests
  move to `tests_stack.rs`); `pictura-app` Rust tests for the post-op target of
  duplicate, merge, merge visible, group and ungroup and for the per-document
  panel state; `tst_layer_commands`, `tst_layer_target` and
  `tst_layers_panel_fixes` cases for the duplicate target, the current row, the
  document switch and the per-document state.
- **A golden baseline moves.** `duplicate_paths_returns_new_paths` asserts the
  stale paths today; the corrected paths are the change's first test.
- No new dependency. The only new engine state is the per-document solo and
  expansion set, moved out of the panel.
- `docs/` is unchanged: the corpus already says the target is the document's
  one authoritative selection and that the panel shows it. `ROADMAP.md` is
  unchanged: no roadmap item covers this change.
- **Behaviour changes.** The Layer menu's `Select Similar` keeps the active
  layer selected, matching the `Select` menu's existing path and the
  `compositing/layer-management` rule that every matching layer is selected. An
  adjustment layer created from the panel menu or the `Layer` menu becomes the
  target, which is what `ui/adjustments-panel` requires of the icon grid.
- **Out of scope.** Which layer an operation selects stays as it is, except
  for the duplicate, the adjustment creation and Select Similar. The layer
  dialogs are not redesigned. The Move tool's auto-select, the history capture
  of the target and the undo/redo rules are untouched: the history already
  carries the pre- and post-operation target of every state and restores them
  correctly.
