# Axis-aligned transforms keep a hard edge

## Why

The 0.4.0 transparent-padding fix anti-aliases every transformed edge. An
axis-aligned scale of a solid layer therefore gains a gray border on the new
edge even when the original edge was hard; 0.3.0 clamped the source edge and
kept it hard. The padding is only needed for rotated or skewed edges, where
clamping would stair-step.

## What Changes

- The engine resampler SHALL pad the source with transparency only when the
  map's edges are not axis-aligned; an axis-aligned scale SHALL clamp at the
  source edge and keep the layer opaque to its edge.
- The live transform preview SHALL use the same rule: clamped for an
  axis-aligned map, transparent-padded for a rotated, skewed, or projective
  map, so the preview edge matches the commit.
- A solid layer scaled vertically or horizontally SHALL no longer gain a gray
  border.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `document/free-transform`: the resampler and preview distinguish axis-aligned
  edges from rotated/projective edges.

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/transform.rs`: the plane map
  selects its source padding.
- `crates/pictura-app/cpp/image_view_transform_warp.cpp`: the preview selects
  the matching sampler.
- `crates/pictura-render/src/document_ops/layer_ops/transform_oracle.rs` and the
  transform tests follow the selected padding.
- No document-model, commit-path, or dependency change.
