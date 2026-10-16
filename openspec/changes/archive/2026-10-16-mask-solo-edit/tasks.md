# Tasks

## 1. Rust view state and bridge

- [x] 1.1 Add `mask_solo: Option<String>`, `mask_level0: Option<PixelBuffer>`, and `mask_pyramid: ViewPyramid` to `PictureViewRust` (`state.rs`) with `Default` init; clear `mask_solo` in `reset_edit_state` (`impl_core.rs`); verify a freshly opened document reports no solo via `mask_solo_get` (Rust test).
- [x] 1.2 Add the `mask_solo_set(path)` / `mask_solo_get()` bridge pair in `impl_layers/layer_masks.rs` (no `build.rs` change — the file is already listed), mirroring `mask_edit_target_*`: a path without a raster mask resolves to nothing, an empty path clears, a stale target self-clears, and no history is recorded; verify a set/get round-trip, the stale-clear, and an unchanged `history_count()` and `history_index()` (Rust test).

## 2. Mask display source (canvas-only)

- [x] 2.1 Create `crates/pictura-app/src/cxxqt_object/impl_core/display.rs` (declare `mod display;` in `impl_core.rs`), move `display_crop` / `display_level_count` / `display_level_size` there unchanged (they stay composite-only), and confirm `impl_core.rs` drops back under its 1200-line cap.
- [x] 2.2 In `display.rs` implement the mask plane assembler — a document-sized sibling of `mask_thumbnail_image` (`helpers_composite.rs:542`) that replicates `mask_value` semantics (`data` in-rect, `default_color` off-rect, `255` disabled/data-less) across RGB at alpha 255 — and `rebuild_mask_solo_source` / `drop_mask_solo_source`; verify with a Rust test on in-rect, off-rect, disabled, and data-less masks.
- [x] 2.3 In `display.rs` implement `canvas_present_image(level, rect)`: the mask-pyramid crop when soloing, else the composite crop; add the `canvas_display_image` bridge in `layer_masks.rs`; verify `display_image` is untouched (composite) and `canvas_present_image` returns the mask crop while soloed (Rust test).
- [x] 2.4 Point `frame_canvas.cpp`'s provider `crop` lambda at `canvas_display_image` and rebuild the mask source in `reset_pyramid` (`impl_core.rs`); bump `canvas_revision` and `frame_revision` on `mask_solo_set` and widen `frame_revision`'s doc comment; verify with a Rust `canvas_view_test.rs` test that a soloed crop is greyscale/opaque, the level count/sizes match, `image()` stays the composite, and a recomposite keeps the mode.

## 3. Live mask edits

- [x] 3.1 In `apply_refreshed_region` (`impl_paint.rs`) patch the mask level-0 region and `mask_pyramid.update` when soloed, and route the present image of `refresh_region` / `refresh_regions` (`impl_core.rs`) through `canvas_present_image`; verify with a Rust test that a mask region edit while soloed presents the new coverage (compare against a fresh mask-plane crop).

## 4. Panel gesture

- [x] 4.1 In the mask-thumbnail block of `layers_panel_events.cpp` apply the guarded order `Shift && Alt` (consume) → `Shift` (toggle enabled) → `Alt` (`setCurrentIndex` + `mask_edit_target_set` + toggle `mask_solo_set`) → plain (set the edit target), and clear the solo state in the other thumbnail paths; declare `altClickMaskThumbnailForTest` in `layers_panel.h` and define it in a new `layers_panel_test_solo.cpp` added to `CMakeLists.txt` (keeps `layers_panel_test.cpp` under its cap).
- [x] 4.2 Push `mask_solo_set("")` from `LayersPanel::clearSolo` (`layers_panel_actions.cpp`) so the shared reset (add/duplicate/group/ungroup/delete, document switch) ends solo; verify with a Qt Test (`tst_mask_edit.cpp`) that an Alt-click selects the row, sets the target and the solo state with no history, a second Alt-click restores, any other thumbnail click restores, `Shift`-click still toggles the mask, `Alt`+`Shift` does neither, and deleting the mask ends solo.

## 5. Mask-thumbnail context menu

