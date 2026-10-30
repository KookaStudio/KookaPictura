# compositing/described-damage Specification

## Purpose
The operation-side contract that bounds a pixel-changing operation to the region
it can affect, so an ordinary edit recomposites only that region instead of the
whole document while staying byte-identical to a full recomposite.

## Requirements

### Requirement: A bounded pixel operation refreshes only its region

The app SHALL refresh only a bounded dirty rectangle, through the same regional
composition path used by visibility toggles and move previews, for a
pixel-changing operation whose effect it can bound: a filter or adjustment
applied to a pixel layer with a bounded destination rectangle, a blend mode,
opacity, or fill-opacity change (including a live preview that updates the
document composite and the commit that finishes it) on a layer whose rectangle
bounds its effect (a canvas-sized rectangle is allowed and uses the region
path), a clipboard clear or paste on a bounded rectangle, a paint stroke commit
or cancel on the stroke's changed rectangle, and a layer move or free-transform
commit on the union of the source and destination rectangles. The refreshed
canvas MUST be byte-identical to a full recomposite of the document, every pixel
outside the rectangle MUST keep its previous value, and the operation MUST NOT
run the full-document recomposite path. A layer whose rectangle equals the canvas
MAY composite that rectangle through the region path; its cost then equals a
full composite, but the region path and its signals are still used.

A display-only preview, which presents an approximate frame and defers the
composite to the commit, is governed by `compositing/render-session` and is not
a pixel-changing operation under this requirement; the commit that finishes it
is.

#### Scenario: A filter on a bounded layer updates only its rectangle

- **WHEN** a filter is applied to a pixel layer whose rectangle is smaller than the document
- **THEN** only that rectangle is composited, every pixel outside it keeps its previous value, and the result equals a full recomposite

#### Scenario: An opacity change on a bounded layer updates only its rectangle

- **WHEN** a layer's opacity or blend mode is changed and the layer has a bounded rectangle
- **THEN** only that rectangle is composited and the result equals a full recomposite

#### Scenario: A live opacity preview updates only its rectangle [dd_preview_region]

- **WHEN** a layer's opacity or fill is previewed during a drag, the preview updates the document composite, and the layer has a bounded rectangle
- **THEN** only that rectangle is composited, every pixel outside it keeps its previous value, and the result equals a full recomposite

#### Scenario: A canvas-sized layer uses the region path [dd_canvas_region]

- **WHEN** a full-canvas pixel layer's opacity or fill changes and the change updates the document composite
- **THEN** the region path is used even though the rectangle equals the canvas, and the full-document recomposite path is not run

#### Scenario: A display-only preview is not a region edit [dd_display_preview]

- **WHEN** a canvas-sized adjustment or opacity change is previewed display-only
- **THEN** the preview presents an approximate frame without updating the document composite, and the commit that finishes it follows this requirement or the full-recomposite requirement according to the committed change's bound

#### Scenario: A paste updates only the pasted rectangle

- **WHEN** a clipboard paste writes an image at a bounded destination rectangle
- **THEN** only that rectangle is composited and the result equals a full recomposite

#### Scenario: A paint commit updates only the stroke's rectangle

- **WHEN** a completed or cancelled paint stroke is committed
- **THEN** only the stroke's changed rectangle is composited and the result equals a full recomposite

#### Scenario: A transform commit updates the union of source and destination

- **WHEN** a layer move or free transform is committed
- **THEN** the union of the layer's source and destination rectangles is composited and the result equals a full recomposite

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

### Requirement: A selection-only edit does not recomposite

An edit that changes only the active selection and no pixel or layer content
SHALL NOT run the compositor and SHALL NOT mark display damage; it SHALL update
the selection overlays only. The selection-editing operations (select all,
deselect, reselect, inverse, grow, similar, move selection, and save or load
selection) MUST NOT recomposite the document.

#### Scenario: Select all does not recomposite

- **WHEN** the whole document is selected
- **THEN** the document composite is unchanged and the compositor is not run

#### Scenario: Inverse selection does not recomposite

- **WHEN** the selection is inverted
- **THEN** no full-document composite runs and the displayed document pixels are unchanged

### Requirement: The refresh region is derived from the operation

