# Spec Delta

## MODIFIED Requirements

### Requirement: Arrange layers within the container

The system SHALL provide the `Layer > Arrange` commands **Bring to Front**,
**Bring Forward**, **Send Backward**, and **Send to Back**. Each SHALL move the
active layer within its own container only — never into a sibling container or
a group — where the top of the stack is the front. Bring to Front SHALL move
the layer to the top of its container, Bring Forward SHALL move it one position
towards the front, Send Backward SHALL move it one position towards the back,
and Send to Back SHALL move it to the bottom-most position that is above any
Background in the container. A move that would leave the layer in its current
position SHALL be refused and record no history state, and the command SHALL be
disabled when no move is possible. The Background layer and a fully-locked
layer MUST NOT reorder. Every applied arrange SHALL recomposite and record
exactly one undo state.

#### Scenario: Bring to Front moves the active layer to the top

- **WHEN** a layer is selected in a container with other layers above it and
  Bring to Front runs
- **THEN** the layer is the topmost node of its container, the relative order of
  the other nodes is unchanged, and the change is one undo step

#### Scenario: Send to Back moves the active layer to the bottom

- **WHEN** a layer is selected in a container without a Background and Send to
  Back runs
- **THEN** the layer is the bottom-most node of its container and the change is
  one undo step

#### Scenario: Bring Forward and Send Backward move one position

- **WHEN** Bring Forward runs on a layer with a node above it
- **THEN** the layer and that node swap positions, and Send Backward performs
  the inverse swap

#### Scenario: A boundary move is refused and disabled

- **WHEN** the active layer is already at the front and Bring Forward or Bring
  to Front is invoked
- **THEN** the command is disabled, the document is unchanged, and no history
  state is recorded

#### Scenario: The Background does not reorder

- **WHEN** the Background layer is the active layer and any Arrange command runs
- **THEN** the command is disabled and the document is unchanged

#### Scenario: Send to Back stops above the Background

- **WHEN** a container has a Background and Send to Back or Send Backward runs
  on the layer directly above it
- **THEN** Send to Back places the layer directly above the Background, Send
  Backward is refused as a boundary no-op, and the Background stays bottom-most

## ADDED Requirements

### Requirement: The Background remains bottom-most

Within every container, the node carrying the Background flag SHALL remain the
bottom-most node. A container SHALL contain at most one Background, and a
document SHALL contain at most one Background overall. No other node SHALL be
inserted at a child index at or below the Background's index. A within-container
delta move SHALL clamp at the position directly above the Background; a delta
move whose source is already below the Background SHALL be refused rather than
jump a sibling. A move-to-target and its dry run SHALL both refuse a destination
at or below the Background. A container nested inside a group SHALL follow the
same rule when it contains a Background. The rule SHALL use the Background flag
and MUST NOT depend on the node's name or index.

#### Scenario: A move below the Background is refused

- **WHEN** a normal layer is moved to the position below a Background in the
  same container
- **THEN** the move is refused, the dry run reports the same refusal, and the
  document is unchanged

#### Scenario: A delta move clamps at the Background

- **WHEN** a layer directly above a Background is moved towards the back by any
  negative delta
- **THEN** the layer stays directly above the Background and records no change

#### Scenario: A delta move from below the Background is refused

- **WHEN** a document is in the invalid state where a layer sits below a flagged
  Background and that layer is moved by a delta
- **THEN** the move is refused so no sibling is jumped, and the document is
  unchanged

#### Scenario: A second Background is refused

- **WHEN** `Background From Layer` runs on a normal layer while the document
  already contains a Background
- **THEN** the conversion is refused and the document is unchanged

#### Scenario: A nested group applies the same floor

- **WHEN** a group contains a Background and a layer above it is moved below
  that Background
- **THEN** the move is refused and the nested Background stays bottom-most
