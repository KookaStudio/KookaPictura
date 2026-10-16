# Tasks

## 1. Routing seam

- [ ] 1.1 Add `virtual void onContextMenu(ToolServices& ctx, const QPointF& imagePos, const QPoint& globalPos)` to `ToolHandler` (`tool_handler.h`), default no-op; verify the file compiles via a small build of the app crate.
- [ ] 1.2 Add `ToolController::showCanvasContextMenu(const QPointF& imagePos, const QPoint& globalPos)` that resolves the active handler and calls its hook; add `ToolController::contextMenuForTest(imagePos) -> QMenu*` returning the built menu without popping it (mirror `LiquifyCanvas::zoomMenuActions`).
- [ ] 1.3 Change `PicturaMainWindow::showCanvasContextMenu(globalPos)` to split on the pointer: inside the document rect call the controller; on the pasteboard open the canvas-background menu (placeholder until group 7). Verify with a Qt Test that a Zoom right-click over the document routes and a Hand right-click opens nothing.

## 2. Migrate Zoom and Move/Isolation

- [ ] 2.1 Add `ToolServices::showZoomMenu(globalPos)` and `ToolServices::showBrushPicker(globalPos)`; implement both in the frame by delegating to `Toolbox::zoomMenu()` and `OptionsBar::brushPicker()`.
- [ ] 2.2 Move the Zoom preset menu out of the frame chain into `ZoomToolHandler::onContextMenu` (call `ctx.showZoomMenu`); verify the existing Zoom-menu behaviour (`#255`) still passes and the toolbox right-click still opens it.
- [ ] 2.3 Move the Move tool's Isolation Mode entry into `MoveToolHandler::onContextMenu`; add `ToolServices::enterIsolationMode()` implemented by the frame via `layersPanel_->enterIsolation`; verify choosing Isolation Mode isolates the selected layers.
- [ ] 2.4 Add a Qt Test covering Zoom-menu contents (Fit on Screen, 100%, 200%, Print Size, Zoom In/Out) and disabled-with-no-document, plus the Move Isolation entry.

## 3. Move layers-under-the-pointer

- [ ] 3.1 Refactor the layer walk in `helpers_composite.rs` into a collector returning all hits topmost first; keep `auto_select_path` behaviour identical (existing `tests_auto_select.rs` passes).
- [ ] 3.2 Add bridge `layers_under_layer(view, x, y) -> QStringList` in `impl_layers/auto_select.rs`; add a Rust unit test for ordering and transparent-pixel exclusion.
- [ ] 3.3 Build the Move list menu (topmost first, each entry calls `ctx.selectLayer(path)`) when Auto Select is off; add a Qt Test asserting the list order and that choosing an entry changes the active layer.

## 4. Eyedropper menu

- [ ] 4.1 Implement `EyedropperToolHandler::onContextMenu` with Sample Size (Point, 3×3, 5×5, 11×11, 31×31, 51×51, 101×101) and Sample (All Layers, Current Layer) writing through the tool's options group; add a Qt Test asserting the entries and that a choice updates the options bar and the sampler.

## 5. Crop menu

- [ ] 5.1 Implement `CropToolHandler::onContextMenu` while a crop box is present: Delete Cropped Pixels toggle and Reset Crop; add a Qt Test asserting the toggle flips the option, the options bar reflects it, and Reset Crop restores the box to the whole canvas.

## 6. Slice menu

- [ ] 6.1 Implement `SliceToolHandler` / `SliceSelectToolHandler::onContextMenu`: Combine Slices (enabled only for more than one user slice; disabled with any layer-based slice), stacking-order entries, and Delete Slice; add a Qt Test asserting enablement and the combine result.

## 7. Canvas-background menu

- [ ] 7.1 Implement the canvas-background menu (Black, Dark Gray, Medium Gray, Light Gray, Custom) reusing the `Space+F` palette; wire it as group 1.3's pasteboard fallback; add a Qt Test asserting a choice sets the canvas colour and that it opens only on the pasteboard.

## 8. Paint-tool brush picker

- [ ] 8.1 Add `BrushPresetPicker::popUpAt(const QPoint& globalPos)` beside `popUpUnder`; verify with a Qt Test it opens at the point and stays on screen.
- [ ] 8.2 Implement `BrushToolHandler::onContextMenu` calling `ctx.showBrushPicker(globalPos)`; add a Qt Test asserting a canvas right-click with Brush opens the picker and choosing a preset changes the tip.

## 9. Documentation and integration

- [ ] 9.1 Record the paint-tool canvas picker as a deliberate non-parity affordance in `docs/02-ui-ux/menus.md` (commit with `TASK-ALLOWS-DOCS`) and verify `scripts/guard.sh` passes.
- [ ] 9.2 Register any new test source in `CMakeLists.txt` and run `ctest --test-dir build -R '^tst_' --output-on-failure`; confirm the new suite and the existing toolbox/Zoom tests all pass.
- [ ] 9.3 Run `bash scripts/verify-fast.sh` and confirm fmt, clippy, file-size, guard, and openspec checks pass; update `docs/dev/STATE.md`.
