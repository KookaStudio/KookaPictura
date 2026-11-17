# Tasks

## 1. Tests first

- [x] 1.1 Add `LayerCommandsTest::ctrlJCopiesSelection` to
  `crates/pictura-app/cpp/tests/tst_layer_commands.cpp`: select a pixel layer,
  `select_rect`, assert the action is enabled and keeps `Ctrl+J` and the
  `Layer via Copy` label, dispatch `layer.new.layerViaCopy`, then assert the
  copy thumbnail keeps alpha only inside the selection, the source is unchanged,
  and one history state is recorded. Verify
  `python3 scripts/check-test-plan.py` resolves the slot.
- [x] 1.2 Add `LayerCommandsTest::ctrlJDuplicatesGroup`: build a group with a
  named child, select the group with no selection, refresh the registry, assert
  the action is enabled, dispatch, and assert row count +2, a `"<name> copy"`
  group holding a child copy, the group copy selected, the source group, its
  child, and the child count unchanged, and one history state. Then repeat the
  dispatch with an active selection and assert the group still duplicates.
  Then clear the selection, select the source group plus the plain bottom layer
  with the group as the anchor, dispatch, and assert row count +3 (a whole group
  copy with its child plus the plain layer's copy) and one history state.
  Attempt the accelerator with
  `QTest::keyClick(&frame, Qt::Key_J, Qt::ControlModifier)`, falling back to the
  action's `trigger()` when offscreen Qt does not deliver the shortcut (the
  pattern in `tst_merge_commands.cpp:627-637`); first assert
  `commandForShortcut(QKeySequence("Ctrl+J"))` resolves to the command.
  Verify `python3 scripts/check-test-plan.py` resolves the slot.
- [x] 1.3 Add `LayerCommandsTest::ctrlJDuplicatesLayerWithoutSelection`: select
  a pixel layer with no selection, assert the action is enabled after the
  refresh, dispatch, and assert the `"<name> copy"` appears above the source, is
  selected, and adds one history state. Then make an active selection that
  covers no pixels with an out-of-document `select_rect` and dispatch again;
  assert the duplicate path still runs (`"Duplicate Layer"` history label).
  Verify `python3 scripts/check-test-plan.py` resolves the slot.
- [x] 1.4 Add `LayerCommandsTest::ctrlJCutOutcomes` covering three cases in one
  test: (a) a raster layer carrying alpha with a selection → copy above,
  selected pixels cleared from the source, outside pixels preserved, one history
  state; (b) no selection → action disabled, dispatch changes nothing, and a
  zero-coverage active selection → action disabled; (c) a group with an active
  selection → action disabled, dispatch changes nothing.
  Verify `python3 scripts/check-test-plan.py` reports no gaps.

## 2. Contextual Ctrl+J dispatch

- [x] 2.1 In `crates/pictura-app/cpp/menu_registrar.cpp`, split the shared
  enablement loop: `Layer via Copy` enables on document + non-empty layer
  anchor; `Layer via Cut` keeps document + a selection covering at least one
  pixel + anchor and additionally requires the anchor row kind to be non-group.
  Verify by reading the providers and by the group/cut test assertions.
- [x] 2.2 Branch the `LayerNewLayerViaCopy` handler: when the anchor row kind is
  `group` or `selection_count() == 0`, call
  `panels_->layersPanel()->duplicateSelection()` (return without a Layers
  panel); otherwise keep `layer_via_copy(anchor)`. Verify against `design.md`
  D2 and the test plan.
- [x] 2.3 Run `cmake --build build --target tst_layer_commands`, the offscreen
  suite
  (`QT_QPA_PLATFORM=offscreen python3 scripts/cpu-budget.py exec -- ctest --test-dir build -R '^tst_layer_commands$' --output-on-failure`),
  and `cargo test -p pictura-render tests_via`; every test-plan row passes and
  the existing duplicate tests still pass.

## 3. Tracking and repository gates

- [x] 3.1 In a `docs:` commit carrying `TASK-ALLOWS-DOCS`, flip
  `docs/dev/doc-code-parity-gaps.md:191` (`LAY-12`) to `SHIPPED` with the handler
  and test locations, and tick `ROADMAP.md:138`. Verify both rows read as
  shipped.
- [x] 3.2 Run `openspec validate --all --strict` and
  `python3 scripts/check-test-plan.py`; both pass.
- [x] 3.3 Archive the change with `openspec archive ctrl-j-duplicate-semantics`
  and run `bash scripts/check-openspec-archived.sh`; only `archive/` remains
  under `openspec/changes/`.
- [ ] 3.4 Run `bash scripts/verify-fast.sh`; every gate passes.
