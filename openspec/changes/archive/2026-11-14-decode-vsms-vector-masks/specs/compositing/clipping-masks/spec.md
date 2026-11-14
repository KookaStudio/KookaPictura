# Spec Delta

## ADDED Requirements

### Requirement: A vector-mask base clips its run to the path

When a clipping base's contribution is confined by an enabled vector mask —
whether the mask is derived from `'vmsk'` or `'vsms'` — the clipped run SHALL be
confined to the base's rendered path coverage, not to the base layer's
rectangle. The base's own render and the clipping coverage SHALL both use the
same decoded path, so a shape base that renders by its outline also clips by
that outline. A `disabled` vector mask SHALL leave the base's rendered alpha
without the vector mask as the coverage; that alpha is the base's rectangle only
when the base is a fill or shape layer.

#### Scenario: A clipped layer follows a `vsms` shape base [clip_vsms_base]

- **WHEN** a full-canvas layer is clipped to a shape base whose `'vscg'` fill is
  confined by a closed-rectangle `'vsms'` mask
- **THEN** the clipped colour appears only inside the rectangle and the pixels
  outside it show the backdrop, not the clipped layer

#### Scenario: A clipped layer follows a `vmsk` shape base [clip_vmsk_base]

- **WHEN** the same clipping group's base carries the equivalent `'vmsk'` mask
  instead of `'vsms'`
- **THEN** the composite equals the `'vsms'` case

#### Scenario: A disabled vector mask leaves the base's rendered alpha [clip_vsms_disabled]

- **WHEN** the shape base's `'vsms'` mask has the disable flag set
- **THEN** the clipped layer is confined to the base's rendered alpha without the
  mask, which is the fill base's rectangle
