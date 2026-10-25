# Tasks

## 1. Reference point in the source frame (#903)

- [x] 1.1 Add the new tests: Qt slots `pivotPresetUsesTheLiveQuad`,
  `pivotSetterUsesDocumentPosition` (also asserting the commit keeps the box in
  place), `pivotFollowsMoveAndScale`, and `pivotStaysDuringRotation` in
  `tst_transform_options.cpp`;
  `againUsesTheSourcePivot` in `tst_transform_commands.cpp`; Rust functions
  `pivot_setter_keeps_the_quad_and_moves_the_marker` in `geometry.rs`,
  `scale_base_carries_the_pivot` in `perspective.rs`, and
  `repeatable_transform_records_the_source_pivot` in `session.rs`. Verify each
  name appears in its suite file and that the unmodified tests fail for the
  expected reason.
- [x] 1.2 Add `MapParams::inverse_map_point` with a singular-determinant
  refusal, and a document-position helper for a session pivot. Verify the Rust
  unit tests for the map pass, including a translated and a scaled map.
- [x] 1.3 Rewrite `set_pivot` to return false on a singular map and to
  inverse-map for `Free`/`Scale`/`Rotate` and to set directly in a projective
  mode, keeping the rendered quad unchanged; handle the result in
  `transform_set_pivot`, `transform_set_pivot_preset`, `gesture_perspective`,
  and `gesture_scale_base` (and ignore a refused set in `pivot_drag`). Verify
  `pivot_setter_keeps_the_quad_and_moves_the_marker`,
  `pivotSetterUsesDocumentPosition`, and a singular map returning false.
- [x] 1.4 Report the pivot's document position in `transform_pivot_x/y`, use it
  in `pivot_hit`, and capture it as `start_pivot` on press. Verify
  `pivotLocatorMovesTheReferencePoint`, `pivotPresetUsesTheLiveQuad`,
  `pivotFollowsMoveAndScale`, `pivotStaysDuringRotation`, and
  `canvasPivotDragSnapsToAnAnchor`.
- [x] 1.5 Carry the pivot in `gesture_scale_base` with the projective
  `carry_pivot` and `set_pivot`. Verify `scale_base_carries_the_pivot`; keep
  `free_scale_anchors_on_the_base` passing.
- [x] 1.6 Record the source-frame offset in `repeatable_transform` and place
  the new session's pivot from it in `transform_repeat`. Verify
  `repeatable_transform_records_the_source_pivot` and
  `againUsesTheSourcePivot`; keep `againRepeatsTheLastTransform` passing.
- [x] 1.7 Update the existing geometry, perspective, and transform options
  tests for the new pivot semantics (including the document-space
  `start_pivot` in the perspective test helper), and make
  `angleAndSkewSettersReachTheSession` use H=30° so it matches the H-skew
  scenario; then run `cargo nextest run -p pictura_app` and confirm the
  transform unit and Qt suites pass.

## 2. Ctrl distort and skew shortcuts (#901)

- [x] 2.1 Add the new tests: `ctrlCornerDistorts`,
  `ctrlShiftCornerDistorts`, `ctrlSideStillScales`, `ctrlCornerOnTypeScales`,
  and `plainDragScalesTheDistortedBox` in `tst_transform_distort.cpp`;
  `ctrlShiftSideSkews` and `ctrlShiftSideKeepsType` in
  `tst_transform_skew.cpp`. Verify each slot exists and the suite fails before
  the wiring lands.
- [x] 2.2 Factor the pure quad helpers `distort_quad` and `skew_quad` out of
  the existing gestures, and move `unmap_quad` from `perspective.rs` to
  `geometry.rs`. Verify the existing gesture unit tests still pass.
- [x] 2.3 Add the transient flags and the `transform_begin_free_distort` /
  `transform_begin_free_skew` bridge functions; like the perspective begin, they
  refuse outside a `Free` session, while a warp is active, and (distort only)
  on editable type. Verify the new Qt slots see non-parallel opposite edges
  (distort), the slid edge (skew), the distort pivot carry, the fixed skew
  pivot, and the correct type refusal.
- [x] 2.4 Wire the C++ press precedence in `tools.cpp`: perspective first,
  then `Ctrl+Shift` on a side handle, then `Ctrl` on a corner. Verify
  `ctrlCornerDistorts`, `ctrlShiftCornerDistorts`, `ctrlSideStillScales`,
  `ctrlShiftSideSkews`, and `freeShortcutDragsPerspective`.
