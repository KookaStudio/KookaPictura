# Proposal

## Why

The layer target is stored twice and the copies disagree. Rust keeps one
`active_layer: Option<String>`; the real selected set lives only in the Layers
panel's `QItemSelectionModel`. The panel clears Rust's target on a
multi-selection, a panel refresh reverts Rust-side target writes, undo never
touches the target, and a layer filter can hide the target so Move acts on the
wrong layer. Commands, tools, and the control server therefore disagree about
which layers an operation targets.

## What Changes

- Add one Rust-owned per-document layer target: the selected layer paths plus
  one active path (the anchor), with a single edit target derived only when
  exactly one path is selected.
- Make the Layers panel a view of that state: user selection writes the target;
  a refresh re-applies it instead of overwriting it; filtering only hides rows
  and never mutates the underlying target.
- Keep the target correct across structural layer operations (add, duplicate,
  group, ungroup, delete, move/arrange/reverse, background conversion, merge,
  flatten, paste/place, type/shape creation, smart object via copy, Alt-drag
  copy, cross-document copy, image-mode conversion).
- Make Move Auto-Select write the target directly, so a hit whose row is hidden
  by a filter still becomes the target.
- Carry the target through history so undo, redo, and named snapshots restore it
  with the document, while a layer selection changed after an edit is preserved
  rather than clobbered.
- Expose the target to C++: `selected_layer_paths()`,
  `layer_selection_anchor()`, `set_layer_target()`, `select_layer_path()`,
  `clear_layer_target()`, `layer_target_changed`.
- Migrate frame menus, panel commands and properties, tools, and layer creation
  to read and write the Rust target instead of `LayersPanel::currentPath()` /
  `selectedPaths()`.
- Keep the existing index-addressed invokables and the control-server JSON
  contract unchanged in this change; retiring them is a follow-up. Their bridge
  functions still get target transitions.

## Capabilities

### New Capabilities

- `document/layer-target`: the document session owns the selected layer paths
  and the active anchor, keeps them resolvable across structural edits and
  history, and defines the single edit target.

### Modified Capabilities

- `ui/layers-panel`: the panel reflects the session target instead of owning
  it; a filter or model refresh cannot clear the active layer; multi-selection
  keeps the underlying target for commands and property edits.
- `tools/tool-framework`: the shared resolver reads the session target; the
  panel no longer pushes the selection; a zero- or multi-selection still refuses
  tool edits while structural commands remain available.

## Impact

- `crates/pictura-app/src/layer_target.rs` (new) and a side bridge
  `cxxqt_object/layer_target_bridge.rs` (keeps the root bridge header and its
  size ceiling intact).
- `crates/pictura-app/src/history.rs`: snapshots and deltas carry the target,
  including named snapshots.
- `crates/pictura-app/src/cxxqt_object/`: state, open/new/reset (the opening
  target is computed before the first history capture), all structural layer
  ops, and the Rust `active_layer` readers split into single-target vs anchor
  uses.
- `crates/pictura-app/cpp/panels/layers_panel*.cpp`: write/apply target, drop
  the refresh capture/re-select and fallback, and read the target for property
  controls and selection-based commands.
- `crates/pictura-app/cpp/frame_menus*.cpp`, `tools.h`, and the 33 non-test C++
  `active_layer_path()` readers: read the target or the anchor as appropriate;
  the dead `layersPanel_` guards disappear as a byproduct.
- Tests: new `tst_layer_target`, Rust unit tests for target and history;
  updates to `tst_layers_panel`, `tst_merge_commands`, and history fixtures.
- `set_active_layer("")` now clears the target instead of only disabling tool
  edits; its callers are updated to the one-selected-layer invariant.
- No new dependencies. No `docs/` changes. No control-server or export-dialog
  request/response change.
