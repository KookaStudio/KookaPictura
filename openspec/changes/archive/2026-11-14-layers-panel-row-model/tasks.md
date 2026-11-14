# Tasks

## 1. Tests first

- [x] 1.1 Add `crates/pictura-app/cpp/tests/tst_layers_row_model.cpp` with `pixelRowActionsFromHandBuiltRows`, `theRowTableCarriesStaticCommandIds`, `dynamicRowsResolveFromRowFacts`, `maskRowsExposeTheirSubmenus`, and `unimplementedRowsAreDisabled`; register `tst_layers_row_model` in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify the file is listed and the suite compiles once group 2 defines the API.
- [x] 1.2 Confirm `test-plan.md` covers every row in this plan and every acceptance bullet from #1167 that this change carries; verify `python3 scripts/check-test-plan.py --self-test` passes (the full check runs in group 4 once the new tests exist).

## 2. Row-action table and model

- [x] 2.1 Add `crates/pictura-app/cpp/panels/layers_row_actions.h` and `.cpp` with `RowFacts`, `ResolvedRowAction`, the `RowAction` table (the 50 current `RowSpec` rows plus a `commandId` column and a `RowDynamic` value), and `resolveRowActions`; move `KindMask`/`kindMaskFor` into the `.cpp`; register both files in `cmake/sources/panels.cmake`; verify `cmake --build build --target pictura` succeeds and no `commandForRow` remains in the new file.
- [x] 2.2 Rename `LayersModel` to `LayerRowModel` in `layers_panel_model.h`, `layers_panel.h` (`model_` and the forward declaration), and `layers_panel.cpp`; add `rowForPath` and `actionsFor(path, facts)`; verify the app builds and `grep -rn '\bLayersModel\b' crates/pictura-app/cpp` returns nothing.
- [x] 2.3 Run `tst_layers_row_model`: the hand-built pixel row resolves the same order, labels, separators, command ids, and enabled states as the current menu; the static rows carry their `command_ids` values; the dynamic rows choose smart/fill rasterize, clipping create/release, and link/unlink from the row and `RowFacts`; the two mask rows expose a submenu and are disabled when the mask exists; unimplemented rows are disabled with an empty command id; an absent path resolves to no row and no actions. Passed.

## 3. Rewire the row menu

- [x] 3.1 Rewrite `LayersPanel::populateRowMenu` to take the looked-up `LayerRowNode` (the caller uses `model_->rowForPath(path)`), fill `RowFacts` from the view once, call `resolveRowActions`, group the result by `block`, and build each `QAction` through `CommandRegistry::detachedAction`; delete `kRowSpecs` and `commandForRow`; keep Rename, Color Label, the Background brief menu, the type rows, the mask child menus, the nine label overrides, and the isolation insertion; verify `tst_layers_row_menu` passes with no edits to the test file.
- [x] 3.2 Make `rowMenuTextsForTest(kind)` build a synthetic `LayerRowNode` and use the same resolver, so the kind-only layout checks keep working without a view; verify `tst_layers_row_menu::kindTextLists`, `pixelRowFullSet`, `shapeOmitsEditAdjustment`, `plainPixelOmitsSmartRows`, and `unimplementedDisabled` pass.

## 4. Integration verification

Verified on 2026-11-14.

- [x] 4.1 Build and run the layers suites offscreen: full `cmake --build build --parallel 12`, then `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_layers_(row_model|row_menu|panel|panel_fixes|effects|mask_commands|smart_filters)$' --output-on-failure`; 7/7 suites pass (`tst_layers_row_model` 0.05s, `tst_layers_row_menu` 4.28s, `tst_layers_panel` 11.30s, `tst_layers_panel_fixes` 6.59s, `tst_layers_effects` 5.17s, `tst_layers_mask_commands` 11.81s, `tst_layers_smart_filters` 6.36s).
- [x] 4.2 Run `python3 scripts/check-test-plan.py`; every plan row names an existing test and the suite has no unplanned row. Passed (1 change checked).
- [x] 4.3 Run `./build/pictura --headless --self-test`; summary `passed=470 failed=0 skipped=0`, exit code 0.
- [x] 4.4 Run `cargo fmt --all --check` (pass), `cargo clippy --workspace --all-targets -- -D warnings` (pass), `bash scripts/check-file-size.sh` (pass, 5 allowlisted, no new entry), and `openspec validate --all --strict` (425 passed, 0 failed); `layers_panel_menu.cpp` is 780 lines (was 1045).
