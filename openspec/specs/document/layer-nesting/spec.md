# layer-nesting Specification

## Purpose

Active-layer targeting across a nested layer tree: an editing operation resolves
the active layer path to the exact leaf layer and edits it in isolation, while
ancestor groups (their masks, clipping, blend mode, opacity, and transform)
affect only the composited result.

## Requirements

### Requirement: Active layer path resolution

The system SHALL resolve an active-layer path by walking the layer tree one
segment per level (`"0/1"` is the child at index 1 of the node at index 0), not
by treating a segment as a top-level index. Every operation that edits the active
layer (paint, fill, filter, move, matting, transform) SHALL resolve the path to
the exact leaf layer before acting, and SHALL edit that leaf's own pixels in
isolation. A path that does not resolve to a layer SHALL leave the document
unchanged.

#### Scenario: Painting a nested layer [ln_nested_paint]

- **WHEN** a layer inside a group is active and the user paints
- **THEN** the stroke is written into that exact layer's pixels

#### Scenario: Querying a nested layer's properties [ln_nested_query]

- **WHEN** a nested layer's lock, kind, or visibility is queried
- **THEN** the query resolves to that nested layer, not a top-level layer with the same trailing index

### Requirement: Groups are not edit targets

A resolved path that names a group SHALL NOT be used as a paint, fill, filter, or
transform target; the operation SHALL be refused or redirected to a required
raster child, and SHALL NOT modify the group.

#### Scenario: Group active refuses paint [ln_group_not_target]

- **WHEN** a group is the active layer and the user paints
- **THEN** no pixels are modified in the group

### Requirement: Ancestors contribute at composite time

Ancestor groups' masks, clipping, blend mode, opacity, and transform SHALL NOT be
baked into a leaf edit. They SHALL be applied only when the layer tree is
collapsed for display: an isolated group SHALL be merged before blending into its
parent, and a Pass Through group SHALL let its children composite directly into
the parent's stacking context. Paint coordinates SHALL be mapped through the
ancestor transform chain at the leaf boundary, and SHALL be the document
coordinates when no ancestor is transformed.

#### Scenario: Group opacity does not weaken the edit [ln_ancestor_composite_only]

- **WHEN** a layer inside a 50%-opacity group is painted at full strength
- **THEN** the stored pixels are full strength and the group opacity applies only when compositing

#### Scenario: A group transform maps the edit address [ln_ancestor_transform]

- **WHEN** an ancestor group is transformed and its child is painted
- **THEN** the paint lands at the canvas point under the pointer, mapped into the child's coordinate space
