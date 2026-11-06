# document/layer-content-bounds

## ADDED Requirements

### Requirement: A new pixel layer starts empty

A new pixel layer created by the New Layer command SHALL be stored with an empty
rect (zero width and height) and empty pixel planes, so it adds no pixel bytes to
the document. Reading a pixel outside the rect SHALL yield transparent. A New
Layer request that fills the layer with a mode-neutral colour SHALL instead
create a canvas-sized layer, because that fill is its content.

#### Scenario: A new layer adds no pixel planes [lcb_new_empty]

- **WHEN** a new pixel layer is added to a document
- **THEN** its rect is 0×0, every channel plane is empty, and the document's
  composite is unchanged

#### Scenario: A neutral fill still creates a canvas-sized layer [lcb_neutral]

- **WHEN** a new pixel layer is added with the neutral-fill option
- **THEN** its rect is the canvas and every pixel holds the neutral colour

### Requirement: An edit grows its target layer

Paint, fill, filter, Move and Free Transform SHALL grow the target pixel layer's
rect to cover their damage, padding the new area with transparent pixels. The
bytes added by a grow SHALL be reserved through the memory governor before they
are allocated, and a refusal SHALL leave the layer and the document unchanged. A
transparency-locked layer SHALL NOT grow. A group and an adjustment layer SHALL
NOT grow.

#### Scenario: A brush dab outside the rect grows the layer [lcb_paint_grow]

- **WHEN** a brush stroke paints on a layer whose rect is smaller than the dab
- **THEN** the layer's rect covers the painted pixels, the pixels outside the
  dab stay transparent, and the composite equals the canvas-sized result

#### Scenario: A fill outside the rect grows the layer [lcb_fill_grow]

- **WHEN** a Paint Bucket or Edit ▸ Fill lands in a selection that lies outside
  the target layer's rect
- **THEN** the layer grows to the selection's box and the filled pixels match a
  fill of a canvas-sized layer

#### Scenario: A filter spreads into grown padding [lcb_filter_grow]

- **WHEN** a blur is committed on a layer whose content reaches its rect edge
- **THEN** the layer grows by the filter's apron, the blurred edge is complete,
  and the layer is trimmed back to its content afterwards

#### Scenario: A locked or non-pixel layer does not grow [lcb_locked]

- **WHEN** a paint, fill or filter targets a transparency-locked layer, a group,
  or an adjustment layer
- **THEN** its rect is unchanged

#### Scenario: A refused grow leaves the document unchanged [lcb_refused]

- **WHEN** the governor's room is smaller than the bytes a grow needs
- **THEN** the grow is refused, the layer's rect and pixels are unchanged, and no
  history state is recorded

### Requirement: Cheap trims follow content

A Move commit and a Free Transform session SHALL trim every moved or transformed
pixel layer to the bounds of its non-transparent pixels. A layer created from a
fill SHALL be trimmed when it is created. A trim SHALL drop only fully
transparent pixels, so the composite SHALL be unchanged.

#### Scenario: A Move commit trims the layer [lcb_move_trim]

- **WHEN** a layer with transparent padding is moved and the move is committed
- **THEN** its rect hugs its non-transparent pixels and the composite equals the
  untrimmed move

#### Scenario: A trim does not change the composite [lcb_trim_composite]

- **WHEN** a layer with transparent padding is trimmed
- **THEN** every document pixel of the composite is unchanged

### Requirement: Growth preserves rendering

A layer that grew SHALL composite exactly as the canvas-sized layer it replaces.
Existing paint, fill and filter goldens SHALL be unchanged.

#### Scenario: A grown layer composites identically [lcb_goldens]

- **WHEN** the same paint, fill and filter operations run on an empty-rect layer
  and on a canvas-sized layer
- **THEN** the document composites are byte-identical

### Requirement: The memory gate estimates the grown rect

The fill memory gate's band estimate (`fork_bytes`) SHALL count the bands of the
layer's grown rect (`layer.rect ∪ the fill's box`), not only the bands of the old
rect.

#### Scenario: A fill that grows the layer is charged for the grown bands [lcb_fork_grown]

- **WHEN** a fill's box lies outside a small layer's rect
- **THEN** the estimate covers the bands of the union rect

