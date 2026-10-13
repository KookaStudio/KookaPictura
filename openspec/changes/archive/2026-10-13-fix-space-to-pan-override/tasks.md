# Tasks

## 1. Make the Space override authoritative

- [x] 1.1 In `ImageView::setPanEnabled` (`crates/pictura-app/cpp/image_view.cpp:102`), ignore a disable while `spacePan_` is set, so `applyToolPolicy` cannot turn panning off mid-hold; verify by code inspection that `setPanEnabled(false)` is a no-op while `spacePan_` is true.
- [x] 1.2 In `ImageView::mousePressEvent` (`image_view.cpp:814`), let `spacePan_` bypass the `panEnabled_` gate so the left press pans regardless of the active tool; verify the existing `tc_space_pan` self-test still passes and a Space pan works with a non-Hand tool.
- [x] 1.3 In `ImageView::mouseMoveEvent` (`image_view.cpp:835`), skip emitting `mouseMoved` while `spacePan_` is set and no mouse button is down, so hover cannot clobber the hand cursor; verify a button-held drag still emits moves (via `tc_space_pan`).
- [x] 1.4 In `ImageView::mouseReleaseEvent` (`image_view.cpp:869`), re-assert `Qt::OpenHandCursor` on a middle-button release while `spacePan_` is set instead of `unsetCursor()`; verify by inspection it matches the left-button branch at line 865.
- [x] 1.5 Add `bool isPanning() const { return panning_; }` (or an equivalent `reassertSpacePanCursor()` helper) to `crates/pictura-app/cpp/image_view.h`; verify the project still builds (`cmake --build build --parallel`).
- [x] 1.6 In `PicturaMainWindow::keyReleaseEvent` (`crates/pictura-app/cpp/frame.cpp:1115`), guard the Space case with `!event->isAutoRepeat()`, matching the press handler at line 1049, so an X11 auto-repeat release cannot clear `spacePan_` mid-hold; verify with task 3.3.

## 2. Guard the cursor funnel

- [x] 2.1 In `ToolController::refreshCursor` (`crates/pictura-app/cpp/tools_marquee.cpp:37`), after the `canvas_` null check, return the hand cursor when `canvas_->spacePanForTest()` (closed while `isPanning()`, open otherwise); verify that a tool switch while Space is held leaves the cursor `Qt::OpenHandCursor`.

## 3. Regression test and verification

- [x] 3.1 Add a slot to `crates/pictura-app/cpp/tests/tst_canvas_pan.cpp` (`tst_canvas_pan` is already registered) that holds Space with the Brush active, asserts `Qt::OpenHandCursor`, switches to the Move tool, and re-asserts the hand cursor and `panEnabled()`; run `ctest --test-dir build -R '^tst_canvas_pan' --output-on-failure`.
- [x] 3.2 Extend that slot to press/move/release on the canvas after the tool switch and assert `offset()` changed with no paint committed, then release Space and assert `panEnabled()` is false and the cursor is restored; run the same CTest command.
- [x] 3.3 Send a Space `QKeyEvent(KeyRelease, Qt::Key_Space, Qt::NoModifier, /*autorep=*/true)` while Space is held and assert `spacePanForTest()` and `panEnabled()` are still true (the override survives auto-repeat); run the same CTest command.
- [x] 3.4 Run `openspec validate --all --strict` and `bash scripts/verify-fast.sh` and confirm both pass; report any golden baseline change explicitly.

## 4. Post-review hardening

- [x] 4.1 End any transient Space pan when a Free Transform session starts (`tool_transform.cpp:68`), so the session's `setPanEnabled(false)` is not swallowed by the override guard.
- [x] 4.2 End the override when the canvas loses focus or the window deactivates (`frame_build.cpp`), so a missed Space key-up cannot leave every later press routed to panning.
- [x] 4.3 Keep `Qt::ClosedHandCursor` when `setSpacePan(true)` runs during an in-progress middle-button drag (`image_view.cpp`).
- [x] 4.4 Add `tst_canvas_pan::spacePanHoverAndMiddleButton` covering the no-button hover and the Space + middle-button paths (including Space pressed mid-drag); run `ctest --test-dir build -R '^tst_canvas_pan'`.
