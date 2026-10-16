## ADDED Requirements

### Requirement: A document keeps its last layer

A delete SHALL NOT remove a document's last top-level layer or group, whichever
command issues it, including the control server's `layer_op` delete, which
SHALL report the refusal. When the selection cannot be deleted, the Layers
panel's trash, the row menu's Delete Layer(s), and Layer > Delete > Layer SHALL
be greyed out. Deciding whether a selection can be deleted MUST NOT copy the
document, and SHALL agree with what the delete would do.

#### Scenario: The delete entry points grey out on the last layer [ll_last_layer_disabled]

- **WHEN** a document is down to one layer
- **THEN** the trash, the row menu's Delete Layer(s), and Layer > Delete > Layer are greyed out, and they were enabled while a second layer existed

#### Scenario: A delete of the last layer is refused [ll_last_layer_refused]

- **WHEN** a delete of the last top-level layer or group reaches the engine (the Delete key, or every layer selected)
- **THEN** that node stays, no history state is recorded, and a nested layer inside the last group can still be deleted

#### Scenario: The control server's delete is guarded [ll_remove_layer_guarded]

- **WHEN** `remove_layer` (the control server's `layer_op` delete) targets the last layer, or an index out of range
- **THEN** it reports a refusal, the layer stays, and no history state is recorded, while it still deletes a second layer as one state

#### Scenario: Availability matches the delete [ll_can_delete_agrees]

- **WHEN** availability is asked for a selection (empty, the only layer, the Background, a locked layer, every layer, a nested child, an unknown path)
- **THEN** it is true exactly when the delete would remove something, and it reads the document without copying it
