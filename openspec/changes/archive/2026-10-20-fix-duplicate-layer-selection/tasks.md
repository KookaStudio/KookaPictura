# Tasks

## 1. Regression tests

- [x] 1.1 Add `tst_layer_commands.cpp` with a `LayerCommandsTest` fixture and
  register it in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] 1.2 Add `duplicateWithoutSelection`: clear the panel selection, refresh the
  registry, assert the action is disabled, dispatch it, and assert the row count
  and history count are unchanged.
- [x] 1.3 Add `duplicateNestedLayer`: add a group and a child, expand the group,
  select the child, run the command, and assert a `"<name> copy"` row directly
  above the source in the same group with one history state.
- [x] 1.4 Add `duplicateMultiSelection`: select two top-level layers, run the
  command, and assert both gain an adjacent copy with one history state.

## 2. Handler fix

- [x] 2.1 Expose `LayersPanel::duplicateSelection` in the public section of
  `panels/layers_panel.h` next to `groupSelection`/`deleteSelection`.
- [x] 2.2 In `frame_menus.cpp`, call `layersPanel_->duplicateSelection()` from the
  `LayerDuplicateLayer` handler instead of the index-based `duplicate_layer`.
- [x] 2.3 Move `LayerDuplicateLayer` out of the document-only enablement loop and
  gate it on a non-empty `selectedPaths()`.

## 3. Integration verification

- [x] 3.1 Build with CMake/Ninja.
- [x] 3.2 Run `ctest --test-dir build -R '^tst_layer_commands$' --output-on-failure`.
- [x] 3.3 Run `python3 scripts/check-test-plan.py` and
  `openspec validate --all --strict`.
- [x] 3.4 Archive the change with
  `openspec archive fix-duplicate-layer-selection -y`.
