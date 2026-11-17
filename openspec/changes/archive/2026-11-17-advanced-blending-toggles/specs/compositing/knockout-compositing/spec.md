# Spec Delta

## MODIFIED Requirements

### Requirement: Knockout punches a layer through to the background

The CPU compositor SHALL apply the `Knockout` mode of a non-bottom layer. When
such a layer's mode is `Shallow` or `Deep`, the layer's content SHALL be
composited against a knockout base instead of the running backdrop, and the
result SHALL replace the running backdrop at pixels the layer covers, so the
layers between the knockout layer and the base do not contribute there. The
base SHALL depend on the mode. For `Deep` it SHALL be the document background —
the bottom layer's composite, or transparency when there is none — inherited
across pass-through groups and reset at an isolation boundary to the enclosing
isolated group's initial backdrop. For `Shallow` it SHALL be the initial
backdrop of the compositor applying the layer: the document background at the
document root and for a top-level layer, and the backdrop current when the
enclosing group began for a layer inside a group. At the document root
`Shallow` and `Deep` SHALL produce byte-identical output. The layer's
Transparency Shapes flag SHALL choose what the knockout covers: when `true`
(the default), the pixels the layer's content covers; when `false`, the whole
document canvas regardless of source alpha, bounded by a mask whose hide flag is
off. A layer whose mode is `None`, a layer at the bottom of the document, and a
layer in a group with no knockout SHALL have no knockout effect.

#### Scenario: Deep punches through the intermediate layer

- **WHEN** a stack of bottom red, middle green, and a top blue layer with `Knockout::Deep` composites at a covered pixel
- **THEN** the result is blue over red, and the middle green does not contribute

#### Scenario: Deep punches through a pass-through group to the background

- **WHEN** a pass-through group above a red background contains a green layer and a blue layer with `Knockout::Deep`
- **THEN** the result is blue over red, and the group's green does not contribute

#### Scenario: Deep stops at an isolated group

- **WHEN** an isolated group above a yellow layer contains a green layer and a half-fill blue layer with `Knockout::Deep`
- **THEN** the group's green is punched through to the group's initial backdrop and the yellow below the group still contributes

#### Scenario: Shallow inside a pass-through group stops at the group backdrop

- **WHEN** a red background, then an opaque yellow layer, then a pass-through group of green and a half-fill blue layer with `Knockout::Shallow` composite at a covered pixel
- **THEN** the blue is composited against the red-plus-yellow backdrop current when the group began, the group's green is punched through, and the yellow still contributes; the same stack with `Deep` instead reveals red

#### Scenario: Shallow equals Deep at the document root

- **WHEN** a top-level stack uses `Knockout::Shallow`
- **THEN** the composited output is byte-identical to the `Deep` result

#### Scenario: None is a no-op

- **WHEN** a non-bottom layer's mode is `None`
- **THEN** the composited output is byte-identical to the same stack without the field

#### Scenario: A transparent pixel does not punch through

- **WHEN** a knockout layer with transparency-shapes on covers a pixel with zero source alpha
- **THEN** the running backdrop at that pixel is unchanged

#### Scenario: Transparency Shapes off cuts the whole canvas [ko_tsly_off_canvas]

- **WHEN** the same knockout layer has transparency-shapes off
- **THEN** the knockout base replaces every canvas pixel the layer's masks allow (with no mask, the whole canvas), including pixels where the layer has no content

#### Scenario: Transparency Shapes off respects the layer mask [ko_tsly_off_masked]

- **WHEN** the same knockout layer has a half mask and transparency-shapes off
- **THEN** the masked-out half is not punched through and the layers below it remain
