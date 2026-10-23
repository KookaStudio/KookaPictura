# Spec Delta — document/free-transform

## MODIFIED Requirements

### Requirement: A pending transform locks layer edits

While a transform is pending (a modal Free Transform, `Edit > Transform`, or
Warp session that has not been committed or cancelled), every command under the
`Layer`, `Image`, and `Filter` menus and the `Edit` pixel commands (`Cut`, the
Paste family, `Clear`, `Fill`, and `Stroke`) SHALL be disabled and SHALL refuse
dispatch, and the Layers panel SHALL be disabled and shown greyed. While a
**lifted** transform is pending, the direct document-edit entry points SHALL
also refuse, so an edit made under the lift cannot be discarded when the
transform commits or cancels. A refused command SHALL leave the document
bit-identical, add no history state, and leave the transform pending. The
transform's own commit and cancel, history navigation (which cancels the
transform first), view navigation, and non-mutating commands SHALL remain
available. Both the disabled controls and the refusal SHALL return to their own
state as soon as the session commits or cancels. A non-lifted transform SHALL
accept document edits made through the direct API; its commit transforms the
edited layer. The Move tool's `Show Transform Controls` box, which commits each
gesture on release, SHALL NOT lock.

#### Scenario: The Layer menu is disabled during a transform [lock_layer_menu]

- **WHEN** a Free Transform session is pending
- **THEN** `Layer > New > Layer`, `Layer > Delete > Layer`, `Layer > Hide Layers`, and `Layer > Arrange > Send Backward` are disabled, dispatching New Layer adds no layer, and after the commit they are enabled again

#### Scenario: The Layers panel is greyed during a transform [lock_layers_panel]

- **WHEN** a Perspective session is pending
- **THEN** the Layers panel is disabled, and it is enabled again once the session is cancelled

#### Scenario: Document-editing commands are refused during a transform [lock_document_edits]

- **WHEN** a lifted Free Transform is pending and `Edit > Fill`, `Edit > Stroke`, or a filter commit is dispatched
- **THEN** the command is refused, the document and history are unchanged, and after the transform commits or cancels the command succeeds with the transform's one state recorded

#### Scenario: The Move tool's controls box does not lock [lock_controls_box]

- **WHEN** the Move tool's `Show Transform Controls` box has an active gesture session
- **THEN** `Edit > Fill` and a filter command remain enabled and the controls session stays live
