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
opacity, or fill-opacity change (including a live preview and the commit that
finishes it) on a layer whose rectangle bounds its effect (a canvas-sized
rectangle is allowed and uses the region path), a clipboard clear or paste on
a bounded rectangle, a paint stroke commit or cancel on the stroke's changed
rectangle, and a layer move or free-transform commit on the union of the source
and destination rectangles. The refreshed canvas MUST be byte-identical to a
full recomposite of the document, every pixel outside the rectangle MUST keep
its previous value, and the operation MUST NOT run the full-document recomposite
path. A layer whose rectangle equals the canvas MAY composite that rectangle
through the region path; its cost then equals a full composite, but the region
path and its signals are still used.

#### Scenario: A filter on a bounded layer updates only its rectangle

- **WHEN** a filter is applied to a pixel layer whose rectangle is smaller than the document
- **THEN** only that rectangle is composited, every pixel outside it keeps its previous value, and the result equals a full recomposite

#### Scenario: An opacity change on a bounded layer updates only its rectangle

- **WHEN** a layer's opacity or blend mode is changed and the layer has a bounded rectangle
- **THEN** only that rectangle is composited and the result equals a full recomposite

#### Scenario: A live opacity preview updates only its rectangle [dd_preview_region]

- **WHEN** a layer's opacity or fill is previewed during a drag and the layer has
  a bounded rectangle
- **THEN** only that rectangle is composited, every pixel outside it keeps its
  previous value, and the result equals a full recomposite

#### Scenario: A canvas-sized layer uses the region path [dd_canvas_region]

- **WHEN** a full-canvas pixel layer's opacity or fill changes
- **THEN** the region path is used even though the rectangle equals the canvas,
  and the full-document recomposite path is not run

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
size), flatten, merge, layer reorder, layer color-profile conversion, the GPU
backend toggle, an adjustment layer whose mask is unbounded or whose default
color contributes outside a rectangle, a group, pass-through, or knockout stack
that can read pixels outside the changed layer's rectangle, and any layer
carrying an object-based effect block.

#### Scenario: A dimension change recomposites fully

- **WHEN** a resize, crop, rotate, flip, or canvas-size change rebuilds the document
- **THEN** the whole document is recomposited and the whole canvas is marked damaged

#### Scenario: An unbounded adjustment recomposites fully

- **WHEN** an adjustment layer with an unbounded mask or a non-zero default color is applied
- **THEN** the whole document is recomposited

#### Scenario: A layer with an effect block recomposites fully

- **WHEN** a layer carrying an object-based effect block (such as a drop shadow) is edited
- **THEN** the whole document is recomposited, because a bounded region composite would be incomplete

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