A bounded operation SHALL compute its dirty rectangle from its own parameters —
the target layer or mask rectangle, the pasted rectangle, the stroke's changed
rectangle, or the union of a move or transform's source and destination — and
SHALL pass that rectangle to the regional refresh. The refresh SHALL clip the
rectangle to the canvas, and an empty rectangle MUST be a no-op that marks no
damage.

#### Scenario: The rectangle is the operation's own extent

- **WHEN** a bounded operation refreshes its region
- **THEN** the rectangle it passes is derived from its target or changed extent, not from the whole canvas

#### Scenario: An empty rectangle is a no-op

- **WHEN** a bounded operation's computed rectangle does not intersect the canvas
- **THEN** no pixels are composited and no damage is marked

### Requirement: A paint commit composites its dirty tiles

A paint stroke's commit SHALL track its dirty area on the 64×64 tile grid and
composite the area as disjoint rectangles rather than the stroke's bounding
box. It SHALL fall back to the bounding box when the area collapses to one
rectangle, exceeds 64 rectangles, or the rectangles fill at least three quarters
of their bounding box. The recomposited canvas MUST be byte-identical to a full
recomposite, and pixels outside the dirty area MUST keep their previous value.

#### Scenario: A diagonal commits its tiles, not its bounding box [dd_commit_tiles]

- **WHEN** a thin diagonal stroke is committed
- **THEN** the dirty area is composited as multiple disjoint tile rectangles
  covering less than the stroke's bounding box

#### Scenario: A filled blob collapses to its union [dd_commit_collapses]

- **WHEN** a stroke fills most of its bounding box
- **THEN** the commit composites the bounding box as a single rectangle

#### Scenario: The tiled commit equals a full recomposite [dd_commit_equal]

- **WHEN** a stroke is committed through the tiled path
- **THEN** `doc.composite` equals a full recomposite of the committed document

### Requirement: A collapsed commit blits the region buffer it already composited

When a committed dirty area collapses to a single rectangle, the commit SHALL
build its one region blit image from the sRGB region buffer that its region
composite already produced, rather than a second crop of level 0. It SHALL still
emit exactly one region blit per commit, so the shell's region handler — and the
command registry it refreshes — runs once. The emitted image MUST be
byte-identical to the level-0 crop it replaces.

#### Scenario: The region image is emitted, not a second crop [dd_commit_blit_buffer]

- **WHEN** a stroke whose dirty area collapses to one rectangle is committed
- **THEN** the emitted region image is the premultiplied region buffer and no
  level-0 crop is built

#### Scenario: The region image equals the level-0 crop [dd_commit_blit_equal]

- **WHEN** the collapsed commit's emitted image is compared pixel for pixel with
  the level-0 crop of the same rectangle
- **THEN** every channel is identical

#### Scenario: One blit per commit [dd_commit_one_blit]

- **WHEN** any paint stroke is committed
- **THEN** exactly one region blit is emitted, whatever the tile count

### Requirement: A selection content move refreshes only its bounds

A committed selection content move SHALL refresh only the union of the
selection's source bounds and its destination bounds, clipped to the canvas,
through the regional composition path. The refreshed canvas MUST be
byte-identical to a full recomposite of the document, every pixel outside the
union MUST keep its previous value, and the commit MUST NOT run the
full-document composite. The commit SHALL still emit the document-changed
notification, because it records history and marks the document dirty.

#### Scenario: A content move refreshes its source and destination union [dd_content_move_region]

- **WHEN** a selection covering part of a layer is moved by `(dx, dy)` and committed
- **THEN** only the union of the selection's source and destination rectangles is composited, and every pixel outside it keeps its previous value

#### Scenario: The content move result equals a full recomposite [dd_content_move_equal]

- **WHEN** a content move has been committed through the regional path
- **THEN** `doc.composite` equals a full recomposite of the committed document

### Requirement: Content-neutral layer creation does not recomposite

Clipped layers attach to the nearest unclipped sibling below them. Inserting a
new empty pixel layer or a new empty group into a container is content-neutral
only when all of the following hold:

- the document had at least one layer before the insertion;
- the new node carries no pixels, no mode-neutral fill, no mask, and no effects;
- the sibling directly above the new node, if any, is not clipped, so no
  existing clipped run is moved to a new base;
