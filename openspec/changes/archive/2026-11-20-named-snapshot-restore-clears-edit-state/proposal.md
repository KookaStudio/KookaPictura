# Proposal

## Why

`history_restore_snapshot` is a second restore path that bypasses the edit-state
invalidation rule. It rewrites the document itself and calls `session.record`
directly instead of `PictureView::record`, which is where Fade is cleared. After
a named-snapshot restore the retained Fade record survives, `fade::available`
stays true, and applying Fade resolves the old layer path in the restored
document. If that path names a different layer there, Fade pastes the wrong
layer's pixels.

The hand-written reset in `clear_document_scoped_state` has also drifted from
the document-scoped fields it must clear: it misses the path-drag damage, the
four Move above-layer preview fields, the transform preview level, the Smart
Filter edit target, and the Warp Text original.

## What Changes

- Move the six-field edit-state slice (`fade`, `last_transform`, `link_sets`,
  `link_remap`, `solo_snapshot`, `pending_previews`) under `DocumentSession` and
  clear it through one `clear_edit_state`.
- Add `DocumentSession::restore_named_snapshot`, which owns the document,
  selection, Quick Mask, deselected selection, document generation, and the
  history record for a named-snapshot restore.
- Call `clear_edit_state` from both `replace_document` and the snapshot restore.
- Reset the missed document-scoped fields on document replacement.
- Add Rust regression tests for replacement and snapshot restore.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `document/document-session`: adds the requirement that document replacement
  and named-snapshot restore share one edit-state clear, and that replacement
  also resets the previously missed document-scoped fields.
- `imaging/filter-fade`: adds the requirement that a named-snapshot restore
  discards the retained last-effect record.

## Impact

- `crates/pictura-app/src/cxxqt_object/document_session/session.rs`: owns the
  edit-state slice and the snapshot-restore method.
- `crates/pictura-app/src/cxxqt_object/impl_history.rs`: the snapshot restore
  routes through the session.
- `crates/pictura-app/src/cxxqt_object/state.rs` and the call sites that read
  the moved fields.
- No display-frame (#783) or layer-target (#829) ownership change, and no change
  to the deliberate stored/derived dirty split (#851).
