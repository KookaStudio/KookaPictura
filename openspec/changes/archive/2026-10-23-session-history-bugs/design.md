# Design: Session and history data-loss bugs

## Approach

The saved state is a history identity, not a boolean. Revert and Trim join the
normal history path instead of replacing the document. Snapshot restore appends
a state. A pending transform gates every document mutation at the entry points
before they touch the document.

Order: #851 saved-state identity -> #854 Revert -> #853 transform gate -> #850
Trim -> #852 snapshots. The identity lands first because Revert and snapshot
restore both depend on it. The transform gate lands before Trim so Trim can
refuse a pending transform before it mutates.

## Decisions

### #851 — saved state is a history identity

`History` already owns the cursor and a bounded list of states. Add a monotonic
identity to each state and expose the current identity. `DocumentSession` keeps
the `dirty` flag and stores the identity that was current at the last successful
save/open/install; history navigation recomputes `dirty` from
`history.current_id() != saved_id`. A view with no document is unmodified. A
`saved_id` of `None` with a document means there is no saved point and any state
is dirty.

- Save paths (`save`, PDF save, Save As) mark the current identity saved.
- `install` marks the history's current identity saved. A successful Revert
  records its `Revert` state and then marks it saved.
- `New` captures its initial state after installing the empty history, so it
  must mark that state saved after the capture.
- Undo, redo, and history jump recompute dirty; returning to the saved identity
  clears it. A snapshot restore appends a new identity, so it marks the document
  modified.
- Bounded drops and redo truncation remove identities; because identities are
  never reused, a dropped saved identity can never match again, so the document
  stays dirty. `purge_states` and `purge` keep the surviving current state's
  identity, so purging history cannot dirty a clean document.
- Duplicate keeps its current replacement semantics: its `Duplicate` state is
  recorded after installing the empty history, so the new document is dirty.
- Recovery installs an untitled document with no saved point: clear `saved_id`
  and set dirty, so undoing back to the recovered state cannot report clean.

Keep the existing direct `dirty = true` assignments for previews; a recovery
open additionally clears the saved identity. The saved identity is the authority
when history navigation completes.

Rejected: always setting `dirty = true` on navigation. It is smaller but makes
undoing back to the saved state report a phantom unsaved change, which
contradicts the `docs/` model and the "dirty follows the stack revision" rule.

### #854 — Revert decodes before it installs

Add one Rust entry point that resolves the path kind and decodes into an
`OpenedDocument` without touching the live session: PSD/PSB through
`decode_psd`, PDF through `decode_pdf` with the stored password, raster through
the import decoder. A missing or corrupt file, or a PDF that needs a password
the document does not have or whose stored password is wrong (all `PdfOpen`
failures included), returns false with no mutation and no session cancellation.

On success:

1. Cancel the live session against the outgoing document.
2. Clear the outgoing selection, prior-selection memory, mask target, the active
   Work/Saved path, previews, ruler, and canvas proxy, while keeping the file
   identity (`path`, `session.pdf_password`, `session.pdf_options`,
   `source_format`) and the existing `History` with its named snapshots.
3. Install the decoded document, selection `None`, and the decoded layer target
   without calling `replace_document` or `DocumentSession::install`, both of
   which replace the history.
4. Record one state labelled `Revert` on the preserved history.
5. Mark the new state saved, reset the pyramid, bump the document generation,
   recomposite, and emit one full change.

C++ `revertActive` calls the new `document_api::revert` for every path kind,
including PDF, and keeps its current failure reporting. The `File > Revert`
action is enabled from the document path (`document_api::filePath`), not the
tab's source path, so a raster or smart-object import that is still untitled
cannot invoke Revert. The synchronous `PictureView::open` is left alone: it
remains the fresh-history Open entry point and its failure behavior is not part
of this change.

Rejected: load through `open`, then restore the old history. It still wipes the
document before the decode result is known and cannot distinguish a failed load
from a successful one without a second decode.

### #850 — Trim crops in place

