# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ds_edit_state_replacement` | Build a view with a recorded Fade, a repeatable transform, link sets and a pending remap, a solo snapshot, pending opacity/fill/mask-refine previews, path-drag damage, the four Move above-layer fields, a non-zero transform preview level, a Smart Filter edit target, and a Warp Text original; call `replace_document`. | Every edit-state field and every listed document-scoped cache is cleared; `fade::available` is false. | `pictura-app::replacement_clears_edit_state` |
| `ds_edit_state_snapshot` | Build a view with the same edit-state slice; call `DocumentSession::restore_named_snapshot` with a known document and target. | The snapshot document, selection, and Quick Mask are installed; the edit-state slice is cleared; `fade::available` is false; one history state with the snapshot label is current. | `pictura-app::snapshot_restore_clears_edit_state` |
| `fade_snapshot_restore_disables` | Record a Fade, restore a named snapshot, then query `fade::available`. | The retained record is gone and `Edit ▸ Fade` reports unavailable. | `pictura-app::snapshot_restore_clears_edit_state` |
