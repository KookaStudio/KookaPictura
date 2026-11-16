# Proposal

Tracking issue: #458 (follow-up: the zoomed Blur Gallery preview has no
scrollbars or panning).

## Why

The Blur Gallery zoom now enlarges the preview, but a picture larger than the
workspace is clipped with no way to reach the rest of it. Photoshop's workspace
scrolls and lets the scroll wheel's middle button drag the picture.

## What Changes

- Add horizontal and vertical scrollbars to the preview, shown only when the
  zoomed picture overflows the workspace and ranging over the overflow.
- Scrolling moves the picture; a middle-button drag pans it.
- A fitted picture keeps both scrollbars hidden.

## Capabilities

### Modified Capabilities

- `imaging/blur-gallery`: the preview gains scrollbars and middle-button
  panning when zoomed past the workspace.

## Impact

- `crates/pictura-app/cpp/blur_gallery_canvas.{h,cpp}`: the scrollbars, their
  ranges and placement, and the middle-button pan.
- `crates/pictura-app/cpp/tests/tst_blur_gallery_dialog.cpp`: coverage.
