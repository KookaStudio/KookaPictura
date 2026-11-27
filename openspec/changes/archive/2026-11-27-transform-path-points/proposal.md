# Proposal

## Why

The Transform Path and Transform Points entries are inert, leaving editable path geometry outside the interactive transform workflow (#530). Point and component selection must support the multi-selection needed to transform and repeat edits across a path.

## What Changes

- Support Shift-click and marquee selection of anchors and components across the active path, with visible selection and invalidation when its target changes.
- Enable Transform Path Scale, Rotate, Skew, Distort, Perspective, Warp and Again on selected components, and Transform Points Again on selected anchors, for Work Paths, saved paths, and editable shape/vector-mask paths.
- Preserve unrelated path geometry, vector-mask flags, pixels, and transaction history. Projective and warp maps follow the existing shape-layer convention of mapping anchors and direction points; this approximates the transformed Bézier curve.
- Magnetic pen and boolean path merging remain outside this change and are tracked by #531 and #532 respectively.

## Capabilities

### New Capabilities

- `tools/path-component-transform`: Interactive transforms and repeat on selected components and anchors of an editable path.

### Modified Capabilities

- `tools/path-selection-tool`: Multi-component and multi-anchor selection for transforms.

## Impact

Path selection and overlay, path editing bridge, transform session and command routing, catalogue and golden inventory, and Qt/Rust tests are affected. No new dependencies are anticipated.
