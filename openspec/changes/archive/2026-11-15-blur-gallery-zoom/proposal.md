# Proposal

Tracking issue: #458 (follow-up: the Blur Gallery preview has no zoom control).

## Why

The Blur Gallery preview is always fitted to the workspace. Inspecting a pin,
an iris handle, or the grain at actual pixels needs a zoom control, and the
owner's Photoshop capture shows the status-bar zoom field at the bottom left.
The workspace has no such field and its wheel does nothing.

## What Changes

- Add an editable zoom field at the bottom-left of the Blur Gallery options
  bar, showing the live percentage with CS6's preset levels plus **Fit in
  View** and **Actual Pixels**.
- The mouse wheel over the preview zooms about the pointer; `Ctrl++` /
  `Ctrl+-` and the `+` / `−` buttons step the zoom.
- Zoom only places the preview; the pin fractions and blur amounts OK commits
  are unchanged.

## Capabilities

### Modified Capabilities

- `imaging/blur-gallery`: the workspace preview gains a zoom field, wheel zoom
  anchored at the pointer, and the zoom shortcuts.

## Impact

- `crates/pictura-app/cpp/blur_gallery_canvas.{h,cpp}`: the preview zoom, its
  placement, the wheel, and the resize re-fit.
- `crates/pictura-app/cpp/blur_gallery_dialog.{h,cpp}`: the bottom-left field,
  the step buttons, the shortcuts, and the label sync.
- `crates/pictura-app/cpp/tests/tst_blur_gallery_dialog.cpp`: coverage.
