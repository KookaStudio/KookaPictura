# Tasks

## 1. Zone logic (`crop_grip.h`)

- [x] 1.1 Add `kCropFarMarginPx = 100.0` and bound `cropRotateZone` to the ring `(inner margin, outer margin]` in screen pixels at any zoom, updating its doc comment; verify the header still compiles with `cmake --build build --parallel`
- [x] 1.2 Add `cropFarZone(box, p, zoom)` for points beyond the outer margin, sharing the box-clamp distance math; verify the new predicate has no side effects on a null box
- [x] 1.3 Update `cpp/tests/tst_crop_grip.cpp`: `(200, 200)` is no longer a rotate point, add far-zone cases and a genuine 100 px boundary case; verify `ctest --test-dir build -R '^tst_crop_grip$' --output-on-failure`

## 2. Tool routing (`tool_crop.cpp`)

- [x] 2.1 Add `CursorKind::CursorDefault` and return it for the far zone in `cursorStateAt` for both the active and preview states; map it to the workspace arrow in `hoverCursor`; verify with `tst_crop_cursor` cases asserting the active and preview far cursor kinds
- [x] 2.2 In `onPress` active, branch handle → body → rotate band → far zone, calling `commitCrop()` in the far zone (commit on press); verify with `tst_crop_cursor` far-press cases (on-canvas and workspace-past-canvas) including the release that follows the press
- [x] 2.3 In `onPress` preview, make a far press a no-op that does not start a rotate or adopt the box; verify with the `tst_crop_cursor` preview far-press case that the document is unchanged and no box is adopted
- [x] 2.4 Re-run `ctest --test-dir build -R '^tst_crop_cursor$' --output-on-failure` and confirm the existing rotate/handle expectations still hold

## 3. Handle rendering (`image_view_overlays.cpp`)

- [x] 3.1 Extract a pure glyph-geometry helper (widget-space box → corner L-bracket lines + inset edge bar rects) and unit-test it for handle count, inset, and size constancy; verify the new test passes
- [x] 3.2 Replace the eight centred `drawRect(..., 6, 6)` handles in `paintCropGroupOverlays` with the helper (white over a dark outline), leaving the perspective quad block untouched; verify `ctest --test-dir build -R '^tst_' --output-on-failure` still passes

## 4. Documentation (`docs/03-tools/crop-tool.md`)

- [x] 4.1 Document the L-bracket corner and thin edge-bar handles and add "click outside the box" to the commit triggers; verify the guard accepts the docs change (`bash scripts/guard.sh` with `TASK-ALLOWS-DOCS`)

## 5. Integration verification

- [x] 5.1 Run `openspec validate --all --strict` and confirm the change validates
- [x] 5.2 Run `bash scripts/verify-fast.sh` and confirm fmt, clippy, the unified test report, file-size, guard, and openspec all pass
- [x] 5.3 Confirm no `scripts/file-size-allowlist.txt` entry was needed (`tool_crop.cpp` stays under the code cap)

## 6. Review fixes

- [x] 6.1 Preview cursor and press must share the 8 px resize margin (the cursor used the 6 px default, so the 6–8 px rim showed the wrong cursor); verify with the preview cursor case in `tst_crop_cursor`
- [x] 6.2 Draw the handle glyphs before the box outline and clamp the shadows to the box, enable antialiasing for the glyph pass, and skip degenerate (zero-width/height) boxes; verify `ctest -R '^tst_crop_grip$'` still passes and the outline is unbroken
- [x] 6.3 Add a MODIFIED delta for the main spec's "Crop rotate cursor zones" so the eight zones are bounded to the rotate band (previously they were said to partition the whole outside); reword the rotation-gesture requirement to "beyond the outer margin"; verify `openspec validate --all --strict`

## 7. Manual-test follow-ups

- [x] 7.1 Remove the vertical separator to the right of the Crop options-bar Reset button; verify the options-bar crop test still builds
- [x] 7.2 Lighten the crop options-bar separators one step (`QFrame::Sunken` → `QFrame::Plain`)
- [x] 7.3 Show the Crop W/H fields blank when the ratio is unconstrained (they held `1`); add `NumericField::setBlank`/`isBlank` and update `tst_crop_options` to assert the blank default
- [x] 7.4 Arrow keys nudge the crop box in Classic and pan the composite in Modern, via a new `ToolHandler::nudge` hook and `ToolController::nudgeActive`, wired in `PicturaMainWindow::keyPressEvent`

## 8. Manual-test follow-ups (round 2)

- [x] 8.1 Draw the corner brackets as thick as the edge bars (white stroke = bar thickness) and inset their centre line by half a thickness; verify `tst_crop_grip`
- [x] 8.2 Keep the options-bar separators dark grey, one shade lighter than the `separator` token (revert the light `Plain` line)
- [x] 8.3 Snap after the ratio fit and add the composite bounding box as a snap candidate (`PictureView::composite_bounds`); verify `tst_crop_snap`
- [x] 8.4 Reset makes the crop box the whole canvas and blanks the W/H fields; verify `tst_crop_options`

## 9. Handle polish and edge/Alt resize (round 3)

- [x] 9.1 An aspect-ratio edge drag drives its own dimension and derives the other, so edge handles resize instead of snapping back; cover with a `tst_crop_grip` case
- [x] 9.2 Alt-drag mirrors the dragged edge about the box centre (ratio fit stays centred); cover `dragBox(..., mirror)` in `tst_crop_grip`
- [x] 9.3 Draw the corner brackets and edge bars 1 px thinner and at 80% opacity
