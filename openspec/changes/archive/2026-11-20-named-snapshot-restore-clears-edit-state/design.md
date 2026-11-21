# Design

## Context

The document session owns the document, selection, history, and live edit
session. The Qt adapter keeps the presentation state around it. A normal
history restore clears Fade in `PictureViewRust::restore`, but the named-snapshot
path (`history_restore_snapshot`) replaces the document itself and calls
`session.record` directly, so it never reaches the adapter's record path and
never clears Fade. The same path hand-resets a subset of the adapter's
document-scoped fields, and `clear_document_scoped_state` has fallen behind the
fields added since it was written.

## Goals / Non-Goals

**Goals:**

- One owner for the edit-state slice that must not cross a document boundary.
- One clear operation called by document replacement and named-snapshot restore.
- A named-snapshot restore that owns its document/selection/Quick Mask/
  generation/history work instead of reimplementing it in the adapter.
- Replacement resets every document-scoped field the adapter keeps.

**Non-Goals:**

- Moving the nine QImage preview caches or the display frame into the session
  (#740, #783).
- Moving layer-target ownership into the session (#829); the target is still
  retained by the adapter and recorded by the session.
- Changing the stored/derived dirty split (#851).
- Clearing the extra preview caches on a named-snapshot restore; the issue's
  scope resets them on document replacement.

## Decisions

### The session owns the edit-state slice

`DocumentSession` gains `fade`, `last_transform`, `link_sets`, `link_remap`,
`solo_snapshot`, and `pending_previews`, and `clear_edit_state` clears all six.
The adapter calls it from `clear_document_scoped_state`, and the snapshot
restore calls it inside `restore_named_snapshot`. This removes the second,
hand-maintained reset list for the slice and makes it impossible for the
snapshot path to forget Fade.

### The snapshot restore moves into the session

`DocumentSession::restore_named_snapshot(snapshot, target, label)` installs the
snapshot's document, prunes layer-comp warnings, retains the incoming target
against the restored document, restores the selection and Quick Mask, clears
the deselected-selection memory, clears the edit state, records the restore, and
bumps the document generation. It returns the retained target so the adapter can
set its layer target and continue with display work.

The adapter still owns the display-frame reset, the Quick Mask invalidation, the
path/mask/ruler/canvas-proxy resets, and the presentation choice between the
stored composite and a recomposite.

### Replacement resets the drifted fields

`clear_document_scoped_state` also clears `path_drag_damage`, `move_above`,
`move_above_origin`, `move_above_scale`, `move_above_presented`,
`transform_preview_level`, `smart_filter_edit`, and `warp_text_original`.

## Risks / Trade-offs

- **A test constructs a `FadeState` directly.** The fade record is created
  through `fade::capture` and `fade::record` instead, so the test exercises the
  real record path and keeps the record's fields private.
- **A named-snapshot restore still leaves the extra preview caches alone.**
  Intended: the acceptance scope resets those on document replacement; the
  snapshot restore only shares the six-field edit-state clear.
