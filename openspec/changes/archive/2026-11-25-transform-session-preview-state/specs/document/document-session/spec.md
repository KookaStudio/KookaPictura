# Spec Delta

## MODIFIED Requirements

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

The C++ tool controller SHALL NOT keep a stored copy of the transform
controls-session flag either. Whether a live transform belongs to the Move
tool's controls box SHALL have one owner, and the controller's predicate SHALL
be derived from the idle-box state it already owns plus the session flag read
over the bridge.

The preview state a transform gesture draws with SHALL have one field teardown,
`clear_preview_overlay`, that writes the same set of preview and Move fields
every time. The transform's own preview clear SHALL be a thin wrapper over that
teardown; the transform's cancel and document-replacement paths SHALL call the
same teardown directly. Every way a transform ends — an explicit cancel, a
commit whether it changed the document or was refused, a deferred commit that
lands, a smart-object commit, a failed begin, a history restore, a tool switch,
and a document replacement — SHALL clear those fields through it. A cancelled
or failed transform SHALL leave no Move preview field and no preview field set:
no cached base or moving image, no base origin, no origin, no above-layer image,
origin, scale, or presented key, no preview level, no placement or opacity, no
prepared-group cache key, no cache-hit flag, no active-drag flag, and no
mask-move baseline. A failed begin that has already taken the outgoing session
SHALL clear the preview fields that session had left set; a refusal before the
take SHALL keep the live session and its preview. The C++ controller SHALL reset
its transform overlay when a failed begin leaves no live session and SHALL
preserve the overlay when the live session remains.

The transform lifecycle SHALL clear the controls-session flag when a begin takes
an outgoing session, when a new session is installed, and at commit entry. Both
cancel paths SHALL clear it as well, so history restore, Undo or redo, a tool
switch, a document replacement, or the controller's own cancel cannot leave the
flag set. A begin refused before taking the outgoing session SHALL preserve the
live controls session and its flag. The Move-tool preview paths that never owned
the flag SHALL NOT clear it.

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

#### Scenario: Every way a transform ends clears the same fields [ds_preview_one_teardown]

- **WHEN** a transform session's preview has been built, and it ends in turn by
  an explicit cancel, by a commit that changed the document, by a commit that
  was refused for memory, by a failed begin, by a deferred commit that lands,
  and by a smart-object commit
- **THEN** after every one of those endings the same four observed fields are
  clear: no base image, no moving image, no above-layer image, and no base origin.
  No observed value differs between the paths

#### Scenario: A cancelled controls gesture leaves no stale preview [ds_preview_cancel_clears]

- **WHEN** a Move controls gesture is live with an above-layer image, a mask-move baseline, and an active drag, and its session is cancelled through the document-session teardown rather than the Move tool's own cancel
- **THEN** the above-layer image, origin, scale, and presented key are cleared, the mask-move baseline is unset, the active-drag flag is false, and a landing Move composite is not told that a drag still owns the canvas

#### Scenario: A replaced document leaves no stale preview [ds_preview_replacement]

- **WHEN** a document is replaced while the Move preview's base origin is set,
  the prepared-group key is set, and the preview level is non-zero
- **THEN** the base origin, the prepared-group key, and the preview level are
  cleared along with the rest of the preview fields

#### Scenario: The controls-session flag is derived, not stored [ds_controls_flag_derived]

- **WHEN** the idle controls box is up, then a handle gesture begins, then its
  changed release promotes the transform to pending, and then the transform is
  cancelled
- **THEN** the controller reports a controls session while idle and during the
  gesture, does not report one after the changed release while the session is
  still active, and after the cancel reports no active session, no pending
  transform, and a controls session again because the idle box is rebuilt

#### Scenario: A Move preview does not clear a controls transform [ds_controls_flag_move_path]

- **WHEN** a Move preview begins and ends during a live controls gesture, then a changed release promotes the transform to pending and another Move preview begins
- **THEN** the live controls flag stays set during the first preview, and the pending transform stays pending with its flag unchanged during the second

#### Scenario: Direct transform calls own the controls flag [ds_controls_flag_direct_lifecycle]

- **WHEN** a direct begin is refused before taking a live controls session, and that session is then committed directly
- **THEN** the refused begin preserves the live session and its controls flag, and the direct commit clears the flag
