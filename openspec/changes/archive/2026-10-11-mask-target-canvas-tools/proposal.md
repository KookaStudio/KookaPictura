# Canvas tools honour the mask edit target

## Why

With a layer's mask thumbnail active, painting on the canvas wrote the layer's
pixels (#314). The mask edit target was only consulted by the bridge's
`begin_paint`, but the canvas Brush, Pencil, and Eraser start their strokes
through `begin_brush` / `begin_eraser`, which ignored it. The foreground and
background colours also stayed on the image's pair while the mask was edited.

## What Changes

- `begin_brush` (Brush, Pencil) and `begin_eraser` paint the active mask's
  coverage when a mask edit target is set; the Eraser paints the background
  colour's luma.
- The other painting tools, the Magic Eraser, and the shared fill path (Paint
  Bucket, Gradient, Edit ▸ Stroke, path fill/stroke, shapes) refuse while a mask
  is the target instead of writing the layer's pixels.
- Activating the mask swaps the foreground/background to the mask's own pair
  (initially black/white); activating the image restores the image's pair. Each
  target remembers its pair.

## Impact

- Capability: `compositing/layer-masks`.
- `crates/pictura-app/src/cxxqt_object/{paint_tools.rs,paint_tools/fills.rs,impl_paint.rs,helpers.rs}`.
- `crates/pictura-app/cpp/panels/layers_panel*`, `frame_build.cpp`, `frame.h`.
- `tst_mask_edit`.
