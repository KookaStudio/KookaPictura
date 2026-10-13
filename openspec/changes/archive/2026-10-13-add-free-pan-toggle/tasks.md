# Tasks

## 1. Shared clamp policy

- [x] 1.1 Add a `freePan` parameter (default `true`) to `offsetRangeFor` in `crates/pictura-app/cpp/canvas_range.h`: when `false` and the scaled axis is non-zero and not larger than the viewport, collapse that axis to the single centred value `(viewport − scaled) / 2`; leave overflowing and zero-size axes on the existing range. Add `#include "canvas_range.h"` to `crates/pictura-app/cpp/tests/tst_canvas_pan.cpp`. Verify with a new Qt Test case calling `offsetRangeFor` directly: a fitting axis returns `min == max == centre`, an overflowing axis is byte-for-byte unchanged, and a zero-size image is unaffected (`ctest --test-dir build -R '^tst_canvas_pan$' --output-on-failure`).
- [x] 1.2 Add a `ponytail:` comment naming the ceiling of the collapse rule (only fires when the axis fits and is non-empty; the 96 px margin is otherwise untouched).

## 2. Canvas honours free pan

- [x] 2.1 Add `ImageView::freePan_` (default `true`), `setFreePan(bool)` that runs `clampOffset()`, `update()`, then `emit viewChanged()`, and a `freePan()` getter in `image_view.h`; make `clampOffset()` in `image_view_document.cpp` pass `freePan_` to `offsetRangeFor`. Verify: extend `tst_canvas_pan.cpp` — a fitting image with `setFreePan(false)` followed by a large `panBy` stays at the centred offset, and with `setFreePan(true)` the same pan moves it within the reveal margin.
- [x] 2.2 Call `clampOffset()` from the document-size replacement paths `ImageView::replaceDocument()` and `ImageView::replaceImage()` in `image_view_document.cpp` so a resize/crop to a fitting size is re-centred. Verify: a `tst_canvas_pan.cpp` case resizes an overflowing document to fit with Free Pan off and asserts the offset is centred.
- [x] 2.3 Pass `view_->freePan()` at all three `offsetRangeFor` call sites in `canvas_scrollbars.cpp` (both `valueChanged` handlers and `syncFromView`), adding `#include "canvas_scrollbars.h"` to the test. Verify: a `tst_canvas_pan.cpp` case builds a `CanvasScrollBars` over an `ImageView` and asserts `horizontalBarForTest()->maximum() == 0` when the fitting document has free pan off and `> 0` when it is back on.

## 3. View menu toggle

- [x] 3.1 Add `ViewFreePan = "view.freePan"` to `commands.h` and a checkable `CommandSpec` for `{ "View", "Free Pan" }` in `command_tree.cpp`, placed after `100%` / `Print Size` and before the Screen Mode separator. Verify: extend `tst_command_tree.cpp` to assert the command exists under View and is checkable.
- [x] 3.2 Add `bool freePan_ = true;` to `frame.h`; load it from the session in `PicturaMainWindow::loadGuideSession()` in `frame_guides.cpp` (not `frame.cpp`), and push it onto each new canvas in `addDocument` (alongside the `gpuCompute_`/`colorPolicy_` block, keeping `frame.cpp` minimal for its size cap). In `frame_menus.cpp` register the `ViewFreePan` handler (flip the flag, apply to every open document's canvas, `saveSession()`, `registry_->refresh()`) and a `setCheckedProvider`. Verify: a `tst_canvas_pan.cpp` main-window case toggles the command and asserts the active canvas's `freePan()` and offset match the new state.

## 4. Persistence

- [x] 4.1 Add `bool freePan = true;` to `SessionState` in `session.h`, load it with a `true` default in `session.cpp`, and write it in the save path; assign `state.freePan = freePan_;` in `frame_session.cpp`. Verify: extend `tst_session_store.cpp` to toggle Free Pan, trigger a save, reload `loadSession()`, and assert the value round-trips (using the existing state-home fixture).

## 5. Integration

- [x] 5.1 Reconfigure and build with CMake, then run `ctest --test-dir build -R '^tst_' --output-on-failure` and confirm the new canvas-pan, command-tree, and session-store cases pass and no existing suite regresses.
- [x] 5.2 Run `bash scripts/verify-fast.sh` and confirm fmt, clippy, the unified test report, file-size, guard (including `openspec validate` and the self-test budget) all pass.
