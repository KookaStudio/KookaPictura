# Move preview keeps the stack order

## Why

Dragging a layer that sits below other layers with the Move tool draws the
moving content over those layers, so they disappear for the length of the drag;
the preview should keep the document's stacking order and only move the dragged
content (#1250). The same drag can leave stale pixels of the previous drag frame
at its old place until the button is released.

## What changes

- The Move preview draws the layers above the moving group over the moving
  image, so a layer below other layers does not hide them while it moves.
- The marching-ants repaint covers the previous frame's moving-layer rectangle,
  so a repaint limited to the ants rectangle cannot leave the previous frame.

## Capabilities

- `tools/canvas-tools` — the Move preview keeps the layer stack order.
- `ui/document-canvas` — a partial repaint during a Move drag clears the previous
  frame.
