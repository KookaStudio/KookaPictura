# Move preview applies the layer mask

## Why

Dragging a layer that has a layer mask draws the raw, unmasked pixels for the
length of the drag, so the areas hidden by the mask reappear while dragging and
then disappear when the move commits (#1228). The preview and the committed
result disagree.

## What changes

- The Move preview's moving image composites the layer when its channels alone
  are not its appearance (a raster or vector mask, layer style, fill, or type),
  so the mask shapes the preview.
- The view preview (a layer too large to bake whole) applies the mask at the
  presented pyramid level too.

## Capabilities

- `tools/canvas-tools` — the Move preview applies the layer mask.
