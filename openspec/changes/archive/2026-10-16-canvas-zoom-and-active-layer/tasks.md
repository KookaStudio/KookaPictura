# Tasks: canvas-zoom-and-active-layer

## 1. Layers panel active layer (#119)

- [x] 1.1 Capture the document's active-layer path before the model reset in
      `LayersPanel::refresh`, and restore it when the tree has no selected row
      (falling back to the first row only without an active layer).
- [x] 1.2 Qt Test `tst_layers_panel::backgroundOnlyStaysActive` and
      `activeLayerNotFirstRow`.

## 2. Zoom anchoring (#121)

- [x] 2.1 `ImageView::zoomAt` anchors zoom-in at the cursor and zoom-out at the
      canvas centre.
- [x] 2.2 Qt Test `tst_zoom_tool::wheelZoomInKeepsCursorFixed`,
      `wheelZoomOutAnchorsAtCentre`, `plainClickZoomsInAtClick`,
      `altClickZoomsOutAtCentre`.

## 3. Zoom marquee (#120)

- [x] 3.1 `ZoomToolHandler` implements press/move/release: marquee draw, Space
      reposition, release zooms the rectangle to fill the viewport; a click with
      no drag keeps the step.
- [x] 3.2 Qt Test `tst_zoom_tool::marqueeFillsViewport`.
- [x] 3.3 Update the `ws_zoom_click_anchor` self-test to press and release.

## 4. Zoom tool context menu

- [x] 4.1 `Toolbox::buildZoomMenu` builds the CS6 preset menu for the Zoom slot
      (Fit on Screen, 100%, 200%, Print Size, Zoom In, Zoom Out), driving the
      active canvas and disabled without a document. Right-click opens it on
      release (a popup shown during the press is dismissed by the matching
      release on some platforms).
- [x] 4.2 Qt Test `tst_command_tree::toolboxZoomMenu`.

## 5. Verification

- [x] 5.1 `ctest -R '^tst_zoom_tool|^tst_layers_panel|^tst_command_tree'` and
      `./build/pictura --headless --self-test`.
- [x] 5.2 `bash scripts/verify-fast.sh`.
