# Spec Delta

## MODIFIED Requirements

### Requirement: A structural edit refreshes only its affected influence

A structural edit SHALL refresh only the union of the document rectangles that
the affected layers can influence before and after the edit, clipped to the
canvas, through the regional composition path. The influence of an unclipped
layer SHALL be the union of its content box, its effect reach, its raster-mask
extent, its vector-mask box, and the contiguous run of clipped layers directly
above it. The influence of a clipped layer SHALL be the influence of its base —
the nearest unclipped sibling below it — because a clipped member shows only
through that base's rendered alpha. When the base is the Background or no
unclipped sibling exists, the clipped layer SHALL keep the union of its own
reach and the clipped run above it.

A move or clipping-flag change SHALL report the union of the before and after
reaches of every layer it changes. Releasing a clipping mask SHALL include every
freed member's reach, including a freed member above the clicked one, so a run
that is released or re-based is never under-bounded.

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

#### Scenario: A clipped member is bounded by its base [dd_structural_clip_member]

- **WHEN** a full-canvas clipped layer is reordered within its clipping run
- **THEN** the damage is the base's reach, not the clipped layer's own full-canvas reach, and the result equals a full recomposite

#### Scenario: A clipped member that leaves the run includes its own reach [dd_structural_clip_out]

- **WHEN** a clipped layer is moved out of its run and becomes unclipped
- **THEN** the damage includes its own before reach and its unclipped after reach

#### Scenario: Moving a base includes the released run [dd_structural_clip_base]

- **WHEN** a non-clipped base with a clipped run above it is moved
- **THEN** the before damage includes the run's members, because the move releases the run

#### Scenario: Releasing a clipping mask includes the freed run [dd_structural_clip_release]

- **WHEN** Release Clipping Mask is applied to a clipped member that has clipped layers above it
- **THEN** the damage includes every freed member's after reach, so a freed full-canvas member is refreshed

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

## ADDED Requirements

### Requirement: A move that draws nothing does not recomposite

A move of a layer or group that draws nothing SHALL NOT run the compositor and
SHALL NOT mark display damage when the move is content-neutral. A node draws
nothing when it is hidden, or inert: an empty group, or a pixel layer whose alpha
is all zero and that carries no mask, effect, adjustment, smart object, type
content, raw channels, blend-if, or knockout.

A move is content-neutral only when all of the following hold:

- the moved node draws nothing before and after the move;
- the moved node is not a clipping base on either side, where a base is a
  non-clipped layer with a clipped sibling directly above it;
- neither the before nor the after document contains a knockout layer, because
  moving the bottom layer or a group-entry backdrop can reassign a knockout base.

A content-neutral move SHALL still clear the positional links, update the
selection, and record one history state. A content-neutral move made while a live
gesture or an uncommitted preview is pending SHALL keep the full recomposite,
because the display state belongs to that gesture.

#### Scenario: A hidden move is neutral [dd_neutral_move_hidden]

- **WHEN** a hidden layer is moved into or out of a clipping run
- **THEN** the compositor does not run and the stored composite stays byte-identical

#### Scenario: An inert move inside a run is neutral [dd_neutral_move_inert]

- **WHEN** an empty layer or empty group is moved into or out of a clipping run
- **THEN** the compositor does not run and the stored composite stays byte-identical

#### Scenario: Moving a clipping base is not neutral [dd_neutral_move_base]

- **WHEN** an inert or hidden layer is a clipping base before or after the move
- **THEN** the move is not content-neutral and the full recomposite runs

#### Scenario: A knockout document never has a neutral move [dd_neutral_move_knockout]

- **WHEN** an inert bottom layer is moved while the document contains a knockout layer
- **THEN** the move is not content-neutral, because it reassigns the knockout base

#### Scenario: Neutral moves preserve the composite [dd_neutral_move_property]

- **WHEN** every move the predicate calls neutral is applied to a set of small stacks
- **THEN** each result's composite is byte-identical to the before composite

#### Scenario: A pending preview keeps the full recomposite [dd_neutral_move_pending]

- **WHEN** a content-neutral insertion or move commits while an uncommitted opacity, fill, or mask preview is pending
- **THEN** the whole document is recomposited
