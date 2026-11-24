# Proposal: type-move-layer-edges

## Why

Moving a type layer on a large document can leave the dashed Layer Edges box
behind: the box is drawn from the layer's ink, but the Move preview and damage
used the whole paragraph box, and the outline cache keyed only on the canvas
revision, which a deferred move does not change before the outline is refreshed.
The outline then stays at an older place while the text has moved.

## What Changes

- The Move path trims any type layer whose reach is the raster alone to its ink,
  not only a box that nearly fills the canvas.
- The Layer Edges cache also keys on the active layer's rect, so a move that
  changes the rect updates the outline even when the canvas revision does not
  move first.
- A deferred commit keeps the Move preview after the document has moved; the
  overlay refresh now recognises the committed outline and stops adding the drag
  delta a second time.
- `tst_move_feedback::largeTypeMoveKeepsLayerEdgesOnTheText` pins the outline on
  a 4000 × 4000 document through the drag and after the commit.

## Capabilities

### Modified Capabilities

- `ui/view-menu`: a type layer's Layer Edges outline follows the text through a
  Move on a large document.

## Impact

- `pictura-app` Move bounds and Layer Edges cache, plus `pictura-render`'s damage
  helper. Qt Test only; no document-format change.

## Provenance

Behavior follows the existing Layer Edges and Move contracts; no external
implementation is used.
