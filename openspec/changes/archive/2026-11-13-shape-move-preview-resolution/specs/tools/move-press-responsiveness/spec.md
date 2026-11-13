## MODIFIED Requirements

### Requirement: A large layer previews from the view

When the Move tool presses on one plain pixel layer (no shape, type, smart
object, effects, group, or adjustment), or on one shape layer whose appearance
is its solid fill, vector mask, and layer effects, and whose content is larger
than 4 MP, and the canvas presents a pyramid level, the press SHALL build the
preview at that level from what the canvas shows. The backdrop SHALL cover only
the layer's content clipped to the presented rect, with the layer hidden, and the
moving image SHALL cover only the layer within a quarter of the presented rect
beyond each edge. A shape layer's content SHALL be its reach (its outline box,
its effect reach, and its clipped run), and its moving image SHALL be a reduced
composite of the shape layer alone. Its layer-effect sizes MUST be scaled by the
preview level so a stroke keeps its document width. At level 0 the backdrop MUST
equal the document composited with the layer hidden. The marching ants SHALL box
the layer's whole content.

#### Scenario: The preview follows the view, not the document [mvp_view_preview]

- **WHEN** a canvas-sized layer over a background is pressed while the canvas
  presents level 2 over part of the document, at level 0, and with the layer off
  screen
- **THEN** the backdrop is the background over the on-screen part at that level,
  the moving image covers the layer within a quarter view at that level, a layer
  off screen has neither, and a Move drag commits the dragged offset

#### Scenario: A large shape previews from the view [mvp_shape_view]

- **WHEN** a shape layer whose reach is larger than 4 MP over a background is
  pressed while the canvas presents level 2 over part of the document
- **THEN** the backdrop is the background with the shape hidden at that level,
  the moving image is the shape over its reach at that level, the marching ants
  box the reach, and a Move drag commits the dragged offset

#### Scenario: A stroked shape's effect follows the level [mvp_shape_stroke]

- **WHEN** a shape layer with a 20 px layer-effect stroke and a reach larger than
  4 MP is pressed while the canvas presents level 2
- **THEN** the moving image is built at level 2 and its stroke is about a quarter
  of its document width

### Requirement: Other moves keep the full-resolution preview

The press SHALL keep the full-resolution preview for several layers, a layer that
is not plain pixels or a supported shape layer, content of at most 4 MP, a canvas
presenting no level or an empty rect, and, above level 0, a document whose
visible layers a reduced copy would not draw faithfully. The moving layer is
hidden for that check, so a hidden layer never disqualifies the reduction.

#### Scenario: Exceptions keep the exact preview [mvp_exact_fallback]

- **WHEN** a view preview is asked for with no presented level, an empty rect, two
  layers, a 1 MP layer, or a styled layer elsewhere in the document above level 0
- **THEN** none is built, and the styled document still builds one at level 0

### Requirement: Only a scale-free document reduces faithfully

A document SHALL count as reducing faithfully only when no visible layer, at any
depth, has layer effects, live type, a smart-object source or smart filters, fill
content, or an undecoded vector fill. A hidden layer MUST NOT disqualify a
reduction, because it draws nothing. A shape layer's moving image is reduced
separately: its solid fill and vector mask are scale-free, and its layer-effect
sizes SHALL be scaled by the preview level.

#### Scenario: Effects and nested type disqualify [mvp_faithful]

- **WHEN** a plain document, the same document with an effects block, one with
  live type inside a group, and one whose styled layer is hidden are checked
- **THEN** only the plain document and the document with the hidden styled layer
  reduce faithfully

### Requirement: A drag past the margin rebuilds the moving image

While dragging a view preview, when the layer pixels on screen leave the region
the moving image covers, the moving image SHALL be rebuilt around the shifted
view at the same level. A shape's coverage SHALL use its reach, not its
canvas-sized layer rect. A drag that stays inside the region MUST NOT rebuild it.

#### Scenario: Leaving the region rebuilds [mvp_follow]

- **WHEN** a view preview is dragged within its margin, and then half a view further
- **THEN** the first drag rebuilds nothing and the second rebuilds an image whose
  region covers the layer now on screen

#### Scenario: A shape follows its reach [mvp_shape_follow]

- **WHEN** a shape view preview is dragged within its margin, and then far enough
  to carry the shape's reach past the margin
- **THEN** the first drag rebuilds nothing and the second rebuilds an image around
  the shifted view

#### Scenario: A large document stays responsive [mvp_large_manual]

- **WHEN** on a real display, a 16507×16196 document with a canvas-sized layer is
  switched to the Move tool and the layer is dragged at fit zoom and at 100 %
- **THEN** the switch shows no stall, the press starts the drag without a visible
  stall, and the dragged layer shows its pixels at each zoom
