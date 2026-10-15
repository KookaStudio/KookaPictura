# Tasks

## 1. Crop commit and crop-box ports

- [x] 1.1 Add `crates/pictura-app/cpp/tool_sessions.h` declaring the pure-virtual `CropCommitSession` (`commitCrop`) and `CropBoxSession` (`cropActive`, `resizeCrop`, `cropWidth`, `cropHeight`, `resetCropBox`, `cropCenter`, `setStraightenMode`, `hasPendingCrop`, `pendingCropRect`), and list `tool_sessions.h` in `CMakeLists.txt`; verify the app configures and builds with the new header present.
- [x] 1.2 Make `CropToolHandler` (`tool_crop.cpp`) multiply-inherit `ToolHandler`, `CropCommitSession`, and `CropBoxSession`; add `ToolController::cropCommitSession()` (active-aware `PerspectiveCrop ? PerspectiveCrop : Crop`) and `ToolController::cropBoxSession()` (fixed `ToolId::Crop`); retarget the controller's internal crop callbacks `cropCenter`/`setStraightenMode` (`tools.cpp:148-162`) and the inline `hasPendingCrop`/`pendingCropRect`/`cropCenterForTest` (`tools.h:298-307`, `tools.cpp:375-381`) onto `cropBoxSession()`; verify the app builds and `ctest -R '^tst_crop_'` passes.
- [x] 1.3 Retarget the crop calls in `options_bar_crop.cpp` (`commitCrop` → `cropCommitSession()`, and `resetCrop`/`resizeCrop`/`cropWidth`/`cropHeight`/`cropActive` → `cropBoxSession()`) and the Crop `hasPendingCrop`/`commitCrop` wrappers in `frame.{h,cpp}` (`frame.cpp:276-288`) and `frame_menus_image.cpp:22`; verify `ctest -R '^tst_crop_'` passes and the options bar reads the same values.
- [x] 1.4 Retarget the crop test/self-test call sites (`tst_crop_session`, `tst_crop_options`, `tst_crop_snap`, `tst_crop_preview`, `tst_crop_cursor`, `selftest_crop_tool`) onto `cropCommitSession()`/`cropBoxSession()`; verify each passes under `ctest` and `./build/pictura --headless --self-test`.

## 2. Perspective Crop port

- [x] 2.1 Add `PerspectiveCropSession` (`setPerspectiveOutput`, `perspectiveQuadComplete`, `setPerspectiveShowGrid`) to `tool_sessions.h`; make `PerspectiveCropToolHandler` (`tool_perspectivecrop.cpp`) inherit `ToolHandler`, `CropCommitSession`, and `PerspectiveCropSession`; add `ToolController::perspectiveCropSession()` resolving fixed `ToolId::PerspectiveCrop`; verify the app builds and `tst_crop_preview` passes.
- [x] 2.2 Retarget the perspective reads in `options_bar_crop.cpp` (`setPerspectiveOutput:566`, `setPerspectiveShowGrid:594`, `perspectiveQuadComplete:612`) onto `perspectiveCropSession()` and the Perspective Apply onto `cropCommitSession()`; verify the perspective commit/quad behavior is unchanged (`tst_crop_preview` and `selftest_crop_tool` pass).

## 3. Type port

- [x] 3.1 Add `TextSession` (`commitText`, `cancelText`, `textActive`, `insertText`) to `tool_sessions.h`; make `TypeToolHandler` (`tool_type.cpp`) inherit it; add `ToolController::textSession()` resolving the active type tool; verify the app builds and `tst_type_tools` passes.
- [x] 3.2 Retarget `options_bar_type.cpp` (`textActive:231`, `cancelText:282`, `commitText:283`), `panels/glyphs_panel.cpp` (`textActive:170`, `insertText:185`), and the text branch of `frame_menus_edit.cpp:321` onto `textSession()`; verify `ctest -R '^tst_type_'` passes and glyphs insertion still lands at the caret.

## 4. Delete reach-through virtuals and trampolines

- [x] 4.1 Delete the crop, perspective, and type virtuals from `tool_handler.h` and their controller trampolines/aliases from `tools.{h,cpp}` (`commitCrop`, `resetCrop`, `cancelCrop`, `resizeCrop`, `cropWidth`, `cropHeight`, `cropActive`, `setPerspectiveOutput`, `perspectiveQuadComplete`, `setPerspectiveShowGrid`, `commitText`, `cancelText`, `textActive`, `insertText`), keeping the generic `toolUndo`/`toolRedo`/`canTool*` family, `clearAnnotations`, and `nudge`; verify the app builds with no remaining callers and `ctest -R '^tst_'` passes.
- [x] 4.2 Add a Qt Test (`tst_session_ports.cpp`, listed in `CMakeLists.txt`) asserting each port resolver returns non-null for its owning `ToolId` and null for a non-owning tool (e.g. `cropBoxSession()` for a paint tool); verify it passes under `ctest -R 'tst_session_ports'` and would fail if a port implementation were removed.

## 5. Reclassify the generic gesture hooks

- [x] 5.1 Rename `commitPolygonLasso` → `onFinishPath`, `cancelPolygonLasso` → `onEscape`, `removeLassoPoint` → `onDeletePoint`, `lassoInProgress` → `gestureInProgress` across `tool_handler.h`, the controller (`tools.{h,cpp}`, forwarders renamed to `handleFinishPath`/`handleEscape`/`handleDeletePoint`), the overrides (`tool_pen.cpp`, `tool_selection.cpp`, `tool_magneticlasso.cpp`, `tool_slice.cpp`, `tool_rotate_view.cpp`, `tool_path_selection.cpp`, `tool_crop.cpp`, `tool_perspectivecrop.cpp`), and the selftests (`selftest_magnetic_lasso.cpp`, `selftest_tools_selection.cpp`), keeping the `bool` consumed/passthrough contract; verify the app builds and no old name remains.
- [x] 5.2 Clean `frame.cpp`'s Enter/Esc routing (`:1062-1085`: replace the hardcoded `{PolygonalLasso, MagneticLasso, Pen}` list with a dispatch to the active tool's `onFinishPath`, keep the Esc fall-through on the bool) and `frame_menus_edit.cpp`'s Delete routing (`:329`); verify `frame.cpp` stays within its 1200-line cap (`scripts/check-file-size.sh`), and `selftest_magnetic_lasso`, `selftest_tools_selection`, and the crop Esc tests pass.

## 6. Integration verification

- [x] 6.1 Run the full gate set — `cmake --build build`, `ctest --test-dir build -R '^tst_' --output-on-failure`, `./build/pictura --headless --self-test`, `scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`, `scripts/guard.sh`, `openspec validate --all --strict` — and confirm no behavior change and that `tool_handler.h`/`tools.cpp` shrank.
