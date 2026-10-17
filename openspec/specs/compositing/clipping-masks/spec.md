# compositing/clipping-masks Specification

## Purpose
Composites a clipped layer against the alpha of the layer beneath it, matching
Photoshop's clipping-group behavior.

## Requirements

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

### Requirement: Clipping membership follows stack position

A node dropped between a clipping base and its clipped layers SHALL join the
clipping mask. A clipped node dropped directly beneath a clipped layer SHALL join
that run when it has a layer below, even if the layer directly below it is not a
member; at the bottom of a container it SHALL be released. A clipped node that is
not adjacent to a clipped layer — for example, below its base with no clipped
layer above it, above a non-member, or at the bottom of its container — SHALL be
released. Moving within its own run, including
to the top or the bottom of the run, SHALL keep it clipped. Release SHALL persist:
reordering released nodes back to their former positions SHALL NOT re-clip them;
they re-join only by being dropped inside a run again.

#### Scenario: A clipped layer dragged below its base is released

- **WHEN** a clipped layer above its base is dragged below that base
- **THEN** the layer is released, the Layers panel no longer shows the clipping
  arrow or indent, and the layer composites unclipped

#### Scenario: A clipped layer dragged above a non-member is released

- **WHEN** a clipped layer is dragged above a layer that does not belong to its
  clipping mask, leaving the clipped run
- **THEN** that layer is released while the layers still in the run keep their
  clipping state

#### Scenario: A clipped layer moved into another run joins it

- **WHEN** a clipped layer from one run is dropped directly beneath a clipped
  layer of another run
- **THEN** it joins that run and stays clipped

#### Scenario: A layer dropped inside the run joins

- **WHEN** an unclipped layer is dragged between a clipping base and a clipped
  layer above it
- **THEN** that layer becomes clipped to the same base and shows the clipping
  arrow

#### Scenario: A clipped layer moved to the top of its run stays clipped

- **WHEN** a clipped layer is moved above the other clipped layers but remains in
  the same run
- **THEN** it keeps its clipping state

#### Scenario: A clipped layer moved to the bottom of its run stays clipped

- **WHEN** a clipped layer is moved directly above its base, below the other
  clipped layers
- **THEN** it keeps its clipping state

#### Scenario: Released layers do not rejoin when reordered back

- **WHEN** a base is dragged out of its run, releasing its clipped layers, and
  those layers are then reordered back above the base
- **THEN** they remain released until one is explicitly dropped inside a run

### Requirement: Reorder and removal edits never leave a baseless clip

After a move, reparent, delete, Reverse, ungroup, Background conversion, or an
insertion at the bottom of a container, the bottom layer of every affected
container SHALL NOT be clipped. When a clipping base leaves its container — by
moving, deletion, conversion to Background, or ungrouping a group that is the base
— every layer that was clipped to it SHALL be released and MUST NOT silently
re-base to another layer. Deleting a clipped layer that is not a base SHALL leave
the rest of the run clipped. Reverse moves flags with their layers and MAY leave a
reversed run re-based to a non-clipped sibling; it MUST still clear a clipped
container bottom. These postconditions constrain the Arrange, Reverse, Delete, and
Ungroup commands defined by `compositing/layer-management`.

#### Scenario: Deleting the base releases its run

- **WHEN** a base with one or more clipped layers above it is deleted
- **THEN** every formerly clipped layer is released and does not clip to a lower
  layer

#### Scenario: Moving the base out releases its run

- **WHEN** a base is moved away from directly below its clipped layers
- **THEN** every layer that was clipped to it is released

#### Scenario: Converting the base to Background releases its run

- **WHEN** a clipping base is converted to the Background and moves to the bottom
  of the document
- **THEN** its former clipped layers are released and the Background itself is not
  clipped

#### Scenario: Deleting a clipped layer keeps the run

- **WHEN** a clipped layer in the middle of a run is deleted
- **THEN** the layers above it remain clipped to the same base

#### Scenario: Ungrouping a base group releases its run

- **WHEN** a group that is the clipping base is ungrouped and its children are
  spliced into the parent
- **THEN** the run that was clipped to the group is released

#### Scenario: Reverse never leaves a clipped bottom layer

- **WHEN** Reverse places a clipped layer at the bottom of a container
- **THEN** that layer is released and the container's bottom layer is not clipped

#### Scenario: Ungrouping never leaves a clipped bottom layer

- **WHEN** ungrouping splices a clipped layer at the bottom of a container
- **THEN** that layer is released and the container's bottom layer is not clipped

#### Scenario: A new clipped layer at a container bottom is released

- **WHEN** a layer requested with clipping enabled is inserted at the bottom of a
  container
- **THEN** its clipping flag is false