`trim_image` already computes the content bounds and crops a clone. Refuse first
while any live edit session or pending opacity/fill/mask-refine preview is
active (a modal transform is a document operation under #853), so preview pixels
cannot affect the bounds. Then compute and validate the bounds: a full-canvas or
empty composite returns false. Crop the live document in place, clear the
selection (its coordinates no longer map), and record one `Edit::Full` state
labelled `Trim`. Keep the current layer target when it still resolves; otherwise
select the top layer. Do not call `replace_document`, so the history, cursor, and
named snapshots survive. The pre-Trim selection is captured by the normal history
path; Undo restores it.

`finish` remains the replacement path for Duplicate only.

### #852 — snapshot restore appends a state

After installing the snapshot document, selection, and target, record one state
labelled with the snapshot's label using the normal capture path. That truncates
redo, moves the cursor, makes the pre-restore state the undo target, and marks
the document modified because the appended state has a new identity. Remove the
direct `set_current_target` call; `record` owns `target_before/target_after`.
The snapshot list is untouched.

Rejected: jumping the cursor to the snapshot's original index. The snapshot does
not retain one, and jumping would not make the pre-click state the undo target
the issue requires.

### #853 — one transform predicate at the mutation entries

Add one predicate: a **lifted** modal transform session is live. A non-lifted
transform accepts direct document edits — the edit lands in the layer and the
transform commit keeps it (`a_filter_commit_leaves_a_live_transform` pins this);
only a lifted selection restores its pre-lift snapshot and would discard the
edit. The predicate SHALL exclude the Move tool's controls box, matching
`transformPending()`; the session needs a flag (set by the C++ begin path)
because `history_label` is `"Free Transform"` for both.

Check the predicate before the document is touched at every mutation entry:

- `PictureView::apply_filter` and `apply_mask_filter_command` before they mutate.
- `fills::apply` before cloning the document.
- `add_adjustment` before it pushes the adjustment layer.
- `clipboard_cut`/`clipboard_clear` and `clipboard_paste` before they mutate.
- `apply_op_active_region` for both preview and commit. The existing
  `!commit && !slot_allows(FilterPreview)` check stays; the new check is
  separate, so committing a filter preview that is already live still works.
- The background filter commit starter (`commit_start`/`commit_filters`) before
  it takes the document and history.
- The image-op entries that mutate before calling `edit`: `resize_image`,
  `resize_canvas`, `rotate_doc`, `flip_doc`, `rotate_document_arbitrary`, `crop`,
  `canvas_size_apply`, `image_size`, `convert_depth`, plus the Image > Mode and
  profile-conversion entries (`assign_profile`, `convert_profile`,
  `convert_image_mode`, `convert_image_depth`, `convert_to_indexed`).

Guard the entries rather than `PictureView::edit`: selection and metadata
commands mutate before they call `edit`, so a guard there would run after the
mutation and leave a partial state. The registry is the gate for the menu
commands; the Rust checks are the backstop for the pixel-mutating API entries.

A refused entry SHALL leave the document bit-identical, add no history, and
leave the transform pending.

GUI enablement stays in `frame_transform_lock.cpp`: broaden the blocked-command
provider from `path.first() == "Layer"` to the document-editing set — the
`Layer`, `Image`, and `Filter` menus plus `Edit > Assign Profile…`,
`Edit > Convert to Profile…`, `Edit > Cut…`, `Edit > Fill…`, `Edit > Stroke…`,
`Edit > Clear`, and the Paste family. Do not block the whole `Edit` menu: leave
undo/redo, history navigation, view commands, `Edit > Free Transform`, and the
`Transform` submenu enabled. The Rust predicate is the backstop for API/headless
paths.

Rejected: force-committing the transform from every edit entry. It is more
surprising, and `docs/02-ui-ux/menus.md` requires inapplicable commands to be
disabled.

## Risks

- History identity touches every save path; missing one leaves a document dirty
  after save. Mitigation: route all save completions through one
  `mark_saved()` helper and test PSD, raster, and PDF save.
- Revert keeps the old history, so the pre-Revert state may be the current state
  of a document with a different geometry. `History::capture_live` already
  starts a new segment on a geometry change.
- The transform predicate must not block a non-lifted transform's direct edits
  or the transform's own commit. The existing
  `a_filter_commit_leaves_a_live_transform` test pins the non-lifted case; the
  commit takes the session out and uses `record`/`edit`, not the guarded pixel
  entries. Add a regression test for committing after a refused lifted edit.
- The Rust predicate can block the Move tool's controls box unless the session
  carries the modal flag; add a check that the controls box leaves commands
  enabled.
- The modified `document-session` requirement is large; copy it whole to avoid
  losing scenarios at archive time.

## Constraints

- Follow `docs/dev/testing-conventions.md`: Qt Test for GUI, std `#[test]` for
  engine.
- Do not weaken or delete an existing test; update a baseline only with a
  before/after assertion.
- Keep each file under its size cap; split along the seams named above.
