# Spec Delta

## MODIFIED Requirements

### Requirement: An unbounded pixel operation keeps the full recomposite

An operation whose effect cannot be bounded to a rectangle SHALL keep the
full-document recomposite and mark the whole canvas damaged. Unbounded
operations include a document size change (resize, crop, rotate, flip, canvas
size), flatten, a merge of a scope whose affected layers include an unbounded
layer, a layer reorder or other structural edit whose affected paths include an
unbounded layer, layer color-profile conversion, the GPU backend toggle, an
adjustment layer whose mask is unbounded or whose default color contributes
outside a rectangle, a group, pass-through, or knockout stack that can read
pixels outside the changed layer's rectangle, a structural edit that can change
a root-level or group knockout base, and a structural edit performed while a
mask is soloed.

A layer whose content box and effect reach can be determined is bounded for the
Layer Style preview and for an ordinary region composite: the preview SHALL
recomposite the union of the before and after paint bounds, and the region
composite SHALL composite only the requested region, rather than the whole
document.

#### Scenario: A dimension change recomposites fully

- **WHEN** a resize, crop, rotate, flip, or canvas-size change rebuilds the document
- **THEN** the whole document is recomposited and the whole canvas is marked damaged

#### Scenario: An unbounded adjustment recomposites fully

- **WHEN** an adjustment layer with an unbounded mask or a non-zero default color is applied
- **THEN** the whole document is recomposited

#### Scenario: A layer with an effect block recomposites fully

- **WHEN** an operation on a layer with an object-based effect block cannot be
  confined to a rectangle (for example its knockout or pass-through stack reads
  pixels outside the layer's box)
- **THEN** the whole document is recomposited, because a bounded region composite
  would be incomplete

#### Scenario: A bounded effect preview recomposites its reach [dd_effect_bounded]

- **WHEN** a Layer Style value changes on a layer with a bounded effect block
- **THEN** only the union of the before and after reach is recomposited

## ADDED Requirements

### Requirement: A structural edit refreshes only its affected influence

A structural edit SHALL refresh only the union of the document rectangles that
the affected layers can influence before and after the edit, clipped to the
canvas, through the regional composition path. The influence of a layer SHALL be
the union of its content box, its effect reach, its raster-mask extent, its
vector-mask box, and the contiguous run of clipped layers directly above it.

The before and after path sets are one-sided. A path that the edit creates is
omitted before and included after; a path that the edit removes is included
before and omitted after; a path that moves or is regrouped is listed under its
old path before and its new path after. A path that is listed on a side where it
should resolve but does not is an unexpected missing path and is unbounded.

The refreshed canvas MUST be byte-identical to a full recomposite of the
document, every pixel outside the union MUST keep its previous value, and the
edit MUST NOT run the full-document recomposite path except in the fallback
cases below.

A structural edit SHALL keep the full-document recomposite and pass no history
hint when:

- an affected path is unexpectedly missing or its influence cannot be bounded;
- an adjustment layer is unmasked or its mask does not confine it (disabled,
  data-less, non-zero default color, non-255 density, or a vector mask);
- a group or clipping stack's reach cannot be proven;
- the edit can change a root-level or group knockout base while a knockout
  layer exists in that stack;
- a mask is soloed;
- a live gesture or an uncommitted preview is active;
- the sum of the clipped disjoint damage rectangle areas is at least three
  quarters of the canvas area. This is the sum of the individual rectangle
  areas, not the area of their bounding box.

A bounded structural edit SHALL still clear the positional link sets and still
emit one change notification.

#### Scenario: Deleting a bounded layer composites only its influence [dd_structural_delete]

- **WHEN** a 500×500 pixel layer is deleted from a 6000×4000 document with eight layers
- **THEN** only that layer's before influence rectangle is composited, every pixel outside it keeps its previous value, and the result equals a full recomposite

#### Scenario: Reordering a bounded layer composites its influence [dd_structural_reorder]

- **WHEN** a layer whose influence is smaller than the canvas is reordered within its container
- **THEN** only the union of its before and after influence is composited and the result equals a full recomposite

#### Scenario: A clipped run above a deleted base is included [dd_structural_clip_run]

- **WHEN** a non-clipped layer with clipped layers directly above it is deleted or moved
- **THEN** the influence includes those clipped layers, because removing or moving the base releases or re-bases the run

#### Scenario: Grouping and ungrouping stay bounded [dd_structural_group]

- **WHEN** layers with bounded influence are grouped or a bounded group is ungrouped
- **THEN** only the union of the affected before/after influence is composited and the result equals a full recomposite

#### Scenario: Duplicating a layer stays bounded [dd_structural_duplicate]

- **WHEN** a bounded layer is duplicated
- **THEN** only the union of the source's before influence and the copy's after influence is composited and the result equals a full recomposite

#### Scenario: A non-neutral insertion stays bounded [dd_structural_insert]

- **WHEN** a layer with bounded content is inserted into a document with existing layers
- **THEN** only the inserted layer's after influence is composited and the result equals a full recomposite

#### Scenario: A large but bounded edit keeps the full path [dd_structural_large]

- **WHEN** the sum of a structural edit's clipped disjoint damage areas is at least three quarters of the canvas
- **THEN** the whole document is recomposited and no history hint is passed

#### Scenario: A live gesture or pending preview keeps the full path [dd_structural_live]

- **WHEN** a bounded structural edit commits while a live gesture or an uncommitted opacity, fill, or mask preview is pending
- **THEN** the whole document is recomposited and no history hint is passed

#### Scenario: A soloed mask keeps the full path [dd_structural_mask_solo]

- **WHEN** a bounded structural edit commits while a mask is soloed
- **THEN** the whole document is recomposited, so the solo source is re-resolved or cleared

#### Scenario: An unbounded adjustment keeps the full composite [dd_structural_unbounded]

- **WHEN** a structural edit affects an unmasked adjustment layer, an unexpectedly missing path, a root-level knockout base, or a group knockout base
- **THEN** the whole document is recomposited

### Requirement: A damage bound covers the actual pixel diff

For a bounded edit, the reported damage SHALL contain every document pixel whose
value differs between a full composite before the edit and a full composite
after it. The bound MAY be larger than the diff; it MUST NOT be smaller. An
operation that cannot prove this containment SHALL report no bound and use the
full recomposite.

#### Scenario: Random structural edits stay covered [dd_damage_property]

- **WHEN** a generated sequence of delete, reorder, duplicate, group, ungroup,
  visibility, effect, clipping, knockout, adjustment, and mask-with-feather
  edits runs on a small document
- **THEN** every pixel that changes between the before and after full composites lies inside the reported damage, or the edit reported no bound

#### Scenario: A feathered adjustment mask is not under-bounded [dd_damage_feather]

- **WHEN** a masked adjustment with a feather radius has a bounded reach
- **THEN** the damage includes the mask extent grown by the feather reach

#### Scenario: Every newly bounded operation equals a full recomposite [dd_damage_ops]

- **WHEN** a bounded merge, type, shape, rasterize, smart-object, warp,
  batch-changed, smart-filter, or non-neutral insertion edit commits
- **THEN** the stored composite equals a full recomposite and every pixel outside the damage keeps its previous value
