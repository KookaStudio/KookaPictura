# Spec Delta — document/free-transform

## ADDED Requirements

### Requirement: A layer click resolves a pending transform

While a modal Free Transform, `Edit > Transform`, or Warp session is pending,
the Layers panel SHALL stay interactive for choosing the active layer. A change
of the active layer — a row click, a keyboard row move, a right-click that moves
the selection, or a tree edit gesture — SHALL first resolve the pending
transform with the same `Apply` / `Cancel` / `Don't Apply` prompt as a tool
switch, before the selection moves. `Apply` SHALL commit the transform (one
`"Free Transform"` state) and then move the selection; `Don't Apply` SHALL
cancel it (no state) and then move the selection; `Cancel` SHALL keep the
transform pending and leave the selection unchanged. A plain click that does not
change the active layer SHALL ask nothing. A `Cancel` answer to a layer gesture
SHALL leave the document bit-identical.

#### Scenario: Clicking another layer applies the pending transform [layer_click_apply]

- **WHEN** a moved Free Transform is pending, another layer is clicked in the Layers panel, and `Apply` is answered
- **THEN** one `"Free Transform"` state is recorded, no session remains, and the clicked layer is the active layer

#### Scenario: Clicking another layer and refusing applies nothing [layer_click_dont_apply]

- **WHEN** a moved Free Transform is pending, another layer is clicked in the Layers panel, and `Don't Apply` is answered
- **THEN** no state is recorded, no session remains, and the clicked layer is the active layer

#### Scenario: Cancelling a layer click keeps the transform [layer_click_cancel]

- **WHEN** a transform is pending, another layer is clicked in the Layers panel, and `Cancel` is answered
- **THEN** the session is still pending, the previous layer is still active, and the panel's layer-editing controls stay disabled

## MODIFIED Requirements

### Requirement: A pending transform locks layer edits

While a transform is pending (a modal Free Transform, `Edit > Transform`, or
Warp session that has not been committed or cancelled), every command under the
`Layer`, `Image`, and `Filter` menus and the `Edit` pixel commands (`Cut`, the
Paste family, `Clear`, `Fill`, and `Stroke`) SHALL be disabled and SHALL refuse
dispatch. The Layers panel SHALL stay interactive for choosing the active layer
(see "A layer click resolves a pending transform"); its layer-editing controls
(the blend mode, opacity, fill, the lock toggles, and the action strip) SHALL be
disabled until the transform is resolved. While a **lifted** transform is
pending, the direct document-edit entry points SHALL also refuse, so an edit
made under the lift cannot be discarded when the transform commits or cancels. A
refused command SHALL leave the document bit-identical, add no history state,
and leave the transform pending. The transform's own commit and cancel, history
navigation (which cancels the transform first), view navigation, and
non-mutating commands SHALL remain available. Both the disabled controls and the
refusal SHALL return to their own state as soon as the session commits or
cancels. A non-lifted transform SHALL accept document edits made through the
direct API; its commit transforms the edited layer. The Move tool's `Show
Transform Controls` box, which commits each gesture on release, SHALL NOT lock.

#### Scenario: The Layer menu is disabled during a transform [lock_layer_menu]

- **WHEN** a Free Transform session is pending
- **THEN** `Layer > New > Layer`, `Layer > Delete > Layer`, `Layer > Hide Layers`, and `Layer > Arrange > Send Backward` are disabled, dispatching New Layer adds no layer, and after the commit they are enabled again

#### Scenario: The Layers panel is greyed during a transform [lock_layers_panel]

- **WHEN** a Free Transform session is pending
- **THEN** the Layers panel's editing controls — its blend mode, opacity, fill, lock toggles, and action strip — are disabled, and they are enabled again once the session is cancelled

#### Scenario: Document-editing commands are refused during a transform [lock_document_edits]

- **WHEN** a lifted Free Transform is pending and `Edit > Fill`, `Edit > Stroke`, or a filter commit is dispatched
- **THEN** the command is refused, the document and history are unchanged, and after the transform commits or cancels the command succeeds with the transform's one state recorded

#### Scenario: The Move tool's controls box does not lock [lock_controls_box]

- **WHEN** the Move tool's `Show Transform Controls` box has an active gesture session
- **THEN** `Edit > Fill` and a filter command remain enabled and the controls session stays live
