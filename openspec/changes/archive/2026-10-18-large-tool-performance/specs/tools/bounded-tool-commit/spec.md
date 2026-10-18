## ADDED Requirements

### Requirement: A stroke commits only what it has not presented

While a layer stroke runs, each region it presents SHALL also be patched into
the document's stored composite, so the commit SHALL composite only the region
still pending (the dabs since the last present), not the stroke's whole extent,
and SHALL NOT rebuild the canvas frame. A stroke previewed at reduced
resolution SHALL still refresh its dirty tiles at commit, and a mask stroke
SHALL leave the stored composite alone until it commits. The committed
composite SHALL equal a full recomposite of the document.

#### Scenario: A large stroke's composite is exact after commit [btc_stroke]

- **WHEN** a 200 px brush paints eight dabs over a textured document, presenting
  after some of them and leaving the last ones pending, and the stroke ends
- **THEN** the canvas frame is not rebuilt, the stroke shows in the composite,
  and the composite equals a full CPU recomposite

### Requirement: Cancelling a stroke repairs the composite

Cancelling a layer stroke whose presents patched the composite SHALL composite
the stroke's extent again from the restored layer, so the stored composite
holds the pre-stroke pixels.

#### Scenario: A cancelled stroke leaves no paint [btc_cancel]

- **WHEN** a stroke is painted and presented over a textured document and then
  cancelled
- **THEN** the stored composite equals its pre-stroke pixels

### Requirement: A new shape layer composites its own bounds

Adding a shape layer SHALL composite only the region the new layer can touch:
its outline's bounds grown by its stroke's width and one pixel. The canvas
frame SHALL NOT be rebuilt. The whole document SHALL be recomposited instead
when another layer carries a layer style, or when the layer just above the new
one is clipped. The composite SHALL equal a full recomposite either way.

#### Scenario: A stroked ellipse composites its bounds [btc_shape]

- **WHEN** an ellipse shape layer with a 6 px outside stroke is added over a
  textured document
- **THEN** the frame is not rebuilt, the ellipse's centre is its fill colour, a
  pixel outside its bounds keeps its colour, and the composite equals a full
  recomposite

### Requirement: A fill composites the rectangle it changed

The fill family (Gradient, Paint Bucket, Fill, Stroke, Fill Path, Stroke Path,
and the shape tools' Pixels mode) SHALL composite only the rectangle its edit
reports as changed, then notify the panels, without rebuilding the canvas
frame. When any layer carries a layer style, which could spread the change
past that rectangle, it SHALL recomposite the whole document.

#### Scenario: A gradient in a selection composites its rect [btc_fill]

- **WHEN** a gradient is drawn inside a rectangular selection of a textured
  document
- **THEN** the frame is not rebuilt and the composite equals a full recomposite

#### Scenario: A styled document recomposites whole [btc_styled]

- **WHEN** a gradient is drawn on a document where a layer carries a Color
  Overlay
- **THEN** the frame is rebuilt from a full recomposite, and the composite
  equals a full recomposite

#### Scenario: A style elsewhere in the tree counts [btc_style_scope]

- **WHEN** the document's layers are searched for a style, exempting one path
- **THEN** a style nested in a group is found, the exempt layer's own style is
  not, and exempting a group does not exempt its children
