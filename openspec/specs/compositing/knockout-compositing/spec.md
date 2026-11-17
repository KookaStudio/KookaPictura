# knockout-compositing Specification

## Purpose
Knockout layers that punch through to the background, verified against psd-tools references including group cases.

## Requirements

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

### Requirement: The GPU compositor declines a knockout layer

The GPU compositor SHALL report an advanced-blending error for a stack containing
a layer, at any depth, whose knockout mode is `Shallow` or `Deep`, so the caller
composites on the CPU. A stack with no such layer SHALL continue to use the GPU
path.

#### Scenario: A knockout layer forces the CPU oracle

- **WHEN** a document contains a layer with `Knockout::Shallow` or `Knockout::Deep`
- **THEN** the GPU compositor declines with the advanced-blending error and the CPU composite is the result

#### Scenario: No knockout keeps the GPU path

- **WHEN** no layer carries a non-`None` knockout
- **THEN** the GPU compositor is attempted as before

### Requirement: Knockout is verified against a psd-tools reference

The repository SHALL carry a committed knockout PSD fixture and a pixel
reference derived from `psd-tools`' own compositor, and a render test SHALL
composite the fixture with the CPU compositor and compare the result to the
reference within a documented tolerance. The reference-comparison test SHALL
self-skip when `psd_tools` is unavailable, and SHALL independently assert that
the knockout layer punches through the intermediate layer.

#### Scenario: The fixture decodes as a knockout

- **WHEN** the committed knockout fixture is read
- **THEN** the knockout layer's typed mode is `Deep`

#### Scenario: The CPU composite matches the reference

- **WHEN** the fixture is composited by the CPU compositor
- **THEN** the result is within the documented tolerance of the psd-tools-derived reference at every pixel

#### Scenario: The intermediate layer is punched through

- **WHEN** the result is examined at a pixel the knockout layer covers
- **THEN** the intermediate layer's green does not contribute

#### Scenario: Missing oracle self-skips

- **WHEN** `psd_tools` is not importable
- **THEN** the reference-comparison test skips rather than fails

### Requirement: The pass-through-group knockout is verified against psd-tools

The repository SHALL carry a committed knockout-in-a-pass-through-group PSD
fixture and a pixel reference derived from `psd-tools`' own compositor, and a
render test SHALL composite the fixture with the CPU compositor and compare the
result to the reference within the documented tolerance, self-skipping when
`psd_tools` is unavailable. The test SHALL assert that the fixture's group is
`PassThrough` and its child is `Deep`, so a fixture that silently becomes
isolated fails rather than passing for the wrong reason.

#### Scenario: The group fixture matches the reference

- **WHEN** the pass-through-group knockout fixture is composited by the CPU compositor
- **THEN** the result is within the documented tolerance of the psd-tools-derived reference

#### Scenario: The group's intermediate layer is punched through

- **WHEN** the result is examined at a pixel the knockout layer covers
- **THEN** the group's intermediate green does not contribute

### Requirement: The isolated-group knockout is verified against psd-tools

The repository SHALL carry a committed knockout-in-an-isolated-group PSD fixture
whose document also holds a layer below the group, and a pixel reference derived
from `psd-tools`' own compositor. A render test SHALL composite the fixture with
the CPU compositor and compare the result to the reference within the documented
tolerance, self-skipping when `psd_tools` is unavailable, and SHALL assert that
the fixture's group is not Pass Through and its knockout child is `Deep`.

#### Scenario: The isolated-group fixture matches the reference

- **WHEN** the isolated-group knockout fixture is composited by the CPU compositor
- **THEN** the result is within the documented tolerance of the psd-tools-derived reference

#### Scenario: The layer below the group still contributes

- **WHEN** the result is examined at a covered pixel
- **THEN** the group's green is punched through, and the layer below the group is blended (not the document background)

### Requirement: The nested pass-through Shallow knockout is verified against psd-tools

The repository SHALL carry a committed PSD fixture with a red Background, an
opaque intervening layer, and a pass-through group containing green and a
half-fill blue layer with `Knockout::Shallow`, plus a pixel reference derived
from `psd-tools` (>= 1.19) own compositor. A render test SHALL composite the
fixture with the CPU compositor and compare the result to the reference within
the documented tolerance, self-skipping when `psd_tools` is unavailable, and
SHALL assert that the fixture's group is `PassThrough` and its knockout child is
`Shallow` so a fixture that silently changes fails rather than passing for the
wrong reason. The test SHALL independently assert the intervening layer still
contributes (Shallow did not punch through it).

#### Scenario: The nested-shallow fixture matches the reference

- **WHEN** the nested pass-through Shallow fixture is composited by the CPU compositor
- **THEN** the result is within the documented tolerance of the psd-tools-derived reference at every pixel

#### Scenario: Shallow does not punch through the intervening layer

- **WHEN** the result is examined at a pixel the knockout layer covers
- **THEN** the group's green is punched through but the intervening layer below the group still contributes

#### Scenario: The nested-shallow oracle self-skips

- **WHEN** `psd_tools` is not importable or is older than 1.19
- **THEN** the reference-comparison test skips rather than fails
