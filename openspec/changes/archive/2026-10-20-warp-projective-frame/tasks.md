# Tasks

## 1. Tests

- [x] 1.1 Render: projective frame kept exactly, mask wider than the layer, unfollowable mask refused, area ceiling, and a fold that really overlaps.
- [x] 1.2 App: preset points hidden from the press, grid and point drag through a perspective frame.
- [x] 1.3 Qt: release hides the dragged anchor's handles.

## 2. Implementation

- [x] 2.1 `transform_layer_warp` takes the box quad and composes its homography after the warp; mask beyond-box map; refusal when the mask cannot follow.
- [x] 2.2 `triangulate` holds the result to `integer_bbox`'s area ceiling.
- [x] 2.3 Warp mode keeps the mesh in source space behind the box's frame; presses on presets hit only the surface.
- [x] 2.4 The release handler refreshes the warp overlay.
