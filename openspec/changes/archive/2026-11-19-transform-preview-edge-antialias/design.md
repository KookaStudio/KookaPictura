# Design

## Context

The engine's `resample_plane` samples a source plane padded with transparent
samples (`PAD = 2`), so a rotated or resized solid layer gets an anti-aliased
edge instead of a hard, stair-stepped one. The live preview has its own CPU warp
in `image_view_transform_warp.cpp`; its bilinear helper clamps every tap to the
image edge, so the source alpha stays 255 at the boundary and the preview edge
stays hard.

## Goals / Non-Goals

**Goals:**

- Make the live preview's resampled edge match the committed edge.
- Keep the preview's existing view-scale sampling and horizon/bowtie rules.

**Non-Goals:**

- Changing the engine resampler or the commit.
- Changing Move-preview sampling, which does not resample content.
- Adding a new preview cache or a different preview level.

## Decisions

### D1. Treat taps outside the source rectangle as transparent

The preview's `bilinear` now takes the source rectangle in image coordinates and
returns transparent for a tap outside it (including outside the image). This is
the preview-side equivalent of the engine's transparent padding.

Alternative considered: pad the moving image with a transparent border in Rust.
Rejected — it duplicates a preview-only copy of the image and shifts its origin;
the sampler already knows the source rectangle.

### D2. Let destination edge pixels sample just outside the source rectangle

The warp's per-pixel source bounds used to reject any sample outside the source
rectangle, which cut the transformed edge at the destination bounding box. The
bounds now allow one preview pixel of margin so the kernel can blend the edge
against transparent samples; farther samples are still skipped.

## Risks / Trade-offs

- **One extra anti-aliased row/column at the edge.** That is the intended
  coverage, and the committed result has the same row.
- **A source rectangle smaller than the image.** Taps outside the rectangle are
  transparent even when the image has pixels there, preserving the transform
  box's content boundary.
