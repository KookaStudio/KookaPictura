## MODIFIED Requirements

### Requirement: Free Transform session lifecycle and undo contract

The system SHALL provide a modal, per-`PictureView` Free Transform session. It
SHALL begin on an explicit target path, hold the target paths, the union of
their rects, and the current `scale_x`, `scale_y`, `angle_radians`, and
`(dx, dy)`, and SHALL expose press, move, and release callbacks that take
document-space image coordinates and modifier flags. Enter/Return SHALL commit:
the session SHALL apply the transform to every target path, recomposite, and
record exactly one history state labelled `"Free Transform"`. Escape SHALL
cancel: the session SHALL end with the document bit-identical to its state
before the session began and with no history state added. Beginning a session
on the path already active SHALL be a no-op; beginning one on a different path
SHALL cancel the active session and start a new session on the new path.
Switching tools, switching documents, or closing the document SHALL cancel the
active session. Committing an unchanged (identity) transform SHALL record no
history state.

#### Scenario: Begin, gesture, commit records one state

- **WHEN** a session is begun on a raster layer, a scale gesture is applied, and Enter is pressed
- **THEN** exactly one history state labelled `"Free Transform"` is added and the layer rect reflects the gesture

#### Scenario: Escape restores the document exactly

- **WHEN** a session is begun, a rotation gesture is applied, and Escape is pressed
- **THEN** the document equals its pre-session value and the history count is unchanged

#### Scenario: An identity commit records nothing

- **WHEN** a session is begun and Enter is pressed without any gesture
- **THEN** no history state is added and the document is unchanged

#### Scenario: Beginning on the active path is a no-op

- **WHEN** a session is active on a path and begin is called again on that same path
- **THEN** the active session is unchanged

#### Scenario: Beginning on a different path restarts the session

- **WHEN** a session is active on one path and begin is called on a different transformable path
- **THEN** the first session is cancelled without mutating the document and a new session on the second path begins

#### Scenario: Switching documents cancels the session

- **WHEN** a session is active and the active document changes
- **THEN** the session ends without mutating the original document

## ADDED Requirements

### Requirement: Linked layers transform as one and keep their link

A Free Transform session begun on a layer that belongs to a link set SHALL
include every transformable member of that set. The transform box SHALL be the
union of the members' content rects and the default reference point SHALL be
its centre. Every gesture and exact transform command (scale, rotate, skew,
distort, perspective, flip, and warp) SHALL apply the same map to each member,
with the reference point and translation shared across the set. The live preview SHALL draw the visible
members composited over transparency under the same map, without rebuilding the
document composite on each pointer move. A hidden member SHALL transform with
the set. A position-locked member, a group, an adjustment
layer, a Background layer, or any other member the transform engine refuses
SHALL be left in place, matching the Move tool. The commit SHALL record one
history state and SHALL NOT clear the link set; the members SHALL remain
linked afterwards. A lifted selection SHALL keep its existing single-target
behaviour.

#### Scenario: Rotating a linked layer rotates the set [ftr_linked_rotate]

- **WHEN** two pixel layers are linked, Free Transform begins on one, and a 90° rotation is committed
- **THEN** both layers rotate about the union centre, exactly one history state is recorded, and the two layers remain linked

#### Scenario: A hidden linked member transforms too [ftr_linked_hidden]

- **WHEN** one member of a linked pair is hidden and the other is rotated
- **THEN** the hidden member's content receives the same transform

#### Scenario: A position-locked member stays put [ftr_linked_locked]

- **WHEN** a linked set contains a position-locked layer and another member is transformed
- **THEN** the locked member is unchanged and the other member transforms

#### Scenario: A projective mode transforms the set [ftr_linked_projective]

- **WHEN** a Skew, Distort, or Perspective session is begun on a linked layer
- **THEN** every transformable member receives the member-local quad of the same projective map

#### Scenario: Warp transforms the set [ftr_linked_warp]

- **WHEN** a Warp session is begun on a linked layer and the mesh is edited
- **THEN** the preview and the commit apply the same mesh to every transformable member
