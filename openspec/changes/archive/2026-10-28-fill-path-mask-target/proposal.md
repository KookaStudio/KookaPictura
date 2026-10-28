# The shared fill path fills the active mask

## Why

With a layer mask's thumbnail active, the Gradient tool did nothing and the
status bar said "Could not fill: the layer's pixels are locked." (#1022). A type
layer is created with its pixels locked (`add_type_layer` sets
`LockFlags::PIXELS`), and the shared fill path behind the Gradient, the Paint
Bucket, `Edit ▸ Stroke`, and the path and shape fills refused any mask edit
target outright — only `Edit ▸ Fill` wrote a mask. `Edit ▸ Fill` already fills a
text layer's mask, so the Gradient refusing it is a bug, not CS6 behaviour: CS6
fills a mask with the Gradient, the Paint Bucket, and `Edit ▸ Stroke` too.

## What Changes

- The shared fill path (`fills::apply`) resolves a mask edit target and runs the
  fill on the mask's coverage instead of refusing it: the Gradient, the Paint
  Bucket, `Edit ▸ Stroke`, Fill Path, Stroke Path, and the shape tools' Fill
  Pixels now fill the active mask, each recording its own history state and
  leaving the layer's pixels unchanged. A fill colour and a gradient ramp are
  reduced to their Rec. 601 luma, a path's or a shape's coverage is reduced to
  its luma and confined to the mask's rectangle, and the drag and click are read
  in the document's coordinates.
- `Edit ▸ Fill`'s private mask path is folded into the shared one: the fill
  engine already reduces a fill to the mask's luma when it runs on the mask's
  single-channel document, so every fill on a mask goes through one code path.
- `public_docs/text-and-typography.md` no longer says the Gradient cannot paint
  a text layer's mask.

## Impact

- Capability: `compositing/layer-masks`.
- `crates/pictura-app/src/cxxqt_object/paint_tools/fills.rs`,
  `cxxqt_object/{shapes.rs,path_list.rs}`.
- `tst_mask_edit`.
- `public_docs/text-and-typography.md`.