- [x] 2.5 Clear the transient flags on every press and on release while keeping
  `base_quad`, and route later plain drags through `gesture_scale_base`. Verify
  `plainDragScalesTheDistortedBox` and the shortcut commit scenario in
  `ctrlCornerDistorts`.

## 3. Keep the warp across box switches (#902)

- [x] 3.1 Add the new tests: `reenteringWarpKeepsTheMesh`,
  `boxDragKeepsTheWarpedPreview`, `boxScaleCommitsWithTheWarp`,
  `numericBoxEditRefreshesTheWarpedPreview`,
  `exactCommandRefreshesTheWarpedPreview`, and
  `perspectiveBoxWarpPreviews` in `tst_transform_warp.cpp`; extend
  `toggleSwitchesBetweenBoxAndWarp` and `boxTransformCarriesIntoTheMesh`.
  Verify the slots exist and the new retention tests fail before the change
  (the old toggle test may still pass).
- [x] 3.2 Add `warp_editing` to `TransformSession` (initialised false in
  `prepare_transform` and every test fixture) and `transform_warp_stored` to
  `warp.rs`; set `warp_editing` on enter (creating the mesh only when absent)
  and clear it on leave without taking the mesh. Switch `can_set_mode` and the
  transient begin functions (`transform_begin_free_perspective`, `_distort`,
  `_skew`) from `warp.is_some()` to `warp_editing`. Verify
  `reenteringWarpKeepsTheMesh` and the updated
  `toggleSwitchesBetweenBoxAndWarp`.
- [x] 3.3 Gate warp hit-testing, gestures, and the quad readout on
  `warp_editing`, and re-warp the preview after each box gesture when a mesh is
  stored. Verify `boxDragKeepsTheWarpedPreview`, `boxScaleCommitsWithTheWarp`,
  and `boxTransformCarriesIntoTheMesh`.
- [x] 3.4 Re-warp after the numeric W/H/angle/skew setters and after exact
  rotate/flip when a mesh is stored; make `transform_apply` keep the projective
  branch whenever the mode is projective, warp stored or not. Verify
  `numericBoxEditRefreshesTheWarpedPreview`,
  `exactCommandRefreshesTheWarpedPreview`, and the existing exact-command
  tests.
- [x] 3.5 Change the C++ `updateTransformOverlay` to use
  `transform_warp_stored` for the whole-preview choice and keep
  `transform_warp_active` for the grid and pivot; the options bar keeps using
  `transform_warp_active` for its warp page. Verify the warp Qt suite and
  `tst_transform_options` pass.
- [x] 3.6 Move the box to the warped preview's document bounds when Warp is
  left, and keep the preview pixels fixed with a source-space `warp_post`
  applied after the mesh. Apply the post-frame to the preview/commit quad, the
  grid, hit-testing, and drag math; keep the box at `session_quad` in both
  modes. Verify `a_post_frame_keeps_the_preview_quad_while_the_box_moves` and
  `leavingWarpPutsTheBoxOnThePreviewBounds`.
- [x] 3.7 Clear the transform overlay and Move controls box on Cancel, Undo,
  and Redo: add `ToolController::resetTransformUi`, call it from
  `onSessionCancelled`, and add `PictureViewRust::clear_transform_preview` to
  drop the prepared preview. Verify `undoCancelsAndClearsTheTransform`,
  `cancelWarpLeavesNoOverlay`, and `undoAfterWarpCommitRestores`.
- [x] 3.8 Update the warp delta, design, proposal, test plan, and
  `docs/03-tools/move-and-transform.md` for the box-following and cancel/undo
  behavior.

## 4. Integration

- [x] 4.1 Run `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo nextest run -p pictura_app`, and `cargo test -p pictura_app --doc`;
  confirm every suite passes.
- [x] 4.2 Run `cmake -S . -B build -G Ninja && cmake --build build --parallel`,
  then run `ctest --test-dir build -R '^tst_transform_' --output-on-failure`;
  confirm the transform Qt suites pass in the real shell.
- [x] 4.3 Run `openspec validate --all --strict` and
  `python3 scripts/check-test-plan.py`; confirm the change validates and every
  scenario row resolves to a test.
- [x] 4.5 Re-run the focused Rust and Qt transform suites after the warp-bounds
  and cancel/undo fixes, then the full CTest and workspace nextest gates.
- [x] 4.4 Commit the bundle with all three issue numbers (`#903`, `#901`,
  `#902`), open the pull request with the bundle title, and confirm the PR body
  lists the plan's coverage and states that no manual rows remain. The three
  fixes share the session/bridge files, so the code is one bundle commit; the
  main-spec sync and archive are a second docs commit.
