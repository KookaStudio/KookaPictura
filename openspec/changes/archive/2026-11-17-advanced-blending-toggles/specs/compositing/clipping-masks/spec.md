# Spec Delta

## MODIFIED Requirements

### Requirement: Clipping groups composite

A clipped layer with a non-clipped sibling below it (its base) SHALL show only
where the base has content. When the base's Blend Clipped Layers as Group flag
is `true` (the default), the base with its clipped layers SHALL composite as one
group with the base's opacity and blend mode. When the flag is `false`, the base
SHALL composite with its own opacity and blend mode and each clipped member SHALL
composite with its own opacity and blend mode, still bounded by the base's
content coverage; the base's opacity and mode SHALL NOT be inherited by the
members. The base's Fill opacity and effects SHALL NOT narrow the clip; a hidden
base SHALL hide its clipped layers; a clipped layer with no base SHALL composite
unclipped.

#### Scenario: A clipping group renders as CS6 and psd-tools do

- **WHEN** the `clipping` unit tests composite a full-canvas green layer clipped to a red square, with the base at 50 % opacity, at Fill 0, hidden, with a second clipped layer, and with no base, and the `clipping_oracle` test renders a two-layer clipping group with psd-tools
- **THEN** green appears only over the square, the 50 % base fades the whole group to half strength, Fill 0 still clips, a hidden base hides the group, both clipped layers share the base, an unbased clipped layer is unclipped, a region composite matches the full one, and psd-tools agrees within 2 levels

#### Scenario: Blend Clipped off keeps the member's own blending [clip_clbl_off_mode]

- **WHEN** a clipping base has Blend Clipped Layers as Group off and a clipped member carries its own Multiply mode at 40 % opacity
- **THEN** the member composites with Multiply at 40 % and the base's own opacity does not fade it

#### Scenario: Blend Clipped off still bounds the member [clip_clbl_off_bounded]

- **WHEN** a clipped member extends past the base's content with Blend Clipped Layers as Group off
- **THEN** the member appears only where the base has content
