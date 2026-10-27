# Proposal: Unlinking a member clears a two-layer link set

## Why

Follow-up to `linked-layers-move` (#959). A link set is a relationship between
two or more layers, but the transient map keeps a member after its last partner
is unlinked. So with two linked layers, selecting one and choosing `Unlink
Layers` leaves the other in a one-member set: its row keeps showing the chain
badge and `Select Linked Layers` still returns it, though it is linked to
nothing.

## What Changes

- After an unlink, drop any link set left with a single member, so a severed
  relationship shows no link. Linking a single layer still creates its set (the
  existing single-layer link behaviour is unchanged).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-management`: a link set always has at least two members.

## Impact

- Rust: `cxxqt_object/impl_layers_select.rs` (`apply_link`).
- Self-test: `selftest_layers_controls.cpp` (`lpr_link_linked_unlink`).
- Tests: `tst_linked_layers` gains `unlinkClearsTheSet`.
