## ADDED Requirements

### Requirement: The lasso fill visits only the polygon's rows

A lasso selection SHALL be filled row by row within the polygon's bounding box,
each row from its own edge crossings, and MUST select exactly the pixels whose
centres the even-odd per-pixel test puts inside. The fill SHALL report the tight
bounds of the pixels it selected.

#### Scenario: The scanline fill matches the per-pixel test [slr_polygon_scanline]

- **WHEN** fixed-seed random polygons, self-intersecting polygons, polygons with
  repeated, collinear and horizontal edges, polygons across the canvas edges and
  across a 64-row band, and polygons with no area are filled
- **THEN** every mask equals the per-pixel even-odd test's, and the reported
  bounds equal the mask's scanned bounds

### Requirement: A lasso commit works inside the shape's box

A lasso or marquee commit SHALL feather only the shape's box grown by the
kernel's reach and SHALL combine Add, Subtract and Intersect only where the
shape can cover. The result MUST equal the whole-mask feather and combine. When
a lasso commit's bounds follow from the lasso's (a New or an Add of an
unfeathered lasso), the commit SHALL record them so the outline does not scan
the mask.

#### Scenario: The bounded feather and combine equal the whole-mask ones [slr_bounded_ops]

- **WHEN** polygons inside the canvas and against its edges are feathered at
  radii on both the direct and the box-stack kernels, and combined into a base
  mask in each mode
- **THEN** every result equals the whole-mask feather or combine, and no
  feathered coverage leaves the reported window

#### Scenario: A lasso seeds the selection bounds [slr_lasso_seeds_bounds]

- **WHEN** a lasso is committed New, a second is added, the first is
  subtracted, the second is intersected, a feathered lasso is committed, and a
  lasso off the canvas is committed
- **THEN** each selection equals the whole-mask combine of the same lassos; the
  New and Add bounds are read without a scan, Subtract and the feathered lasso
  are scanned once, and the off-canvas lasso reports no bounds without a scan

### Requirement: The Magnetic Lasso computes its edge field on demand

The Magnetic Lasso press SHALL find the edge field's normalising peak without
keeping a document-sized plane, and a trace SHALL compute only the tiles of the
field it reads. Every cost MUST equal the eagerly built field's.

#### Scenario: The lazy field equals the eager one [slr_magnetic_lazy]

- **WHEN** RGB and gray noise, a vertical edge, and a flat image spanning
  several tiles are read at every pixel at three contrasts
- **THEN** every cost has the eager field's exact bits

#### Scenario: A press computes no tile [slr_magnetic_press]

- **WHEN** an edge field is built and one short segment is traced
- **THEN** no tile exists after the build, and the trace computes some tiles
  but not all

### Requirement: A shape layer draws only within its mask's box

A fill or shape layer whose mask confines it SHALL composite its fill, source
its layer effects, and build its GPU fill upload and coverage only within the
mask's box. The composite MUST equal the one over the layer's whole rect.

#### Scenario: A shape composites as its whole rect did [slr_shape_box]

- **WHEN** a document with a shape layer, with and without a drop shadow, over
  a multiply layer is composited with the content box and with the whole rect
- **THEN** the composites are byte-identical and the fill is drawn

#### Scenario: A shape samples its mask only in its box [slr_shape_mask_samples]

- **WHEN** a shape layer much smaller than the canvas is composited
- **THEN** its mask is sampled at no more pixels than its box holds

### Requirement: Moving a shape layer touches only its reach

A Move of a shape layer SHALL render the moving image over the shape's paint
bounds, and its commit SHALL refresh only the shape's influence before and
after the move. A Path or Direct Selection drag on a shape layer SHALL show the
moving outline without compositing, and its release SHALL composite only the
shape's influence before and after the drag.

#### Scenario: A shape Move previews and commits its box [slr_shape_move]

- **WHEN** a shape layer is pressed with the Move tool and the move is
  committed
- **THEN** the moving image is the shape's box at its origin, the commit's
  refreshed region is within the shape's before and after reach, and the
  shape is drawn at its new place and gone from its old one

#### Scenario: A path drag composites on its release [slr_shape_path_drag]

- **WHEN** the active shape layer's outline is dragged in two steps with the
  Path Selection tool and the drag is committed
- **THEN** no region is refreshed during the steps and the fill stays put;
  the release refreshes one region within the shape's before and after reach,
  and the shape is drawn at its new place and gone from its old one

### Requirement: A Fill Pixels shape fills its own box

A shape drawn in Fill Pixels mode SHALL be rasterized into its own box and
filled only where the box reaches, writing only the plane bands the box
crosses. The result MUST equal the document-sized rasterization and fill.

#### Scenario: The coverage box holds every covered pixel [slr_coverage_box]

- **WHEN** ellipses and rectangles inside, across and outside the canvas are
  rasterized into their box and into a document-sized mask
- **THEN** the box expanded into the document equals the document-sized mask,
  and a shape off the canvas has no box

#### Scenario: A boxed fill matches the document mask [slr_boxed_fill]

- **WHEN** a soft box across two bands is filled on a layer at the origin, an
  offset layer, and a Background, each shared with a history snapshot
- **THEN** every plane equals the document-mask fill, the changed rectangle is
  the same, the band it crosses is forked, and a band past it stays shared

### Requirement: Shapes and lassos stay responsive on a large document

On a 17000×17000 document the lasso commits, the Magnetic Lasso press and close,
shape creation in each mode, a shape Move, and a shape path drag SHALL show no
whole-document stall.

#### Scenario: Shapes and lassos on a 17000×17000 document [slr_large_manual]

- **WHEN** on a real display with a new 17000×17000 document and
  PICTURA_PAINT_TIMING=1, each lasso commits a selection, a shape is drawn in
  each mode, moved with the Move tool, and its outline dragged with the Path
  Selection tool
- **THEN** no step stalls on the whole document, and the selection, the shapes
  and their positions are correct
