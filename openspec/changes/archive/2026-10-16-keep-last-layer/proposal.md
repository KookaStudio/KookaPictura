# Proposal

## Why

Issue #670: deleting a document's only layer emptied the Layers panel but left
the image on the canvas. A document with no layers composites from its stored
flat image, so the deleted pixels stayed visible. CS6 never lets a document
lose its last layer.

## What Changes

- `delete_paths` never removes the document's last top-level node. This covers
  every path into it: the trash, the row menu, Layer > Delete > Layer, the
  Delete key, a drop onto the trash, and Delete Hidden Layers.
- `remove_layer`, which the control server's `layer_op` delete calls, now goes
  through `delete_paths` and returns whether it deleted, so the control server
  reports a refusal. It also stops removing the Background or a fully locked
  layer, like every other delete.
- `layers_can_delete` greys out the trash, the row menu's Delete Layer(s), and
  Layer > Delete > Layer when the selection can't be deleted. It asks the
  engine's read-only `can_delete_paths`, which shares `delete_paths`' per-node
  rule, instead of copying the document on every refresh.
- Layer > Delete > Layer was registered twice; the first registration, which
  the second overrode, is removed.
- **BREAKING**: none.

## Capabilities

- Added `ui/layers-delete-guard`.
