# Proposal: rotate-view-tool

## Why

The Rotate View tool (issue #59) was catalogued but disabled. photorust has no
source for it, so it is implemented fresh from `docs/03-tools/rotate-view.md`.

## What Changes

- `ImageView`: a view rotation about the widget centre (`setRotation`,
  normalised to (-180, 180]). Zoom and offset stay in the unrotated view
  frame; the paint applies one rotation (`viewRotation()`), overlays draw in
  the same frame, and only input mapping (`widgetToImage`, the new
  `imageToWidget`, wheel zoom anchor, drag panning) undoes it. A compass (red
  needle to the document's top) is drawn while the tool drags.
- `tool_rotate_view.cpp` (new): drag to rotate, Shift snaps to 15°, Esc resets.
- `ToolController::setViewRotation` / `viewRotationChanged` keep the options
  bar in sync; `options_bar_rotate.cpp` (new): Rotation Angle, the Set Angle of
  Rotation dial (`AngleDial`), Reset View.
- The control server maps image points through `imageToWidget`.
- Catalog row enabled; `shift_plain` (117) now presses K as the
  unimplemented key.
- Qt Test `tst_rotate_view`.

## Capabilities

### New Capabilities

- `tools/rotate-view-tool`: the Rotate View tool.

## Impact

- `pictura-app` (C++). No engine change: the document is never touched. No new
  dependency.

## Provenance

Implemented fresh from the CS6 Help (via `docs/03-tools/rotate-view.md`); the
Shift 15° step and Esc reset are the doc's community-sourced additions. CS6
requires OpenGL; Kooka rotates in its QPainter canvas, so the tool is always
available. Ceiling (`ponytail:`): no Rotate All Windows or trackpad rotate
gestures; the angle belongs to the single canvas widget, so it is shared by
the document tabs; the Navigator's view rectangle and the offset clamping stay
axis-aligned.
