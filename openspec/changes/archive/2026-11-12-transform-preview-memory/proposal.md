# Proposal

## Why

Issue #1297: on a 17000 × 17000 document, a large object that has been moved,
resized, and rotated kills the app (kernel OOM killer, ~4.4 GB resident on a
7.6 GB host) as soon as `Edit > Transform > Skew` or `Distort` is chosen.

Opening a transform session builds its preview from full-resolution images of
the whole target: the layer as an image, and the backdrop patch under its
bounds (composite, sRGB copy, image). Nothing gates those allocations; the #1290
gate covers only the commit. Measured on a 64 Mpx layer, the begin adds 977 MiB
(≈15 bytes a layer pixel, ≈19 at the peak). A 30° rotation leaves the object's
box at ≈325 Mpx, so any later session (Ctrl+T, Skew, Distort) needs 5–6 GB. The
projective modes add a full copy of the moving image on every preview refresh,
because the CPU warp converts it to premultiplied ARGB each time.

Separately, choosing Skew, Distort, or Perspective while a Free Transform is
open throws the pending move, scale, and rotation away and starts over. Scale
and Rotate switch the live session's mode instead (`[tc_mode_switch]`), and
Photoshop CS6 keeps the pending transform for every mode in the submenu.

## What Changes

- **Reduced transform preview.** A single plain pixel layer whose box is over
  16 Mpx previews at the smallest power-of-two level that brings it under
  16 Mpx: both the moving image and the backdrop patch are built reduced, and
  the canvas draws them scaled to document size (the affine draw, the projective
  fallback, and the CPU warp). A Warp preview of such a layer is rebuilt at the
  same level.
- **Converted once.** The CPU warp converts the moving image to premultiplied
  ARGB once per image, not on every refresh.
- **Gated begin.** Opening a transform session first asks the memory governor
  for the preview's bytes. A refusal opens no session, leaves the document as it
  was, and shows the transform memory message.
- **Mode switch into the projective modes.** Choosing Skew, Distort, or
  Perspective while a session is open switches its mode, keeping the box and the
  reference point; the pending similarity becomes the quad.

Making the Warp preview's own engine resample bounded is part of #1292.

## Capabilities

- **ADDED** `document/transform-preview-memory`
- **ADDED** `tools/projective-mode-switch`

## Impact

- `pictura-app` Rust: `impl_transform/session.rs`, `session/preview.rs`,
  `session/view_preview.rs` (shared helpers), `warp/imp.rs`, `bridge/imp.rs`,
  `state.rs`, `helpers/memory_gate.rs`.
- `pictura-app` C++: `image_view.cpp`, `image_view_overlays.cpp`,
  `image_view_transform_warp.cpp`, `image_view.h`, `tool_transform.cpp`,
  `frame_menus_transform.cpp`.
