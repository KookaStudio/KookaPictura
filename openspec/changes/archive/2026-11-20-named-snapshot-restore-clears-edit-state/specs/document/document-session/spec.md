# Spec Delta

## ADDED Requirements

### Requirement: Document replacement and named-snapshot restore clear the edit state

The document session SHALL own the document-scoped edit-state slice: the
retained Fade record, the last repeatable transform, the layer link sets and
their pending remap, the solo visibility snapshot, and the pending panel
previews. One operation SHALL clear that slice. Both installing a replacement
document and restoring a named snapshot SHALL invoke it, so edit state captured
for the outgoing document cannot survive either path.

Document replacement SHALL additionally reset the path-drag damage, the Move
preview's above-layer image, origin, scale, and presented key, the transform
preview level, the Smart Filter edit target, and the Warp Text original.

Restoring a named snapshot SHALL still install its document and selection,
restore its Quick Mask, clear the deselected-selection memory, bump the document
generation, retain the layer target captured with the snapshot, and append
exactly one history state labelled with the snapshot's label.

#### Scenario: Replacement clears the edit-state slice and preview caches [ds_edit_state_replacement]

- **WHEN** a document with a retained Fade record, a last repeatable transform, link sets, a pending link remap, a solo visibility snapshot, pending opacity/fill/mask-refine previews, path-drag damage, Move above-layer preview fields, a non-zero transform preview level, a Smart Filter edit target, and a Warp Text original is replaced
- **THEN** every one of those fields is cleared, the edit state cannot name the outgoing document, and Edit ▸ Fade reports unavailable

#### Scenario: Snapshot restore clears the edit state and disables Fade [ds_edit_state_snapshot]

- **WHEN** a named snapshot is restored while a retained Fade record, a last repeatable transform, link sets, a pending link remap, a solo visibility snapshot, and pending panel previews exist
- **THEN** the snapshot's document, selection, and Quick Mask are installed, every field in the edit-state slice is cleared, Edit ▸ Fade reports unavailable, and exactly one history state labelled with the snapshot's label is appended
