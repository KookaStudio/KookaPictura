# Proposal

## Why

Perspective Crop's on-canvas interaction does not match CS6: the quad is
drag-drawn as one box instead of marked corner by corner, corners do not snap,
there is no rotate gesture or rotate cursor outside the quad, the shield dims
the whole workspace instead of only the canvas, and the tool has no options bar.

## What Changes

- Stage the quad from **four separate corner clicks** (TL, TR, BR, BL), showing
  the clicked points as they are placed and closing on the fourth; corners stay
  draggable and dragging inside the closed quad moves it.
- Add a **rotate gesture outside the quad** (rotate the quad about its centre)
  with the eight-way rotate cursor, reusing the rectangular crop's band logic.
- **Snap** each placed/dragged corner to the canvas edges/corners (and the
  composite bounding box) within the same 6-px screen threshold as the crop.
- Give the tool an **options bar**: `W`, `H` (final output pixels — empty means
  auto), swap, `Resolution`, `Pixels/in`, `Front Image`, `Clear`, `Show Grid`,
  and `Cancel`/`Apply` icon buttons shown only once the quad is complete.
- Draw the perspective **grid at a 1-inch spacing** from the document
  resolution, clipped to the quad, so a larger quad shows more lines at the
  same physical size.
- Shade **only the canvas** (not the workspace) and **only once the quad is
  complete**; the perspective-crop refusal behaviour is unchanged.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `tools/perspective-crop`: the tool's staging (four clicks), pointer
  (rotate outside, corner snapping), shield scope, output sizing, grid, and
  options bar are specified.

## Impact

- `crates/pictura-app/cpp/tool_perspectivecrop.cpp` — click placement, snapping,
  rotate gesture, cursor, commit with an output size.
- `crates/pictura-app/cpp/image_view_overlays.cpp` — partial quad, inch grid,
  canvas-only shield when complete.
- `crates/pictura-app/cpp/options_bar*.cpp` / `options_bar.h` — the perspective
  options page.
- `crates/pictura-render/.../perspective_crop.rs` + the `crop_group.rs` bridge —
  an explicit output size.
- Tests: `selftest_crop_group.cpp`, a new `tst_perspective_crop.cpp`.
- No new dependencies.
