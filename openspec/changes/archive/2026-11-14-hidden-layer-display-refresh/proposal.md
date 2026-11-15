# Refresh the canvas for every node a visibility edit changes

## Why

Hiding the Background or Alt-clicking an ordinary layer's eye could leave the
hidden pixels on the canvas at fit/normal zoom. The Alt-click solo path
refreshed only the rectangle of the layers it kept visible; every node it hid
outside that rectangle (the Background included) kept its old pixels in the
display tiles. Zooming in or out selected other tiles and made the layer seem to
disappear, which is the reported flicker.

## What Changes

- **Solo computes the reach of the nodes it flips.** The visibility edit's
  refresh rectangle is the union of every node whose eye changes, not the
  requested (kept-visible) paths.
- **Every present level follows the edit.** The canvas present at a reduced
  pyramid level (fit zoom), and a lazily opened file's merged preview, refresh
  to the same picture as a full recomposite.
- **Regression coverage.** Qt tests compare the fit-level present with a full
  recomposite after hiding the Background, after Alt-click solo, and after
  restoring the solo snapshot.

## Capabilities

- **ADDED** `compositing/composite-view`

## Impact

- `pictura-app` Rust: `cxxqt_object/impl_layers.rs`.
- `pictura-app` C++ tests: `tst_present_cache.cpp`.
