# Spec Delta

## MODIFIED Requirements

### Requirement: Management commands recomposite and record one state

Every management command in this capability SHALL run through the bridge, SHALL
recompute the document composite before recording, and SHALL add exactly one
undo state per invocation. The exception is a layer or group creation that is
content-neutral under `compositing/described-damage`: it SHALL skip the
recompute, because its stored composite already equals a recomposite. It SHALL
still add exactly one undo state. A command that changes no layer SHALL record
no history state and emit no document change.

#### Scenario: A no-op records no history

- **WHEN** a management command is invoked and its validation finds nothing to
  change
- **THEN** no history state is added and the document is unchanged

#### Scenario: One state per command

- **WHEN** a management command completes
- **THEN** exactly one undo state is recorded and one undo restores the prior
  tree and pixels

#### Scenario: Content-neutral creation records one state without a recomposite

- **WHEN** New Layer creates an empty layer that is content-neutral
- **THEN** exactly one undo state labelled `New Layer` is recorded, no
  composite runs, and one undo restores the prior tree
