# Spec Delta

## ADDED Requirements

### Requirement: The live edit session is one owned value

The document session SHALL hold the live edit session in a single value it
owns, and the value's variant SHALL name the kind of that session. The session
kind SHALL be read from that value and SHALL NOT be inferred from independent
optional state.

The kinds of live session are: a paint stroke, a mask stroke, a healing
stroke, a selection move, a transform, a path gesture, a lasso or magnetic
selection, a filter preview, and a mode preview. The value's stroke variant
covers the paint stroke and the mask stroke, and the kind it reports SHALL be
decided by the stroke's target. A document SHALL NOT carry the state of two
live sessions at once.

The C++ tool controller SHALL NOT keep its own copy of the transform drag
handle or its dragging flag; it SHALL read the dragged handle from the session
over the bridge and treat a value below 0 as no live drag.

#### Scenario: The session kind is read from the owned value [ds_slot_kind]

- **WHEN** a Free Transform session is live
- **THEN** the document session reports the Transform kind, and the session
  holds no state for any other kind of session

#### Scenario: One kind of session state exists at a time [ds_slot_one_state]

- **WHEN** a filter preview is live on a layer's pixels
- **THEN** the session holds no mask filter preview, and cancelling the session
  restores the layer's pre-preview pixels

#### Scenario: The transform drag handle comes from the session [ds_slot_drag_handle]

- **WHEN** a transform drag is pressed on a corner handle and later released
- **THEN** the dragged handle reported over the bridge is the corner's index
  while the drag is live, and -1 once the drag is released
