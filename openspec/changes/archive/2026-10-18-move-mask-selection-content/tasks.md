# Tasks

## 1. CS6 mask colour pair and reset

- [x] 1.1 Update `tst_mask_edit::colorsFollowTheEditTarget` to expect white foreground / black background when the mask is the active target and to restore the image pair when the layer thumbnail is clicked.
- [x] 1.2 Update the Delete-fill case in `tst_mask_edit::maskContextMenu` (not `colorsFollowTheEditTarget`) to set an explicit white mask background before the revealing fill, keeping the assertion that the mask survives.
- [x] 1.3 Add `tst_mask_edit::resetColorsFollowTheEditTarget`: with a mask active, `Toolbox::resetForegroundBackground()` yields white/black; switching back to the layer yields black/white.
- [x] 1.4 Add a mask-active flag to `ColorState` (`panels/color_panel.h`), set it from the existing `editTargetChanged` handler in `frame_build.cpp`, and make `ForegroundBackgroundWidget::resetColors()` use white/black while a mask is active and black/white otherwise.
- [x] 1.5 Initialize `otherForeground_` / `otherBackground_` in `frame.h` to white/black and update the stale comments in `frame.h`, `frame_build.cpp`, and `toolbox.h`; verify the pair swap logic in `editTargetChanged` remains symmetric.

## 2. Mask coverage move engine

- [x] 2.1 Add `move_mask_selection_translates_and_fills_background` in `crates/pictura-render/src/document_ops/layer_ops/mask_edit.rs` covering translation and background fill.
- [x] 2.2 Add `move_mask_selection_blends_partial_coverage` with a 50% selection edge and hand-computed source/destination bytes.
- [x] 2.3 Add `move_mask_selection_clips_at_the_mask_rect` using a mask with a non-zero rectangle origin and a destination clipped at the mask edge.
- [x] 2.4 Add `move_mask_selection_duplicate_keeps_the_source` covering Alt copy semantics.
- [x] 2.5 Add `move_mask_selection_leaves_pixels_and_mask_metadata_unchanged` asserting the layer channels, mask attachment, and mask rectangle are untouched.
- [x] 2.6 Add `move_mask_selection_materializes_missing_data` with `mask.data = None` and a wrong-length plane, both materialized from `default_color`.
- [x] 2.7 Add `move_mask_selection_refuses_a_mismatched_selection_plane` with a passed coverage plane of the wrong length.
- [x] 2.8 Add `move_mask_selection_ignores_layer_locks` pinning that position/pixel locks do not block a mask coverage move, matching mask paint and fill.
- [x] 2.9 Add `move_mask_selection_works_on_a_group_mask` with a hand-built non-empty mask on a group layer.
- [x] 2.10 Implement `move_mask_selection_content` and export it through `layer_ops/mod.rs`, `document_ops/mod.rs`, and `lib.rs`; verify the nine new engine tests pass in 4.1.

## 3. Bridge and app wiring

- [x] 3.1 Extract the selection-move bridge declarations (`begin_selection_move`, `preview_selection_move`, `commit_selection_move`, `cancel_selection_move`, `move_selection_content`) from `cxxqt_object.rs` into a new module under `impl_selection/`, following the free-function bridge pattern in `impl_layers/layer_masks.rs`; update the C++ call site and verify `scripts/check-file-size.sh` passes.
- [x] 3.2 Add `tst_mask_edit::moveSelectionOnMaskMovesCoverage` using `paint_fixture::Fixture` (for the canvas): add a solid layer, add a hide-all mask (auto-target), refresh the panel and assert the white/black mask pair, paint a white patch, Ctrl-A, drag with the Move tool, and assert the patch moved, the vacated area is hidden by the black background, the mask survives, the layer pixels are intact, exactly one `Move Selection` state is recorded, undo restores the mask and selection, a zero-offset drag records nothing, and a release at a moved point without a final move event leaves the selection at the offset.
- [x] 3.3 Add `tst_mask_edit::altDragCopiesMaskCoverage`: Alt-drag the mask selection, assert the source coverage remains, the copy appears at the offset, `canvas->movePreviewActive()` is false, the layer count and active layer are unchanged, and exactly one state is recorded.
- [x] 3.4 Add `tst_mask_edit::ctrlDragOnMaskMovesCoverage`: with the Marquee tool active, Ctrl-drag inside the selection and assert the mask coverage moves and no new selection starts.
- [x] 3.5 Add the `background` argument and the mask branch to `PictureView::move_selection_content`; crop the origin selection to the mask rectangle, call the engine, clear `selection_move_origin`, `recomposite`, and `record("Move Selection")`. Return before the pixel post-processing so the mask branch never runs the `layer_above` repoint. Verify via 3.2-3.4.
- [x] 3.6 Make `begin_selection_duplicate_preview` return false on a mask target so no pixel-clone preview is built.
- [x] 3.7 Pass the current background ARGB from `ToolController::releaseSelectionMove` in `tools_selection_move.cpp`, and call `preview_selection_move(dx, dy)` before the content-move engine call so the selection follows the release offset. Verify 3.2-3.4.

## 4. Verification and archive

- [x] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p pictura-render -p pictura-app`, and `ctest --test-dir build -R '^tst_mask_edit$' --output-on-failure`; all must pass.
- [x] 4.2 Run `python3 scripts/check-test-plan.py`, `bash scripts/check-file-size.sh`, `bash scripts/guard.sh`, `openspec validate --all --strict`, and `bash scripts/verify-full.sh`; all must pass.
  Note: `verify-full.sh` was aborted by the user; every verify-fast constituent gate was run individually and passed. The full Qt suite passed 108/109; the remaining `tst_keyboard_shortcuts` abort is a pre-existing Qt 6.11 `QChar` assertion on a non-BMP shortcut key and is unrelated to this change.
- [x] 4.3 Archive with `openspec archive move-mask-selection-content --yes`, then run `bash scripts/check-openspec-archived.sh` and confirm `openspec list --json` reports no active change.
