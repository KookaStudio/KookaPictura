# Spec Delta

## ADDED Requirements

### Requirement: Untracked stores are shared across history states

The history SHALL share, while unchanged, every bulk byte or sample store that
is not a tracked plane: retained native sample stores, smart-object payloads and
the raw linked-section bytes, preserved byte blocks, and the selection mask. It
SHALL share the layer tree's nodes' bulk stores. The layer-tree container (the
`Arc<Vec<Layer>>` allocation) MAY be forked once per capture path (once per
materialized state) while its nodes' bulk stores stay shared, and small
container vectors such as a layer's channel or block lists MAY be copied. No
bulk byte or sample buffer may be copied by a capture that does not write it. A
write to a shared bulk store SHALL fork only that store, leaving every captured
state byte-identical.

#### Scenario: Unchanged native samples are retained once

- **WHEN** many captures are made on a 16-bit document whose native samples do not change
- **THEN** every state shares one sample allocation per store and retained sample bytes do not grow with the number of captures

#### Scenario: An unchanged selection is retained once

- **WHEN** many captures are made and the selection is not written
- **THEN** every state shares one selection allocation

#### Scenario: Unchanged payloads and byte blocks are retained once

- **WHEN** many captures are made without editing a smart-object payload, the raw linked section, or a preserved byte block
- **THEN** every state shares those allocations and their retained bytes do not grow with the number of captures

#### Scenario: A capture forks the tree container but shares its nodes

- **WHEN** many captures are made without a structural edit
- **THEN** a capture may allocate a new layer-tree container, but every node's bulk stores remain shared across the states

#### Scenario: A native edit retains only the changed plane

- **WHEN** a capture writes one channel plane of a per-channel native store
- **THEN** only that plane is copied and the other planes remain shared with the prior state

#### Scenario: Undo of a native edit is byte-exact

- **WHEN** a capture changes a native sample store, the smart-object payload, the raw linked section, or the selection, and the state is undone
- **THEN** the restored document and selection equal the captured pre-edit bytes
