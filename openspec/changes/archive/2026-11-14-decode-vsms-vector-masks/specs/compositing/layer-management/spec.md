# Spec Delta

## ADDED Requirements

### Requirement: A `vsms` shape layer is a shape layer

A fill-content layer whose shape definition is a `'vsms'` vector mask and whose
layer has no `'vmsk'` block SHALL be recognized as a shape layer for shape
identity: its shape outline is read from the effective vector-mask block, its
path edits write that key, it receives the shape layer's forced locks and shape
row commands, and Rasterize Shape applies to it. Rasterize Shape SHALL drop every
`'vmsk'` and `'vsms'` block along with the shape's fill-content (`'vscg'`),
`'vogk'`, and stroke blocks, bake the composited appearance, and leave the
composite unchanged.

#### Scenario: A `vsms` shape is recognized and rasterized [lm_vsms_shape]

- **WHEN** a `vscg` fill layer carrying only a closed-rectangle `vsms` mask is
  inspected and then rasterized with Rasterize Shape
- **THEN** it reports as a shape layer with its outline readable from the `vsms`
  block, Rasterize Shape succeeds, no `vmsk`, `vsms`, or `vscg` block remains,
  and the composite is unchanged
