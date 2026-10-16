# Tasks

## 1. Shared clamp policy

- [x] 1.1 Add a `snapToWorkspace` parameter (default `false`) to `offsetRangeFor`
  in `crates/pictura-app/cpp/canvas_range.h`: when `true` and both scaled axes
  are non-zero and not larger than the viewport, collapse both axes to the single
  centred value; otherwise leave the existing `freePan` policy untouched. Add a
  `ponytail:` comment naming the document-level ceiling.
- [x] 1.2 Extend `crates/pictura-app/cpp/tests/tst_canvas_pan.cpp` with a direct
  `offsetRangeFor` case: a fitting document with the flag on returns the centred
  point on both axes; a document that overflows one axis is byte-for-byte the
  free-pan result; an empty document is unaffected.

## 2. Canvas honours the lock

- [x] 2.1 Add `ImageView::snapToWorkspace_` (default `false`),
  `setSnapToWorkspace(bool)` (re-clamp + `update()` + `emit viewChanged()`), a
  `snapToWorkspace()` getter, and a `workspaceSnapLocked()` query in
  `image_view.h`; thread the flag through `clampOffset()` in
  `image_view_document.cpp`. Verify: a `tst_canvas_pan.cpp` case pins a fitting
  image to centre when on and rejects a large `panBy`.
- [x] 2.2 Gate the middle-button and pan-enabled left-button pan start in
  `image_view.cpp` on `!workspaceSnapLocked()`. Verify: the pan-rejection case
  sends a middle-button drag while locked and asserts the offset is unchanged.

## 3. Workspace scrollbars

- [x] 3.1 Give `CanvasScrollBars` a corner member and, in `syncFromView`, hide
  the two bars and the corner while `view_->workspaceSnapLocked()`, showing them
  otherwise. Verify: a `tst_canvas_pan.cpp` case asserts both bars are hidden
  while a fitting document is locked and reappear once it is zoomed past the
  workspace.

## 4. View menu toggle

- [x] 4.1 Add `ViewSnapCanvasToWorkspace = "view.snapCanvasToWorkspace"` to
  `commands.h` and a checkable `CommandSpec` for
  `{ "View", "Snap Canvas To Workspace" }` in `command_tree.cpp`, placed directly
  after `View > Free Pan`. Verify: extend `tst_command_tree.cpp` to assert the
  command exists under View, is checkable, and defaults off.
- [x] 4.2 Add `bool snapCanvasToWorkspace_ = false;` to `frame.h`; load it in
  `loadGuideSession()` (`frame_guides.cpp`) and push it onto each new canvas in
  `addDocument` (`frame.cpp`, beside the `setFreePan` call). Register the handler
  (flip the flag, apply to every open document's canvas, `saveSession()`,
  `registry_->refresh()`) and a `setCheckedProvider` in `frame_menus.cpp`.

## 5. Persistence

- [x] 5.1 Add `bool snapCanvasToWorkspace = false;` to `SessionState`
  (`session.h`), load it with a `false` default and write it in `session.cpp`, and
  assign `state.snapCanvasToWorkspace = snapCanvasToWorkspace_;` in
  `frame_session.cpp`. Verify: extend `tst_session_store.cpp` to round-trip the
  value.

## 6. Documentation

- [x] 6.1 Add the workspace snap mode to `docs/dev/canvas-view-spec.md` and state
  that wheel zoom-in stays cursor-anchored in both modes (`TASK-ALLOWS-DOCS`).

## 7. Integration

- [x] 7.1 Reconfigure and build with CMake, then run
  `ctest --test-dir build -R '^tst_' --output-on-failure` and confirm the new
  canvas-pan, command-tree, and session-store cases pass with no regression.
- [x] 7.2 Run `openspec validate --all --strict` and
  `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh`; confirm fmt, clippy, the
  unified test report, file-size, guard, and the self-test budget all pass.
