# Tasks

## 1. Routing seam

- [x] 1.1 Add `virtual void onContextMenu(ToolServices& ctx, const QPointF& imagePos, const QPoint& globalPos)` to `ToolHandler` (`tool_handler.h`), default no-op.
- [x] 1.2 Add `ToolController::routeCanvasContextMenu(ImageView* canvas, const QPointF& imagePos, const QPoint& globalPos)` resolving the handler for `canvas`'s view and calling the hook, plus `ToolController::contextMenuForTest(ImageView* canvas, const QPointF& imagePos) -> QMenu*` that builds the menu without popping; add `ToolHandler::hasCanvasContextMenu()` so a menu-less tool's right-click has no effect, and signals `zoomMenuRequested(const QPoint&)`, `brushPickerRequested(const QPoint&)`, `isolationModeRequested()`.
- [x] 1.3 Change `PicturaMainWindow::showCanvasContextMenu` to take the source canvas: update the `contextMenuRequested` connection in `frame.cpp` to capture `entry.canvas`, split document vs pasteboard on `imagePos`, and route the document case through `routeCanvasContextMenu`; wire the three controller signals to `Toolbox::zoomMenu()`, `OptionsBar::brushPicker()`, and `layersPanel_->enterIsolation`.
- [x] 1.4 Create `crates/pictura-app/cpp/tests/tst_canvas_context_menus.cpp` (`QTEST_MAIN`, `CanvasContextMenusTest`) and register it explicitly in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] 1.5 Add case `handOpensNothing`: activate Hand, right-click over the document, assert no menu is produced.

## 2. Migrate Zoom and Move/Isolation

- [x] 2.1 Remove the Zoom branch from `showCanvasContextMenu` and add `ZoomToolHandler::onContextMenu` emitting `zoomMenuRequested(globalPos)`; the frame connects it to `Toolbox::zoomMenu()`.
- [x] 2.2 Add `MoveToolHandler::onContextMenu` with the Isolation Mode entry emitting `isolationModeRequested()`, replacing the frame's `moveContextMenu_`; add case `moveIsolation` asserting the signal fires.

## 3. Move layers-under-the-pointer

- [x] 3.1 Add `layers_under_point` collector in `helpers_composite.rs` returning all hits topmost first; `auto_select_path` is unchanged.
- [x] 3.2 Add bridge `layers_under_layer(view, x, y) -> QStringList` and `layer_name_for_path` in `impl_layers/auto_select.rs` with a Rust `#[test] fn layers_under_point_lists_hits_topmost_first_skipping_non_hits`.
- [x] 3.3 Build the Move list menu (each entry calls `ctx.selectLayer(path)`) when Auto Select is off, always adding Isolation Mode; add cases `moveListsLayers`, `moveChooseSelects`, and `multiDocumentTargetsClicked`.

## 4. Eyedropper menu

- [x] 4.1 Implement `EyedropperToolHandler::onContextMenu` with Sample Size (Point, 3×3, 5×5, 11×11, 31×31, 51×51, 101×101) and Sample (Current Layer, Current & Below, All Layers, All Layers No Adjustments, Current & Below No Adjustments) writing through `EyedropperOptions`; add `eyedropperOptionsChanged` so the bar re-reads; add cases `eyeDropperMenuOpens`, `eyeDropperSize`, `eyeDropperScope`.

## 5. Crop menu

- [x] 5.1 Implement `CropToolHandler::onContextMenu` while a crop box is present: Delete Cropped Pixels toggle and Reset Crop; add cases `cropToggleDelete` and `cropReset`.

## 6. Canvas-background menu

- [x] 6.1 Build the canvas-background colour menu from the same `kCanvasColors` source `Space+F` cycles (`frame_canvas.cpp`), wired as the pasteboard branch of 1.3; add cases `pasteboardBackgroundMenu` and `backgroundColour`.

## 7. Paint-tool brush picker

- [x] 7.1 Add `BrushPresetPicker::popUpAt(const QPoint& globalPos)` beside `popUpUnder` and implement `BrushToolHandler::onContextMenu` emitting `brushPickerRequested(globalPos)`; add cases `brushPickerOpens` and `brushPresetApplies`.

## 8. Documentation and integration

- [x] 8.1 Record the paint-tool canvas picker as a deliberate non-parity affordance in `docs/02-ui-ux/menus.md` (commit with `TASK-ALLOWS-DOCS`).
- [x] 8.2 Run `ctest --test-dir build -R '^tst_' --output-on-failure` and confirm `tst_canvas_context_menus` and the existing toolbox/Zoom suites all pass.
- [x] 8.3 Run `bash scripts/verify-fast.sh` and confirm fmt, clippy, file-size, guard (including `check-test-plan.py`), and openspec checks pass; update `docs/dev/STATE.md`.

## 9. Non-grabbing brush picker

- [x] 9.1 Rebuild `BrushPresetPicker` as an in-window, non-grabbing overlay: drop `Qt::Popup`, parent to the main window, position and clamp in parent coordinates, and dismiss on an outside pointer press (returned unconsumed, so it reaches the canvas) and on Escape via a `qApp` event filter.
- [x] 9.2 Offset `popUpAt` from the pointer so the brush ring stays visible; add the `QWidget#brushPresetPicker` theme rule; add cases `brushPickerIsNotACapturingPopup`, `brushPickerDismissesOnOutsidePress`, `brushPickerKeepsBrushHover`.
- [x] 9.3 Note the non-capturing picker behaviour in `docs/02-ui-ux/menus.md` (with `TASK-ALLOWS-DOCS`).

> 8.2 and 8.3 passed: 104/104 Qt suites green (`ctest -R '^tst_'`), `openspec
> validate --all --strict` 259/259, `check-file-size`, `check-test-plan`, and
> `guard` all pass. Section 9 compiles and its cases run clean; a review pass
> fixed the canvas-menu QMenu lifetime (delete-on-close), made the Crop menu
> gate on an active box so a boxless right-click falls through to the pasteboard
> menu, deduplicated the Auto-Select hit walk into one `collect_hits` collector,
> and finalised an open brush stroke on tool deactivation (an unfinished stroke
> crashed at teardown).
