# Tasks

Test cases are written before the behavior they cover; the tree is compiled
once at the end.

## 1. Tests first

- [x] Add `tst_fill_layer_editor.cpp` with `solidApply`, `solidCancel`, `gradientApply`, `gradientEditorOpens`, `gradientCancel`, `gradientPersistence`, `patternApply`, `patternSnapToOrigin`, `patternEmpty`, `layerFillMenuPatternGating`, `patternIconGating`, `createFromLayerMenu`, `createCancel`, `createFromPanel`, `contentOptionsOpensEditor`, `thumbDoubleClickOpensEditor`, `adjustmentThumbOpensProperties`, `maskThumbKeepsTarget`, `backgroundDoubleClickConverts`.
- [x] Register `tst_fill_layer_editor` in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] Add Rust tests in `pictura-render`: `encode_gradient_fill_params_round_trips`, `encode_gradient_fill_params_omits_defaults`, `encode_pattern_fill_round_trips`, `encode_pattern_fill_omits_default_origin`.
- [x] Add app Rust tests for the fill bridge: read/write each kind, a no-op write records nothing, an unknown path is refused, and creation anchors/selects.
- [x] Update `tst_adjustments_panel::patternIconIsInert` into `patternIconFollowsPatterns`.
- [x] Update `tst_layers_panel::fillAdjustmentMenu` to the new Pattern gating.
- [x] Extend `tst_layers_snapshot` for the fill previews it now shares with Change A.

## 2. Engine encoders

- [x] Add `encode_gradient_fill_params(&GradientFillParams)` writing `Rvrs`/`Scl ` when non-default; keep `encode_gradient_fill` unchanged.
- [x] Promote the `PtFl` encoder from `tests/mod.rs` to `fill::encode_pattern_fill`; delete the test duplicate.
- [x] Round-trip tests for both encoders.

## 3. Bridge

- [x] Add `cxxqt_object/impl_layers/fill_content.rs`: `layer_fill_kind`, `layer_fill_color`, `layer_fill_gradient`, `layer_fill_pattern`, `set_layer_fill_color`, `set_layer_fill_gradient`, `set_layer_fill_pattern`, `document_pattern_rows`, `add_fill_layer`.
- [x] Each write replaces the adjustment block, records one undo state, and recomposites the layer rect (full when empty).
- [x] `add_fill_layer` uses `insert_layer`, the fill default mask, and selects the created path.
- [x] Add the bridge declarations and module registration; keep `cxxqt_object.rs` under its size ceiling by using a free-function bridge module.

## 4. Dialogs

- [x] `gradient_fill_dialog.*`: sample + Gradient Editor, Style, Angle, Scale, Reverse, disabled Dither/Align With Layer, `getFill`.
- [x] `pattern_fill_dialog.*`: pattern grid, Scale, Link With Layer, Snap To Origin, empty state, `getFill`.
- [x] `fill_layer_editor.*`: kind dispatch, `openFillLayerEditor`, `createFillLayer`.
- [x] Add all new sources to `CMakeLists.txt`.

## 5. Creation and dispatch

- [x] `Layer > New Fill Layer > Solid Color/Gradient`: New Layer dialog then the type editor, create once; cancel creates nothing.
- [x] Add `LayerNewFillPattern` and the Pattern submenu entry; gate it on `document_pattern_rows`.
- [x] Adjustments panel fill icons: create with defaults then open the type editor.
- [x] `Layer Content Options` dispatches to the fill editor for fills, Properties for adjustments.
- [x] The Layers-panel content-thumbnail double-click dispatches to the same helper; mask/vector thumbnails keep their behavior.
- [x] Enable/disable the Pattern entries in the Layer menu, panel menu, and Adjustments panel grid from the document's patterns.
- [x] Row menu `Edit Adjustment…` uses the dispatcher.

## 6. Docs

- [x] `ROADMAP.md`: note the pattern fill layer shipped by this change; keep #262/#599 for the preset library and Define Pattern.
- [x] No `docs/` change is needed; `docs/05-layers/fill-layers.md` already contracts the editors.

## 7. Verification

- [x] `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] `cargo nextest run --workspace` and `cargo test --workspace --doc`.
- [x] Build once with CMake and run `tst_fill_layer_editor`, `tst_adjustments_panel`, `tst_layers_panel`, `tst_layers_snapshot`, then `ctest -R '^tst_'`.
- [x] `openspec validate --all --strict`, `python3 scripts/check-test-plan.py`, the file-size check, and `bash scripts/verify-fast.sh`.
- [x] Archive both changes and update `docs/dev/STATE.md`.
