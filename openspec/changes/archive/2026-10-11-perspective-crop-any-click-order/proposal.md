# Perspective crop from any click order

## Why

The engine maps the clicked corners to the output rectangle in click order, so
starting the four-corner quad anywhere but the content's top-left rotated or
mirrored the warped result. The corner order should not matter.

## What Changes

- The Perspective Crop tool derives the quad's top-left, top-right,
  bottom-right, bottom-left orientation from the clicked positions (clockwise
  about their centroid, starting from the top-left-most point), so the result is
  never rotated or mirrored.
- The four clicks may start at any corner.

## Impact

- Capability: `tools/perspective-crop`.
- `crates/pictura-app/cpp/tool_perspectivecrop.cpp` and `tst_perspective_crop`.
- `docs/03-tools/perspective-crop.md`.
