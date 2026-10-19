# Spec Delta

## MODIFIED Requirements

### Requirement: Create and Release Clipping Mask

Layer > Create Clipping Mask (Ctrl+Alt+G) SHALL clip each selected layer to the
layer below it as one state; the resolved clipping base MUST NOT be the
Background, and a selected layer MUST NOT be the bottom of its container or
itself the Background. A layer that is already clipped SHALL be left unchanged.
Release Clipping Mask SHALL free a selected clipped layer
with the clipped layers above it, or a selected base with every layer clipped to
it, as one state. Alt-clicking the line between two Layers rows SHALL clip the
upper layer, or release it when clipped, under the same base rule. A new layer
requested with clipping whose only possible base is a Background SHALL be
created unclipped rather than refused.

#### Scenario: Menu and Alt-click

- **WHEN** the `tst_layers_panel` test creates a clipping mask from the menu on a green fill above a red shape, releases it from the base, and Alt-clicks the line between the rows twice
- **THEN** the fill clips (one "Create Clipping Mask" state, green only over the square), releasing from the base frees it, and the Alt-clicks clip and release it again

#### Scenario: The Background cannot be a clipping base

- **WHEN** the layer directly above a Background is selected and Create
  Clipping Mask, the Alt-click toggle, or the Properties clipping toggle runs
- **THEN** the menu command is disabled, every gesture leaves the document
  unchanged, and no history state is recorded

#### Scenario: A new clipped layer above the Background lands unclipped

- **WHEN** the New Layer dialog requests a clipping mask while the Background is
  the selected layer
- **THEN** the new layer is inserted above the Background with its clipping flag
  clear

### Requirement: Reorder and removal edits never leave a baseless clip

After a move, reparent — including grouping and ungrouping — delete, Reverse,
Background conversion, or an insertion at the bottom of a container, the bottom
layer of every affected container SHALL NOT be clipped. Grouping SHALL normalize
both the new group's children and the parent container. When a clipping base
leaves its container — by
moving, deletion, conversion to Background, or ungrouping a group that is the base
— every layer that was clipped to it SHALL be released and MUST NOT silently
re-base to another layer. Deleting a clipped layer that is not a base SHALL leave
the rest of the run clipped. Reverse moves flags with their layers and MAY leave a
reversed run re-based to a non-clipped sibling; it MUST still clear a clipped
container bottom. A clipped run MUST NOT be based on a Background: a contiguous
clipped run directly above a Background SHALL be released. These postconditions
constrain the Arrange, Reverse, Delete, and Ungroup commands defined by
`compositing/layer-management`.

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

#### Scenario: Grouping a clipped layer clears the new bottom

- **WHEN** a clipped layer is grouped into a new container where it becomes the
  bottom child
- **THEN** its clipping flag is cleared and the parent container is normalized

#### Scenario: Reverse never leaves a clipped bottom layer

- **WHEN** Reverse places a clipped layer at the bottom of a container
- **THEN** that layer is released and the container's bottom layer is not clipped

#### Scenario: Reverse never leaves a clip on the Background

- **WHEN** Reverse would place a clipped layer directly above the Background
- **THEN** that layer and the contiguous clipped run above it are released

#### Scenario: Ungrouping never leaves a clipped bottom layer

- **WHEN** ungrouping splices a clipped layer at the bottom of a container
- **THEN** that layer is released and the container's bottom layer is not clipped

#### Scenario: A new clipped layer at a container bottom is released

- **WHEN** a layer requested with clipping enabled is inserted at the bottom of a
  container
- **THEN** its clipping flag is false
