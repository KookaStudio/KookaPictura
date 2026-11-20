# Spec Delta

## REMOVED Requirements

### Requirement: Handle gestures apply per drag

### Requirement: Inside and outside presses are Move drags

## ADDED Requirements

### Requirement: Handle gestures become a pending transform

A press on a scale handle or in the rotate band while the controls box is shown
SHALL start the matching transform gesture. Releasing a gesture that changed the
target SHALL keep the transform pending: the idle box SHALL be replaced by the
session overlay with its reference point, the options bar SHALL show the Free
Transform page with Confirm and Reject controls, and no history state SHALL be
recorded yet. Confirm (the Apply button or Enter) SHALL commit it as exactly one
`"Free Transform"` history state and restore the idle controls box on the
resulting layer. Reject (the Cancel button or Esc) SHALL cancel it with no
history state and restore the idle controls box on the unchanged layer. A
gesture that changed nothing SHALL record no state, leave no session, and return
to the idle controls box. On a linked layer the pending transform SHALL cover
every transformable member of the set, and Confirm SHALL apply it to all of
them. The pending transform SHALL lock document edits and ask before a tool
switch or a layer change exactly as a modal Free Transform does. `Ctrl+T` SHALL
replace the idle box with the modal Free Transform session.

#### Scenario: A handle drag stays pending and then commits [lmtc_handle_drag]

- **WHEN** a corner handle is dragged while the controls box is shown and released
- **THEN** no history state is recorded, the transform is pending, the options bar shows the transform page, and Apply records exactly one `"Free Transform"` state and restores the idle box on the scaled layer

#### Scenario: A linked handle drag stays pending and then commits [lmtc_handle_drag_linked]

- **WHEN** a corner handle is dragged while the controls box spans a link set and released
- **THEN** the set stays pending, and Apply scales every transformable member about the same anchor, records one `"Free Transform"` state, and the links survive

#### Scenario: An identity drag records nothing [lmtc_identity]

- **WHEN** a handle is pressed and released without moving
- **THEN** no history state is recorded, no transform session remains, and the idle box remains

#### Scenario: Ctrl+T takes the box over [lmtc_ctrl_t]

- **WHEN** `Ctrl+T` is pressed while the controls box is shown
- **THEN** the modal Free Transform session begins on the layer and the options bar shows the transform page

#### Scenario: Reject cancels the pending transform [lmtc_handle_reject]

- **WHEN** a corner handle is dragged, released, and the transform page's Cancel button is clicked
- **THEN** no history state is recorded, the document is unchanged, and the idle box returns

#### Scenario: The bar shows the transform page while pending [lmtc_pending_bar]

- **WHEN** a handle drag that changed the layer is released
- **THEN** the options bar shows the transform page, and Apply or Cancel returns it to the Move page

#### Scenario: The pending session shows the reference point [lmtc_pending_pivot]

- **WHEN** a handle drag that changed the layer is released
- **THEN** the session overlay shows the reference point, while the idle box showed none

### Requirement: Idle-box presses are Move drags

A press inside the idle controls box away from a handle, or outside it, SHALL
suspend the box and run the ordinary Move tool drag: the layer translates, Alt
duplicates, and a cross-document drag keeps working, recording the Move tool's
own `"Move Layer"` state. The box SHALL return when the drag ends, positioned on
the moved layer.

#### Scenario: An inside drag moves the layer [lmtc_inside_move]

- **WHEN** the pointer is pressed inside the idle box and dragged
- **THEN** the layer translates by the drag delta, one `"Move Layer"` state is recorded, and the box returns on the moved layer

#### Scenario: A click with no movement leaves the document alone [lmtc_click]

- **WHEN** the pointer is pressed and released inside the box without moving
- **THEN** no history state is recorded and the box remains

### Requirement: Pending transform gestures refine the session

While a controls transform is pending, another handle gesture SHALL refine the
same session, and a press inside the box SHALL move the target within the
transform (Free Transform semantics) instead of recording a separate
`"Move Layer"` state. Confirm SHALL apply every refinement as one
`"Free Transform"` history state.

#### Scenario: A second handle drag refines the pending transform [lmtc_handle_refine]

- **WHEN** a second handle drag is made while the transform is pending
- **THEN** the session stays pending, no history state is recorded, and Apply commits both gestures as one `"Free Transform"` state

#### Scenario: An inside drag moves within the pending transform [lmtc_inside_pending]

- **WHEN** the pointer is pressed inside the pending box and dragged
- **THEN** the target moves within the same session, no history state is recorded, and Apply commits the move with the transform
