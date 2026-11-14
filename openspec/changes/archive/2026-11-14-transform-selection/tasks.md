# Tasks

## 1. Engine: border resample

- [x] 1.1 Write `crates/pictura-render/src/document_ops/layer_ops/selection_transform_tests.rs` covering identity, 2× scale, rotation about the pivot, canvas clipping, soft coverage, and the fully off-canvas case; verify `cargo nextest run -p pictura-render selection` fails before the op exists.
- [x] 1.2 Add `layer_ops/selection_transform.rs` with `transform_selection(selection, rect, quad, interpolation)`: solve the box→quad homography, clip the destination bbox to the canvas, resample the tiled coverage into a canvas-sized `Selection` with zero tiles outside the result; re-export from `layer_ops`, `document_ops`, and `lib.rs`; verify 1.1 passes.
- [x] 1.3 Add a dense-oracle parity test for the tiled resample (same destination bytes as a full-plane resample) and a test that the source `Selection` is unchanged; verify both pass.

## 2. Session: selection target and commit

- [x] 2.1 Write Rust tests in `impl_transform/selection/tests.rs` for begin, commit-replaces-mask, cancel, identity, undo, layer-byte preservation, preview contour, and last-target tracking; verify `cargo nextest run -p pictura-app selection_transform`.
- [x] 2.2 Add `TransformTarget { Layer, Selection }` to `TransformSession` and set it in `prepare_transform` and every test struct literal; verify `cargo nextest run -p pictura-app transform` and `cargo test -p pictura-app --lib` pass.
- [x] 2.3 Add `begin_transform_selection` and `commit_selection_transform` in a new `impl_transform/selection.rs`, keeping `session.rs` inside its size cap: require a non-empty document-sized selection, build a session with empty members, the selection bounds as the box, no lift, and no moving image; commit through `Edit::Meta` with the `Transform Selection` label; verify 2.1 passes and `bash scripts/check-file-size.sh` is clean.
- [x] 2.4 Add `transform_session_is_selection` and a preview-contour bridge that maps the cached 50 % contour through the session map (homography, or warp + effective frame); verify the contour test covers a rotated box, a warped box, and no session.
- [x] 2.5 Extend `RepeatTransform` with the target kind and add a `transform_last_target` readout; verify the test that enables Again for a selection target and keeps it disabled for projective/warped commits.

## 3. Command, overlay, and modal lock

- [x] 3.1 Create `crates/pictura-app/cpp/tests/tst_transform_selection.cpp` with the base cases (`menuEnabledWithSelection`, `menuDisabledWithoutSelection`, `menuDisabledWhileModal`, `beginPlacesTheBoxOnTheSelectionBounds`, `scaleWithShiftCommitsProportionally`, `widthFieldDoublesTheBorder`, `rotationKeepsTheReferenceCorner`, `commitRecordsOneStateAndReplacesTheBorder`, `commitLeavesPixelsUntouched`, `undoRestoresThePreviousBorder`, `identityCommitRecordsNothing`, `offCanvasCommitClearsTheSelection`, `softCoverageStaysSoft`, `escapeRestoresTheBorder`, `previewFollowsTheDrag`, `committedOutlineMatchesTheMask`, `cancelKeepsHistoryAndSelection`, `selectMenuLockedWhilePending`, `toolSwitchPromptsForTheBorder`, `fieldsSeedFromTheSelectionBounds`, `applyRecordsTransformSelection`) and register it in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify the target builds.
- [x] 3.2 Flip `select.transform.selection` to `implemented: true`, add `SelectTransformSelection` to the const-name table, regenerate `command_ids.h` and both catalogue goldens; verify `cargo nextest run -p pictura-script`.
- [x] 3.3 Wire the Select handler and enabled provider in `frame_menus_select.cpp` and lock the Select menu while a selection transform is pending in `frame_transform_lock.cpp`; verify the menu, lock, and prompt cases pass.
- [x] 3.4 Add `ToolController::beginTransformSelection`, an overlay-only `ImageView` start (no moving image, no image-preview matrix), the ant preview from the contour bridge, and the committed-contour hide/restore; verify the begin, scale, width, rotate, escape, commit, undo, preview, and contour cases pass.
- [x] 3.5 Verify the options-bar cases on a border session (`fieldsSeedFromTheSelectionBounds`, `applyRecordsTransformSelection`) and that `tst_transform_options` still passes.

## 4. Warp on the border

- [x] 4.1 Write `pictura-render` tests for `warp_selection` (identity, preset, custom mesh, outside coverage 0, canvas clipping) and `warp_dest_rect` (equals the warped mask's bounds); verify `cargo nextest run -p pictura-render warp_selection`.
- [x] 4.2 Add `warp_selection` and `warp_dest_rect` to `selection_transform.rs`: crop the selection bounds, triangulate the warp + effective frame, warp the coverage with `warp_gray` (outside 0), and blit the result; verify 4.1 passes.
- [x] 4.3 Make `refresh_warp_preview` and `transform_enter_warp` skip the moving image for a selection session, and make `transform_leave_warp` move the box to `warp_dest_rect` with a `warp_post` frame; verify the Rust enter/leave test and that the layer warp tests still pass.
- [x] 4.4 Add the Qt warp cases (`warpEntersOnTheSelection`, `warpPresetAndDragReshapeThePreview`, `warpCommitResamplesTheMask`, `warpLeaveKeepsTheBorder`, `warpComposesWithPerspective`) and skip `updateTransformPreviewLayer` for a selection session; verify the suite passes.

## 5. Transform Again and Transform Warp

- [x] 5.1 Add the Qt cases `againRepeatsABorderTransform` and `warpKeepsAnOpenBorderSession`; verify they fail before the wiring.
- [x] 5.2 Make `Edit > Transform > Warp` enter Warp on an open modal session instead of beginning a layer session; verify `warpKeepsAnOpenBorderSession` and the existing `tst_transform_warp`.
- [x] 5.3 Make `Transform Again` target-aware: begin a selection session when the last transform targeted the border and a selection is active; verify `againRepeatsABorderTransform` and `tst_transform_commands`.

## 6. Docs and integration checks

- [x] 6.1 Update `docs/dev/doc-code-parity-gaps.md` `TOOL-2` to its shipped status with evidence and tick `Select ▸ Transform Selection (#526)` in `ROADMAP.md`; verify the docs guard accepts the commit's `TASK-ALLOWS-DOCS` marker and `python3 scripts/check-milestone-names.py`.
- [ ] 6.2 Run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`; verify both pass with every scenario covered.
- [ ] 6.3 Run `bash scripts/verify-fast.sh` and `bash scripts/verify-full.sh`; verify the new Qt suite and every existing transform/selection suite pass.
