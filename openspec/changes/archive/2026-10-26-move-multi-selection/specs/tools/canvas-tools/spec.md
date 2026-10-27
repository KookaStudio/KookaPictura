## MODIFIED Requirements

### Requirement: Move tool translates the active layer
The system SHALL translate the document's selected layers by a dragged offset,
recompositing the document. With one layer selected it SHALL translate that
layer; with several selected it SHALL translate every selected layer. Group and
adjustment layers SHALL be ignored (not translated), and an empty or absent
document SHALL be a no-op. A layer whose lock state includes the position lock
SHALL be refused, and the refusal SHALL leave that layer unchanged; a move that
translates no layer SHALL add no history state and change no pixels. A selected
layer's linked members move with it (see `compositing/layer-management`).

#### Scenario: Drag moves the layer content
- **WHEN** the Move tool is dragged by a delta and the active document has a pixel layer
- **THEN** that layer's content is translated by the delta and the composite updates

#### Scenario: No pixel layer
- **WHEN** the Move tool is dragged and the document has no pixel layer
- **THEN** nothing changes and the document is not corrupted

#### Scenario: A position-locked layer is refused [lct_move_locked]
- **WHEN** the Move tool is dragged and the topmost pixel layer is position-locked
- **THEN** the layer does not move, the composite is unchanged, and no history
  state is added

#### Scenario: Dragging a multi-selection moves every selected layer [lct_move_multi]
- **WHEN** two pixel layers are selected and the Move tool is dragged by a delta
- **THEN** both layers translate by the delta and the move is one `Move Layer`
  history state
