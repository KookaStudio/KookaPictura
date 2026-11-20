# Design

## Context

`resample_plane` pads the source with transparent samples so a rotated solid
edge anti-aliases instead of stair-stepping. For an axis-aligned scale, the
destination pixel grid is offset from the transformed source edge, so the padded
kernel turns the first/last edge pixel into a partial-alpha gray border. 0.3.0
clamped the kernel at the source edge and kept the edge fully opaque.

## Goals / Non-Goals

**Goals:**

- Restore the hard edge for vertical and horizontal layer scaling.
- Keep the anti-aliased edge for rotated, skewed, and projective transforms.
- Keep the live preview and the commit on the same rule.

**Non-Goals:**

- Changing the interpolation method or the layer model.
- Removing the padding from rotated or projective maps.
- Fixing the separate straight-alpha colour fringe tracked by #1446.

## Decisions

### D1. The plane map chooses its padding

`PlaneMap` gains `pad()`. The similarity map returns zero padding when its
rotation is a multiple of 90° (axis-aligned edges) and `PAD` otherwise; the
projective map returns zero when its forward homography has no x/y coupling.
The tiled resampler and the dense test oracle both use the map's choice, so
they stay byte-identical.

### D2. The preview uses the same axis test

`warpTransformPreview` checks the local matrix's off-diagonal terms (and the
90°-rotation form). An axis-aligned matrix uses the clamped sampler and no edge
margin; every other matrix uses the transparent-padded sampler. A crossed
(bowtie) quad keeps the clamped sampler because its triangles must stay straight.

## Risks / Trade-offs

- **A fractional axis-aligned scale snaps to a hard pixel edge.** That is the
  0.3.0 behavior the report asks for; the commit and preview agree.
- **A 90° rotation of a solid layer keeps a hard edge.** Its edges are still
  axis-aligned, so clamping does not stair-step.
- **The 16/32-bit native-store path still uses `pad: 0`.** It was already
  outside the 8-bit padding rule; unchanged.
