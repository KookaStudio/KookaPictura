# Spec Delta

## ADDED Requirements

### Requirement: Layer opacity scales layer effects

A layer's effects SHALL be multiplied by the layer's `opacity` in addition to
the effect's own opacity. At layer opacity 0 the layer MUST contribute no
content and no effects. Effects below the content and effects above the content
SHALL obey the same rule.

#### Scenario: Opacity 0 hides content and styles [le_layer_opacity_styles]

- **WHEN** a layer with a Drop Shadow and a stroke is composited at opacity 0
- **THEN** neither the content, the shadow, nor the stroke contributes to the backdrop, and at opacity 128 each contribution is scaled by `128/255`

### Requirement: Effect geometry follows spread and real content alpha

A layer effect's reach SHALL include every dilation the renderer applies,
including the Spread dilation `round(spread / 100 × size)` for a drop shadow and
an outer glow, so a region composite whose region intersects the effect receives
the whole effect. The effect matte SHALL use the layer content's real alpha:
for a placed smart object with no raster alpha channel it MUST use the rendered
source's alpha, not the layer rectangle. When the embedded source cannot be
rendered, the existing opaque-rectangle fallback SHALL remain.

#### Scenario: Spread is inside the effect reach [le_effects_reach_spread]

- **WHEN** a layer with a Drop Shadow at Spread 100 is composited through the region compositor for a region just inside the spread but outside the blur-only reach
- **THEN** the region composite MUST equal the same slice of the full composite

#### Scenario: A placed smart object's matte uses its source alpha [le_smart_object_matte]

- **WHEN** a placed RGB PSD with transparent corners carries a Drop Shadow
- **THEN** the shadow MUST follow the source's alpha and MUST NOT cover the transparent corners as a rectangle
