## ADDED Requirements

### Requirement: Switching to the Move tool builds no preview

Activating the Move tool, including the tool policy an undo or a transform reset
re-applies, SHALL NOT build the Move drag preview. The first press SHALL build it.

#### Scenario: Activation and undo build nothing [mvp_no_activation_bake]

- **WHEN** the Move tool is activated on a movable layer, and then an edit is undone
- **THEN** no preview image exists after either, and the next press builds one

### Requirement: A large layer previews from the view

When the Move tool presses on one plain pixel layer (no shape, type, smart
object, effects, group, or adjustment) whose content is larger than 4 MP, and the
canvas presents a pyramid level, the press SHALL build the preview at that level
from what the canvas shows. The backdrop SHALL cover only the layer's content
clipped to the presented rect, with the layer hidden, and the moving image SHALL
cover only the layer within a quarter of the presented rect beyond each edge. At
level 0 the backdrop MUST equal the document composited with the layer hidden.
The marching ants SHALL box the layer's whole content.

#### Scenario: The preview follows the view, not the document [mvp_view_preview]

- **WHEN** a canvas-sized layer over a background is pressed while the canvas
  presents level 2 over part of the document, at level 0, and with the layer off
  screen
- **THEN** the backdrop is the background over the on-screen part at that level,
  the moving image covers the layer within a quarter view at that level, a layer
  off screen has neither, and a Move drag commits the dragged offset

### Requirement: Other moves keep the full-resolution preview

The press SHALL keep the full-resolution preview for several layers, a layer that
is not plain pixels, content of at most 4 MP, a canvas presenting no level or an
empty rect, and, above level 0, a document that a reduced copy would not draw
faithfully.

#### Scenario: Exceptions keep the exact preview [mvp_exact_fallback]

- **WHEN** a view preview is asked for with no presented level, an empty rect, two
  layers, a 1 MP layer, or a styled layer elsewhere in the document above level 0
- **THEN** none is built, and the styled document still builds one at level 0

### Requirement: Only a scale-free document reduces faithfully

A document SHALL count as reducing faithfully only when no layer, at any depth,
has layer effects, live type, a smart-object source or smart filters, fill
content, or an undecoded vector fill.

#### Scenario: Effects and nested type disqualify [mvp_faithful]

- **WHEN** a plain document, the same document with an effects block, and one with
  live type inside a group are checked
- **THEN** only the plain document reduces faithfully

### Requirement: A drag past the margin rebuilds the moving image

While dragging a view preview, when the layer pixels on screen leave the region
the moving image covers, the moving image SHALL be rebuilt around the shifted
view at the same level. A drag that stays inside the region MUST NOT rebuild it.

#### Scenario: Leaving the region rebuilds [mvp_follow]

- **WHEN** a view preview is dragged within its margin, and then half a view further
- **THEN** the first drag rebuilds nothing and the second rebuilds an image whose
  region covers the layer now on screen

#### Scenario: A large document stays responsive [mvp_large_manual]

- **WHEN** on a real display, a 16507×16196 document with a canvas-sized layer is
  switched to the Move tool and the layer is dragged at fit zoom and at 100 %
- **THEN** the switch shows no stall, the press starts the drag without a visible
  stall, and the dragged layer shows its pixels at each zoom
