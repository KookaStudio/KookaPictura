# Proposal

## Why

Starting Free Transform (Ctrl+T) while the Brush tool is active leaves the
brush-size ring on the canvas until something else repaints it (#904). The ring
belongs to a paint hover; a modal transform session suspends tool input, so the
ring is stale chrome over the transform box.

## What Changes

- Entering any transform session clears the canvas brush outline before the
  first transform frame is painted.
- A zoom or brush-size change during the session no longer draws the ring; it
  returns on the next hover after the session ends.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/brush-tools`: a new requirement hides the brush outline while a
  transform session is open.

## Impact

- `crates/pictura-app/cpp/image_view.cpp` — `beginTransformPreview` clears the
  outline.
- `crates/pictura-app/cpp/tools_marquee.cpp` — `updateBrushOutline` refuses to
  draw while a session is active.
- `crates/pictura-app/cpp/tests/tst_transform_overlay.cpp` — one Qt Test case.
- No dependency, data-format, API, or history changes.

Closes #904.
