# Proposal: canvas-zoom-and-active-layer

## Why

Three related canvas/tool defects (issues #119, #120, #121):

- **#119** — a document whose only layer is the locked `Background` can end up
  with no active row: the Layers panel rebuilds its model and a
  `selectionChanged` with an invalidated current index clears the document's
  active layer before the panel restores a selection, and the fallback then
  picks the first displayed row rather than the document's active layer.
- **#120** — the Zoom tool only implements `onPress`, so a press-drag-release
  does nothing; CS6 draws a marquee and displays that area at the highest
  possible magnification.
- **#121** — zoom-out keeps the cursor/clicked point fixed, so the image drifts
  toward and off the pointer. Zoom-out should step at the canvas centre; only
  zoom-in stays cursor-anchored.

## What Changes

- **Layers panel (`layers_panel.cpp`).** `refresh()` captures the document's
  active-layer path before the model reset (which clears it) and, when the tree
  has no selected row, restores that path rather than the first displayed row.
  This keeps the `Background` — or any active layer that is not the top row —
  active across a refresh, document switch, or a cleared selection.
- **Zoom tool (`tool_hand_zoom.cpp`).** The handler now implements
  `onPress`/`onMove`/`onRelease`: a drag draws a marquee (a solid rubber band),
  space held mid-marquee repositions it, and on release the view zooms so the
  marquee fills the viewport at the highest magnification that fits it, centred.
  A press-release with no drag keeps the click step (Ctrl/Alt zoom-out).
- **Zoom anchoring (`image_view.cpp`).** `zoomAt` anchors zoom-in at the cursor
  and zoom-out at the canvas centre, so the wheel and the Zoom-tool click step
  no longer drift the image off the pointer. `View > Zoom Out` already centred.
- **Self-test update.** `ws_zoom_click_anchor` now presses and releases (the
  click step is deferred to release so a drag can draw a marquee).
- **Zoom tool context menu (`toolbox.cpp`).** Right-clicking the Zoom slot opens
  the CS6 preset menu — Fit on Screen, 100%, 200%, Print Size, Zoom In, Zoom Out
  — each driving the active canvas (Print Size from the document's resolution,
  CS6's 72 ppi default). The entries are disabled with no document open.

## Capabilities

### Modified Capabilities

- `tools/canvas-tools`: the Hand and Zoom tools requirement distinguishes
  zoom-in (cursor-anchored) from zoom-out (canvas-centred) and adds the
  marquee drag; the Zoom click scenario reflects the release-committed step; a
  new requirement adds the Zoom tool's right-click preset menu.
- `ui/layers-panel`: a new requirement that the panel restores the document's
  active layer when a refresh finds no selected row.

## Impact

- `pictura-app` (C++ only): `tool_hand_zoom.cpp`, `image_view.cpp`,
  `panels/layers_panel.cpp`, and the `ws_zoom_click_anchor` self-test. No engine
  change, no new dependency.
- New Qt Test suites/checks: `tst_zoom_tool` (wheel/click/marquee anchoring), a
  `tst_layers_panel` case (`activeLayerNotFirstRow`, `backgroundOnlyStaysActive`),
  and `tst_command_tree::toolboxZoomMenu`.

## Provenance

Behaviour is from CS6 Help via `docs/03-tools/hand-and-zoom.md` (click steps,
Alt/Option zoom-out, drag a marquee to display that area at the highest possible
magnification, space to reposition). The marquee-fit formula and the 4-device-px
click slop are implementation choices, marked `ponytail:`-free because they are
the natural reading of "highest possible magnification". No Adobe oracle exists
for the pixel result; the checks assert the view transform, not a rendered
image.
