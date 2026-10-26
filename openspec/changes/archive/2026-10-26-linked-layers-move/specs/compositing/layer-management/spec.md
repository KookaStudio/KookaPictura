## ADDED Requirements

### Requirement: Moving linked layers together

The system SHALL move every member of the active layer's link set together when
the Move tool drags or nudges a linked layer. The active layer and each of its
link-set members SHALL shift by the same delta in one undoable `Move Layer`
state, and the live drag preview SHALL show the members moving together. A
member that cannot be translated — a group, an adjustment layer, a
position-locked layer, or an unresolved path — SHALL be left in place. An
unlinked layer SHALL continue to move alone. The link set SHALL remain transient
session state.

#### Scenario: Dragging a linked layer moves the set

- **WHEN** two pixel layers are linked and the Move tool moves one of them by a delta
- **THEN** both layers shift by that delta and one undo restores both positions

#### Scenario: An unlinked layer moves alone

- **WHEN** an unlinked layer is moved by the Move tool
- **THEN** only that layer shifts

#### Scenario: A position-locked member is left in place

- **WHEN** a link set contains a position-locked layer and another member is moved
- **THEN** the position-locked layer does not move and the other members shift
