# Tasks

Order follows `design.md`. Test cases are written before the behavior they
cover; the tree is compiled once at the end.

## 1. Tests first

- [x] Add `tst_layers_row_menu.cpp` with `rowBlocksInOrder`, `backgroundBriefMenu`, `groupRows`, `pixelMaskAndClippingRows`, `linkRows`, `mergeRows`, `rasterizeRows`, `isolationRow`, `rowActionTargetsClickedRow`, `shapeOmitsEditAdjustment`, `plainPixelOmitsSmartRows`, `pixelRowFullSet`, `colorLabelSetsTheRow`, `unimplementedDisabled`.
- [x] Add `tst_layers_panel_bundle.cpp` with `groupInsertsAtTopWithoutSelection`, `duplicateInsertsAboveAndSelects`, `deleteRemovesTheLayer`, `duplicateBackgroundMakesOrdinaryLayer`, `adjustmentInsertsAboveActive`, `fillInsertsAboveActive`, `fillMaskFromSelection`, `fillMaskWhiteWithoutSelection`, `fillMaskOff`, `copyToggleOffKeepsName`, `adjustmentsMenuIncludesAddMaskDefault`, `addMaskByDefaultToggles`, `adjustmentMaskFromSelection`, `adjustmentMaskWhite`, `adjustmentMaskOff`.
- [x] Add `tst_canvas_backdrop.cpp` with `transparentDocumentShowsCheckerboard`, `opaqueContentCoversCheckerboard`, `hiddenBackgroundShowsCheckerboard`.
- [x] Extend `tst_layers_snapshot.cpp` with `adjustmentThumbnailKindIcon`, `fillThumbnailPreviews`.
- [x] Extend `tst_layers_mask_commands.cpp` with `groupMaskThumbnailActivates`, `groupMaskPaintRoundTrip`.
- [x] Add Rust tests in `pictura-render`: `add_layer_mask_group_is_document_sized`, `group_mask_round_trips_through_the_adapter`.
- [x] Register the two new Qt suites in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] Move the existing `CommandTreeTest::menusPanel` row-menu assertions into `tst_layers_row_menu.cpp` and keep `tst_command_tree.cpp` under 1400 lines.
- [x] Move the new #895/#898 cases out of `tst_layers_panel.cpp` so it stays under 1400 lines.

## 2. #900 canvas backdrop

- [x] Delete `ImageView::setCanvasBackdrop`, `canvasBackdrop_`, `canvasBackdropOpaque_`, and the opaque branch in `image_view.cpp`; delete `ToolController::updateCanvasBackdrop` and its calls.
- [x] Update the canvas-backdrop test to render the widget and sample a checker tone after hiding the Background.

## 3. #897 group masks

- [x] Size Reveal All / Hide All / From Transparency masks to the document rectangle for a group target; keep selection masks at the selection rectangle.
- [x] Confirm `mask_document` / `write_mask_back` round-trip a group mask; extend the mask menu/row target test.

## 4. #898 anchored creation

- [x] Add `pictura_render::insert_layer(doc, selection_path, layer)`; delegate `insert_node` through it.
- [x] Anchor `add_adjustment`, `add_solid_fill`, and `add_gradient_fill` on `layer_target.anchor()`; select the created path.
- [x] Keep `insert_layer` available to the fill editor creation path (Change B).

## 5. #895a mask preferences and target

- [x] Add `PictureViewRust::add_mask_by_default` (default true) and extend `set_layers_panel_options`; persist it in the session beside `layersUseDefaultMasksOnFill`.
- [x] Make the Adjustments panel's `Add Mask by Default` row a checkable action routed through the Layers panel, with the checked state set on `aboutToShow`.
- [x] Add `default_layer_mask(doc, selection)`: selection mask when one exists, else a white document-sized mask.
- [x] Use `add_mask_by_default` in `add_adjustment`; set `mask_edit_target` to the created path when a mask exists, clearing solo.
- [x] Keep fills on `use_default_masks`; set the fill's active thumbnail to the layer.

## 6. #894 thumbnails

- [x] Add `adjustment_kind` to `LayerRow` (Rust), `layers_api::LayerRow` (C++), the model role, and `LayerRowNode`.
- [x] Generate it with a helper over `decode_adjustment`: solid color / gradient / pattern fill kinds map to `solid-color`/`gradient`/`pattern`; other adjustments map to their kind key.
- [x] Delegate: paint `adjustments.<kind>` on a neutral card for adjustment rows; keep the checkerboard for pixel/smart/fill rows and draw the fill preview image.
- [x] Add `fill_thumbnail_image` in Rust: flat solid, `gradient_rgba` at thumbnail extent, scaled pattern tile; decode patterns once per snapshot.
- [x] Keep shape, type, and group thumbnails unchanged.

## 7. #720 row menu

- [x] Replace `kRowSpecs`'s flat slicing with a `block` field; emit one separator between non-empty blocks.
- [x] Build the non-Background table in the design's block order.
- [x] Add the Background brief table and route `Layer from Background…` through `openBackgroundConversion`.
- [x] Wire `groupFromLayers`, `link`/`unlink`, `selectLinked`, `mergeDown`/`mergeLayers`, `mergeVisible`, `mergeClippingMask`, `flatten`, and `releaseIsolation`.
- [x] Add `vector_mask_add_path`; use it for the row menu's Add Vector Mask.
- [x] Enable `rasterizeType` and add the per-kind `Rasterize Layer` dispatch; keep `Rasterize Layer Style` disabled with the tooltip.
- [x] Select an unselected right-clicked row before a row action runs.
- [x] Switch the merge label between Merge Down and Merge Layers, and the link label between Link and Unlink.
- [x] Update `tst_layers_panel`, `tst_layers_panel_fixes`, `tst_layers_mask_commands`, `tst_smart_object_actions`, and `tst_command_tree` expectations.

## 8. Docs

- [x] `docs/02-ui-ux/panels/layers-panel.md`: row menu block order, brief Background menu, remove Copy CSS (CC-only), note the adjustment/fill thumbnails.
- [x] `docs/05-layers/layer-management-ui.md:175`: remove Copy CSS from the CS6 row menu.
- [x] `ROADMAP.md`: tick the shipped Layers-panel items; leave pattern presets/#262/#599 open.
- [x] Commit the docs change with `TASK-ALLOWS-DOCS`.

## 9. Verification

- [x] `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] `cargo nextest run --workspace` and `cargo test --workspace --doc`.
- [x] Build once with CMake and run the new Qt suites, then `ctest -R '^tst_'`.
- [x] `openspec validate --all --strict`, `python3 scripts/check-test-plan.py`, the file-size check, and `bash scripts/verify-fast.sh`.
- [x] Archive both changes and update `docs/dev/STATE.md`.
