# Spec Delta

## MODIFIED Requirements

### Requirement: An unbounded pixel operation keeps the full recomposite

An operation whose effect cannot be bounded to a rectangle SHALL keep the
full-document recomposite and mark the whole canvas damaged. Unbounded
operations include a document size change (resize, crop, rotate, flip, canvas
size), flatten, merge, layer reorder, layer color-profile conversion, the GPU
backend toggle, an adjustment layer whose mask is unbounded or whose default
color contributes outside a rectangle, and a group, pass-through, or knockout
stack that can read pixels outside the changed layer's rectangle.

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
