# Tasks

## 1. Tests

- [x] 1.1 Add `document_session::tests::snapshot_restore_clears_edit_state`:
  record a Fade, fill the edit-state slice, restore a named snapshot through the
  session, and assert the slice is clear, Fade is unavailable, and one labelled
  history state was recorded.
- [x] 1.2 Add `document_session::tests::replacement_clears_edit_state`: fill the
  same slice plus the drifted document-scoped fields and assert
  `replace_document` clears all of them.

## 2. Session ownership

- [x] 2.1 Move `fade`, `last_transform`, `link_sets`, `link_remap`,
  `solo_snapshot`, and `pending_previews` from `PictureViewRust` into
  `DocumentSession`.
- [x] 2.2 Add `DocumentSession::clear_edit_state` and call it from
  `clear_document_scoped_state`.
- [x] 2.3 Add `DocumentSession::restore_named_snapshot` owning the document,
  selection, Quick Mask, deselected-selection memory, generation, and record.
- [x] 2.4 Route `history_restore_snapshot` through the session method, keeping
  the adapter's target, display, and presentation work.
- [x] 2.5 Reset `path_drag_damage`, the four `move_above*` fields,
  `transform_preview_level`, `smart_filter_edit`, and `warp_text_original` on
  document replacement.

## 3. Verify

- [x] 3.1 Run fmt, clippy, the new and existing document-session tests, the Qt
  history/document-session suites, and `openspec validate --all --strict`; then
  archive the change.
