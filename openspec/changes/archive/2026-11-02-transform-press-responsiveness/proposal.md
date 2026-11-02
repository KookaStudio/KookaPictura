# Proposal

## Why

Pressing a transform handle freezes on very large images (#1132). On the 16507×16196
world map, pressing a handle of a 190 px text layer took about 4.6 s, and the first
press took 13 s. A second stall of 1.7 s or more could follow the release. Every
press composited the whole document twice to preview one small layer.

## What Changes

- **Moving image.** A rendered layer composites only its own bounds.
- **Backdrop patch.** A layer whose appearance stays inside its bounds gets a patch
  of those bounds, drawn over the live canvas, in place of a whole-document image.
- **Move preview.** A Move drag uses the same patch, so a click no longer lowers
  the image quality.
- **Pixel reads.** Reading one composite pixel no longer materializes the composite.

## Capabilities

### New Capabilities

- `tools/transform-press-responsiveness`: layer-sized transform start and bounded
  composite pixel reads.

## Impact

- `crates/pictura-app`: `impl_transform/session.rs`, `helpers_composite.rs`, the
  `move_preview_base_x`/`y` bridge, `ImageView::beginTransformPreview` and its paint.
- A layer with effects, a clipping group's moving image, and a lifted selection
  keep the whole-document path. #1101 still tracks the Move tool's layer drag.
