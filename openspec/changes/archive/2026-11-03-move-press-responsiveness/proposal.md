# Proposal

## Why

Issue #1148. On a 16507×16196 document, switching to the Move tool took about
8 s. Activation called `prepare_move_preview`, which baked two images the size of
the moving layer's content: the backdrop with the layer hidden, and the layer
itself. For a layer that covers the canvas, both are the whole document. A press
without the warm rebuilt the same images, so moving the bake to the press would
not have removed it.

## What Changes

- **Switching to the Move tool builds nothing.** The first press builds the
  preview.
- **A large layer previews from the view.** When the moving layer's content is
  larger than 4 MP, the press builds the backdrop only under the layer's
  on-screen part and the moving image only within a quarter view of the screen,
  both at the pyramid level the canvas presents. A drag that brings uncovered
  layer pixels into view rebuilds the moving image.
- **Everything else keeps the full-resolution preview**: a link set, a shape,
  type, smart object or styled layer, small content, no presented view, and,
  above level 0, a document whose reduced copy would draw differently.

## Not in this change

The commit still recomposites the moved area at full resolution on the GUI
thread (about 4.6 s for a full-canvas layer at this size). It needs an
off-thread commit, tracked in #1157.

## Capabilities

### New Capabilities

- `tools/move-press-responsiveness`

### Modified Capabilities

- `document/layer-duplicate-responsiveness`: switching to the Move tool no longer
  builds the preview.

## Impact

`pictura-render` (`FilterProxy::into_document` becomes public,
`FilterProxy::reduces_faithfully`), the app's Move preview
(`impl_transform/session/view_preview.rs`, three bridge functions), the Move tool
handler, and `ImageView`'s move preview, which draws images at a level scale.