- [x] 5.1 In `layers_panel_menu.cpp` route a right-click inside `delegate_->maskThumbRect` to a new `showMaskThumbMenu` (declared in `layers_panel.h`): select the row, set the mask edit target, and build the menu in CS6 order with separators — Enable/Disable Layer Mask (label per `MaskDisabledRole`), Delete Layer Mask, Apply Layer Mask, Add/Subtract/Intersect Mask To/From/With Selection, then the disabled placeholders `Select and Mask…` and `Mask Options…`; verify the item texts/order and the enable/disable label with a Qt Test.
- [x] 5.2 Wire the commands: Delete → `layer_mask_delete_path`, Enable/Disable → `layer_mask_set_enabled_path`, Apply → the new `layer_mask_apply_path`, the selection commands → the new `mask_selection`; enable Apply only when the row has a mask and is not a smart object (`SmartObjectRole`); verify each action with a Qt Test (delete leaves pixels, apply folds alpha, selection combine).
- [x] 5.3 Add `deleteActiveMask()` in `layers_panel_actions.cpp` (no confirmation dialog: `layer_mask_delete_path` on the current mask target) and a `maskEditing()` accessor in `layers_panel.h`; verify a Delete-key press while the mask is the target removes the mask with one undo state (Qt Test) and that a pixel selection does not win.

## 6. Mask to selection

- [x] 6.1 Add `selection_from_mask(mask: &LayerMask, w, h) -> Selection` in `pictura-select` following the mask coverage rule (`data` in-rect, `default_color` off-rect, fully selected when disabled or data-less); verify white=selected, the off-rect fill, a disabled mask selecting the whole document, and a layer without a mask.
- [x] 6.2 Add the `mask_selection(view, path, mode)` bridge in `impl_selection.rs` next to `select_layer_alpha`, building the shape from the mask and combining with `CombineMode` via `combine_mode_from`; verify the Add/Subtract/Intersect combine results with a Qt Test.

## 7. Delete key

- [x] 7.1 In `frame_menus_edit.cpp`'s `handleDeleteKey` insert the mask branch ahead of the selection/layer branches: when `layersPanel_->maskEditing()` is set, call `deleteActiveMask()`, `refresh()`, and consume the key; verify the mask is removed even when a pixel selection also exists.

## 8. Docs

- [x] 8.1 Update `docs/05-layers/layer-masks.md` (the mask-thumbnail context menu, the Delete-key behavior, the load-as-selection combine ops), `docs/05-layers/layer-management-ui.md`, and `docs/02-ui-ux/keyboard-shortcuts.md` (the mask-thumbnail right-click row and the Delete-on-mask row); commit under `TASK-ALLOWS-DOCS`.

## 9. Integration verification

- [x] 9.1 Qt Test: with a masked layer, the canvas presents the mask plane after an Alt-click and the composite after a click elsewhere, and the Channels/Histogram/Navigator sources are unchanged; the mask menu commands behave as specified; run `ctest --test-dir build -R '^tst_mask_edit' --output-on-failure`.
- [x] 9.2 Run `bash scripts/verify-fast.sh` and `openspec validate --all --strict`, and confirm `scripts/check-selftest-budget.sh` still passes with no new self-test checks.

## 10. Follow-up fixes (solo lifecycle, delete-fill, menu, channels)

- [x] 10.1 End a mask solo when the current layer changes: `LayersPanel::syncMaskSolo` runs from the tree's `currentChanged` (`layers_panel.cpp`), and a `Shift`-click toggles the mask's enabled bit without ending the solo (`layers_panel_events.cpp`); verified by `tst_mask_edit::maskSoloFollowsLayerChange`.
- [x] 10.2 A disabled soloed mask renders its stored coverage, not reveal-all white: `mask_sample` (`impl_core/display.rs`) ignores the `disabled` bit; verified by the `display.rs` unit test.
- [x] 10.3 Delete/Backspace with a pixel selection on the mask edit target fills that selection with the background colour (one `Fill` state), deleting the mask only with no selection (`frame_menus_edit.cpp`); verified by `tst_mask_edit::maskContextMenu`.
- [x] 10.4 The mask context menu's Select and Mask… is enabled and opens Refine Edge: `selectAndMaskRequested` is emitted and wired in the frame (`layers_panel_menu.cpp`, `frame_build.cpp`); verified by `tst_mask_edit::maskContextMenu`.
- [x] 10.5 The Layers action-strip Add Layer Mask button Alt-click adds a Hide All mask; the behaviour already held, now guarded by `tst_mask_edit::addLayerMaskStripInverts`.
- [x] 10.6 The Channels panel lists the active layer's raster mask as a `<Layer> Mask` channel row (`Ctrl+\`); selecting the row shows the mask through the view-only solo (`channels_active_mask_name` / `channels_active_mask_thumbnail`, `channels_panel.cpp`), and `Ctrl+\` in the frame toggles it; verified by `tst_channels_panel::maskChannelRow`.
- [x] 10.7 Re-run `bash scripts/verify-fast.sh` and `openspec validate --all --strict`.
