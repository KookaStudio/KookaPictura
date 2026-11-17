# Spec Delta

## ADDED Requirements

### Requirement: Channel blending restrictions decode and write

The system SHALL parse a layer's `brst` additional-layer-info block into a
first-class channel-restriction list: big-endian `u32` restricted channel
numbers. An absent block SHALL leave the list empty. A payload whose length is
not a multiple of four SHALL be ignored, leave the list empty, and SHALL NOT be
re-emitted on save, because the codec's even-length framing would otherwise turn
a three-byte payload into a valid four-byte one; the document read MUST NOT
fail. A successfully parsed block SHALL NOT remain in `extra_blocks`. On write
the system SHALL emit `brst` only when the list is non-empty, with one
four-byte big-endian value per restricted channel. A document whose list is
empty SHALL not contain `brst`.

#### Scenario: A brst block decodes [brst-decode]
- **WHEN** a layer carries `brst` with two big-endian channel indices
- **THEN** the layer's channel restrictions are those indices and `extra_blocks` has no `brst` entry

#### Scenario: No brst means every channel is included [brst-default]
- **WHEN** a layer has no `brst` block
- **THEN** its channel restrictions are empty

#### Scenario: A malformed brst is dropped [malformed-brst-dropped]
- **WHEN** a `brst` payload is three bytes long
- **THEN** document read succeeds, the restrictions are empty, and the saved file contains no `brst`

#### Scenario: Channel restrictions round-trip [brst-round-trip]
- **WHEN** a document with restrictions 1 and 2 is written and read back
- **THEN** the written file contains `brst` with both indices and the reopened layer reads them

### Requirement: Excluded channels keep the backdrop

The CPU compositor SHALL apply a layer's channel restrictions at the layer
boundary: it SHALL snapshot the excluded channel planes over the layer's paint
reach before the layer's below-content effects and restore that snapshot after
the layer's content and above-content effects, and around a pass-through
group's children. The restored value SHALL be the entry backdrop's straight
channel at the layer's normal output alpha. This SHALL leave an excluded channel
unaffected by the layer's content, effects and group recursion, while the
layer's alpha, other channels, mask, opacity, fill, Blend If and blend mode
apply normally. An empty restriction list SHALL composite byte-identically to
before. Restrictions beyond the three working colour planes SHALL be ignored.

#### Scenario: An excluded channel is unchanged [brst-blend-excludes]
- **WHEN** an opaque red layer over a blue backdrop excludes the red channel
- **THEN** the composited red channel equals the blue backdrop's and the green and blue channels take the layer's blend

#### Scenario: The default restriction list is a no-op [brst-blend-default]
- **WHEN** no layer carries channel restrictions
- **THEN** the composite is byte-identical to the same stack before the feature

#### Scenario: Effects do not reach an excluded channel [brst-blend-effects]
- **WHEN** a channel-excluded layer carries a bevel or an overlay
- **THEN** the excluded channel equals the entry backdrop's while the effect still shows in the other channels

#### Scenario: A pass-through group respects the restriction [brst-blend-group]
- **WHEN** a pass-through group with a non-empty restriction list contains a visible child
- **THEN** the group's excluded channel equals the group-entry backdrop's and the other channels carry the child's blend

#### Scenario: A translucent backdrop keeps its straight channel [brst-blend-translucent]
- **WHEN** a channel-excluded layer composites over a semi-transparent backdrop
- **THEN** the output alpha is the normal composite alpha and the excluded channel is the entry backdrop's straight channel value

### Requirement: The GPU compositor declines a channel-excluded layer

The GPU compositor SHALL report an advanced-blending error for a stack
containing a visible layer, at any depth, with a non-empty channel-restriction
list, so the caller composites on the CPU. A stack with no such layer SHALL
continue to use the GPU path.

#### Scenario: An excluded-channel layer forces the CPU oracle [brst-gpu-declines]
- **WHEN** a document contains a layer with a non-empty channel-restriction list
- **THEN** the GPU compositor declines with `UnsupportedAdvancedBlending` and the CPU composite is the result
