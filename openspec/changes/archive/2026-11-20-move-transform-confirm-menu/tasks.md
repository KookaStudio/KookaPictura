# Tasks

## 1. Tests first (write only; compile after group 4)

- [x] 1.1 Update `tst_transform_controls`: rename `handleDragScalesAndCommits`
  to `handleDragStaysPendingThenCommits` and add
  `rejectCancelsThePendingTransform`, `identityHandleDragReturnsToIdle`,
  `pendingTransformRefinesTheSameSession`, and
  `insideDragMovesWithinThePendingTransform`; cancel any pending session in
  `cleanup()`.
- [x] 1.2 Add `controlsGesturePendingLocks` to `tst_transform_lock`.
- [x] 1.3 Update `tst_transform_overlay::controlsBoxDrawsPastTheDocumentEdge`
  and `tst_linked_layers::linkedControlsBoxResizesTheSet` to confirm the pending
  transform before checking the layer.

## 2. Engine readout

- [x] 2.1 Add the `transform_is_identity` qinvokable and implement it in
  `impl_transform/session/readouts.rs` over `identity_transform`.

## 3. App state machine

- [x] 3.1 Replace `commitTransformControls` with
  `promoteControlsTransform`: clear the C++ and Rust controls flags, show the
  reference point, focus the canvas, and emit `transformChanged`.
- [x] 3.2 In `handleReleased`, cancel an identity controls gesture and promote a
  changed one instead of committing.
- [x] 3.3 Update the stale comments in `tools.h`, `options_bar.cpp`, and
  `frame_transform_lock.cpp`.

## 4. Verify

- [x] 4.1 `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 4.2 Build the app and run the touched Qt suites:
  `tst_transform_controls`, `tst_transform_lock`, `tst_transform_overlay`,
  `tst_linked_layers`, `tst_transform_options`, `tst_transform_selection`.
- [x] 4.3 `python3 scripts/check-test-plan.py` and
  `openspec validate --all --strict`.
- [x] 4.4 Archive the change with `openspec archive move-transform-confirm-menu`.
