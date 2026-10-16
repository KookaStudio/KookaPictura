## ADDED Requirements

### Requirement: Clipping groups composite

A clipped layer with a non-clipped sibling below it (its base) SHALL show only
where the base has content, and the base with its clipped layers SHALL
composite as one group with the base's opacity and blend mode. The base's Fill
opacity and effects SHALL NOT narrow the clip; a hidden base SHALL hide its
clipped layers; a clipped layer with no base SHALL composite unclipped.

#### Scenario: A clipping group renders as CS6 and psd-tools do

- **WHEN** the `clipping` unit tests composite a full-canvas green layer clipped to a red square, with the base at 50 % opacity, at Fill 0, hidden, with a second clipped layer, and with no base, and the `clipping_oracle` test renders a two-layer clipping group with psd-tools
- **THEN** green appears only over the square, the 50 % base fades the whole group to half strength, Fill 0 still clips, a hidden base hides the group, both clipped layers share the base, an unbased clipped layer is unclipped, a region composite matches the full one, and psd-tools agrees within 2 levels

### Requirement: Create and Release Clipping Mask

Layer > Create Clipping Mask (Ctrl+Alt+G) SHALL clip each selected layer to the
layer below it (never the bottom of a container or the Background) as one
state; Release Clipping Mask SHALL free a selected clipped layer with the
clipped layers above it, or a selected base with every layer clipped to it, as
one state. Alt-clicking the line between two Layers rows SHALL clip the upper
layer, or release it when clipped.

#### Scenario: Menu and Alt-click

- **WHEN** the `tst_layers_panel` test creates a clipping mask from the menu on a green fill above a red shape, releases it from the base, and Alt-clicks the line between the rows twice
- **THEN** the fill clips (one "Create Clipping Mask" state, green only over the square), releasing from the base frees it, and the Alt-clicks clip and release it again
