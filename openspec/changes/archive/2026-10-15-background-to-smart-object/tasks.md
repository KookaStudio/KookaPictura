# Tasks

## 1. Engine eligibility and unlock

- [x] 1.1 In `crates/pictura-render/src/document_ops/layer_ops/smart_object.rs`, drop the `!layer.background` term from `can_convert_to_smart_object` and remove "not the Background" from its doc comment; keep the other guards (8-bit, non-group, no adjustment, no existing smart object, positive `rect`).
- [x] 1.2 In `convert_to_smart_object`, unlock a Background target as part of the same operation: clear the flag, release the locks, add an opaque alpha channel when absent, and rename to the next free `Layer N` (reuse `create::{release_background, next_layer_name}`). Author the payload from a copy of the layer with the unlock applied, and mutate the real layer (unlock + attach the smart object) only after `write_psd` succeeds, so a serialization failure still returns `false` without mutating the document. Update the doc comment.

## 2. Menus

- [x] 2.1 In `crates/pictura-app/cpp/panels/layers_panel_menu.cpp`, change the `convertToSmartObject` row's kind mask from `Pixel` to `Pixel | Background`; the handler and `ConvertToSmartCan` predicate are unchanged.
- [x] 2.2 Update the now-false comment in `crates/pictura-app/cpp/frame_menus.cpp` (around line 803) that lists the Background among the ineligible layers.

## 3. Tests

- [x] 3.1 In `crates/pictura-render/src/tests/smart_object.rs`, remove the `"background"` case from `convert_refuses_ineligible_targets` and add a positive test: a Background-flagged layer converts, is no longer the Background, has its locks released, is renamed to the next free `Layer N`, carries a smart object, the composite is unchanged, and a second convert refuses. Also assert `convert_for_smart_filters` on a Background unlocks and converts (it delegates, covering the `imaging/filter-app-ui` delta).
- [x] 3.2 In `crates/pictura-app/cpp/selftest_layers_smart_object.cpp` (check 277), remove the now-invalid Background-refusal assertion (`socRefBackground`), the `background_from_layer("0")` setup it needed, its `ST_PASS` format slot/argument, and the stale "a group and the Background refuse" comment. Keep the group-refusal and unchanged-history assertions; the file must shrink and stays under the 1200-line cap.
- [x] 3.3 In `crates/pictura-app/cpp/tests/tst_smart_object_actions.cpp`, add a Qt Test case for the Background path. The bridge `newDocument` creates a plain `Layer 0` (not a Background), so first call `view->background_from_layer(path)` and take the history baseline after it. Assert: the row menu contains `Convert to Smart Object` and it is enabled; the `LayerSmartObjectConvertTo` command is enabled; dispatching records exactly one `Convert to Smart Object` state; the row is no longer `background` and carries a smart object; a second dispatch is disabled; and a group row omits the row.
- [x] 3.4 In `crates/pictura-app/cpp/tests/tst_command_tree.cpp`, add `Convert to Smart Object` to `expectedBackground` (between `Blending Options…` and `Export As…`) since the exact-list `QCOMPARE` now sees it on a Background row.

## 4. Verification (run when the user asks)

- [x] 4.1 `cargo nextest run -p pictura-render smart_object`
- [x] 4.2 `cmake --build build --parallel` then `ctest --test-dir build -R '^tst_(smart_object_actions|command_tree)$' --output-on-failure`
- [x] 4.3 `bash scripts/verify-full.sh` (includes the CMake configure/build on a clean tree) green, and `openspec validate --all --strict`