- the new node is unclipped, or it is clipped and the sibling directly below it
  is itself clipped, so it joins an existing non-empty clipped run and no base
  gains its first clipped layer.

A content-neutral insertion SHALL NOT run the compositor and SHALL NOT mark
display damage. With CPU compositing, the stored document composite SHALL remain
byte-identical to `composite_rgba` of the new layer tree. Every other insertion
SHALL keep the full recomposite. So SHALL a content-neutral insertion made while
a live gesture (a stroke, a preview, a placement) is open, because the stored
composite is stale until that gesture ends.

This applies to New Layer and New Group from the Layers panel, from the
`Layer > New` dialog, from the `_in` path variants, and from the control server.

#### Scenario: New Layer runs no composite

- **WHEN** a new empty layer is created above an unclipped pixel layer whose
  upper sibling is unclipped
- **THEN** the compositor does not run, and the stored composite equals
  `composite_rgba` of the new document byte for byte

#### Scenario: New Group runs no composite

- **WHEN** a new empty group is created under the same conditions
- **THEN** the compositor does not run and the stored composite equals a full
  recomposite

#### Scenario: Insertion under a clipped sibling recomposites

- **WHEN** a new unclipped layer is inserted directly below a clipped layer
- **THEN** the insertion is not content-neutral and the full recomposite runs

#### Scenario: A first clipped layer recomposites

- **WHEN** the New Layer dialog creates a clipped layer directly above a base
  with no clipped layers yet
- **THEN** the insertion is not content-neutral and the full recomposite runs

#### Scenario: Joining an existing clipped run is neutral

- **WHEN** a clipped empty layer is inserted directly above a clipped layer
- **THEN** the insertion is content-neutral

#### Scenario: The first layer of an empty document recomposites

- **WHEN** a new layer is created in a document with no layers
- **THEN** the full recomposite runs

#### Scenario: Mode-neutral fill recomposites

- **WHEN** the New Layer dialog creates an Overlay layer with Fill With
  Mode-Neutral Color on
- **THEN** the insertion is not content-neutral

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

### Requirement: A paint refresh covers the effect and feather reach

A paint stroke's present, commit, and cancel rectangle SHALL be its changed
pixels grown by the painted layer's paint-change reach (outer and inner effects)
and, for a mask stroke, the mask feather reach. This refines "the stroke's
changed rectangle" in the bounded pixel-operation requirement: the halo and
feather ramp the stroke changes are part of its changed rectangle. The PRESENT
rectangle MUST stay incremental: it MUST NOT grow with earlier dabs of the same
stroke. The COMMIT and CANCEL refresh the pending present for a stroke that
presented as it went, or the union of the stroke's dabs for a previewed or
cancelled stroke; the history hint SHALL cover the union. All three rectangles
MUST be clipped to the canvas. A
reduced-resolution preview present is exempt: it carries the level-space extent,
and the exact commit repairs the composite. The stored composite after a commit
or cancel MUST equal a full recomposite of the document, and every pixel outside
the grown rectangle MUST keep its previous value.

#### Scenario: A styled-layer dab refreshes the halo

- **WHEN** a dab is painted on a layer carrying a drop shadow, outer glow, inner
  shadow, inner glow, stroke, satin, or bevel and the dab is committed
- **THEN** the refreshed rectangle includes the effect's reach, the halo moves
  with the dab, and `doc.composite` equals a full recomposite

#### Scenario: A mask dab refreshes the feather ramp

- **WHEN** a dab is painted on a feathered raster mask and the dab is committed
- **THEN** the refreshed rectangle includes the feather reach, the ramp outside
  the dab is refreshed, and `doc.composite` equals a full recomposite

#### Scenario: A cancelled styled stroke restores the halo

- **WHEN** a stroke on a styled layer or a feathered mask is cancelled
- **THEN** the cancel refreshes the grown rectangle, so the pre-stroke halo and
  ramp are restored without a full-document recomposite

#### Scenario: The grown rectangle is clipped to the canvas

- **WHEN** a dab near the canvas edge is committed
- **THEN** the refresh is clipped to the canvas and no out-of-canvas pixel is
  touched
