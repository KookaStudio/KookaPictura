# Tasks

Order follows `design.md`. The saved identity lands first because Revert and
snapshot restore depend on it.

## 1. #851 saved-state identity and dirty tracking

- [x] Add failing engine tests: `pictura-app::returning_to_the_saved_state_clears_dirty`, `pictura-app::a_dropped_saved_state_stays_dirty`, `pictura-app::purging_history_keeps_a_clean_document_clean`.
- [x] Add the failing Qt regression test: `tst_document_session::historyNavigationMarksDirty`; extend `tst_document_api::saveAndReopenRoundTrip` to assert the opened document is clean.
- [x] Add a per-state identity to `History` and expose the current identity; keep the 20-state bound, label/index/depth/count, and redo truncation unchanged. A purge keeps the surviving current state's identity; identities are never reused.
- [x] Add `DocumentSession::mark_saved()` and store `saved_id`; recompute `dirty` from `current_id != saved_id` on undo/redo/jump, and treat a view with no document as clean.
- [x] Route PSD, raster, and PDF save completions (including Save As) through `mark_saved()`; mark `New` saved after its initial capture; leave `Duplicate`'s recorded state dirty; clear the saved identity on recovery open and extend `tst_recovery::recoverReopensAnOrphanedDocument` to assert the recovered document stays dirty after an edit and undo.
- [x] Run the engine tests, then `tst_document_session`.

## 2. #854 Revert is atomic and undoable

- [x] Add the failing Qt tests: `tst_document_session::failedRevertKeepsDocument` (missing/corrupt file and a live paint session), `tst_import_pdf::wrongPasswordRevertKeepsTheDocument` (a protected PDF whose stored password no longer opens it), `tst_document_session::revertIsUndoable` (starts with a live session and checks `path`, password, options, and source format survive; Undo is dirty, Redo is saved), `tst_document_session::revertUnavailableWhenUntitled` (a New document and a raster import whose source path is only on the tab).
- [x] Extend `tst_document_session::replacementSignalsMatchTheEntryPoint` to cover Revert's one full-change signal; keep `pictura-app::replacement_clears_outgoing_state` for the New path and assert Revert's preserved file identity in `revertIsUndoable`.
- [x] Add a decode-only path for PSD/PSB, PDF (all `PdfOpen` variants), and raster that returns an opened document without mutating the view.
- [x] Implement in-place Revert: decode first; on failure return false; on success cancel, clear document-scoped state while preserving path/password/options/source format and the existing history, assign the decoded document/selection/target without `replace_document`/`install`, record one `Revert` state, mark saved, bump the generation, reset the pyramid, recomposite, and signal once.
- [x] Add `document_api::revert`; switch `PicturaMainWindow::revertActive` to it and keep the existing failure reporting. Gate the `File > Revert` action on the document path (`document_api::filePath`), so an untitled raster/smart-object import with only a tab source path cannot invoke it.
- [x] Run only the new tests, then `tst_document_session`, the PDF import/save suites, and `pictura-app::pdf_password_follows_the_incoming_document`.

## 3. #853 pending transform blocks document edits

- [x] Add the failing Qt regression test: `tst_transform_lock::documentEditsDuringPendingTransform` (registry dispatch, direct fill/stroke/filter/adjustment/image-op calls, the transform's commit records one state afterwards, and the Move tool's controls box stays unlocked).
- [x] Update `tst_transform_lock::layerMenuIsLockedWhilePending` for the broadened blocked set while keeping the transform commands enabled.
- [x] Add one **lifted** modal-transform predicate carrying the controls-box exclusion (a non-lifted transform still accepts direct document edits), and check it before mutation at `PictureView::apply_filter`, `apply_mask_filter_command`, `fills::apply`, `add_adjustment`, `clipboard_cut`/`clipboard_clear`/`clipboard_paste`, `apply_op_active_region` (preview and commit), `commit_start`/`commit_filters`, and every public image/document-op entry that writes `session.doc` before `edit` (`resize_image`, `resize_canvas`, `rotate_doc`, `flip_doc`, `rotate_document_arbitrary`, `crop`, `canvas_size_apply`, `image_size`, `convert_depth`, `assign_profile`, `convert_profile`, `convert_image_mode`, `convert_image_depth`, `convert_to_indexed`).
- [x] Broaden the blocked-command provider in `frame_transform_lock.cpp` to the `Layer`, `Image`, and `Filter` menus plus `Edit > Assign Profile…`, `Edit > Convert to Profile…`, `Edit > Cut…`, `Edit > Fill…`, `Edit > Stroke…`, `Edit > Clear`, and the Paste family, while leaving undo/redo, history navigation, view commands, and the transform commands enabled.
- [x] Verify the transform's own commit still records one state after a refused edit, and that committing an already-live filter preview still works. (the guard is lifted-only; `a_filter_commit_leaves_a_live_transform` pins the non-lifted case)
- [x] Run only that test, then `tst_transform_lock` and `tst_fill_tools`.

## 4. #850 Trim is undoable

- [x] Update `tst_image_ops::trimDocument` to assert the pre-existing history survives, and add `tst_image_ops::trimIsUndoable`.
- [x] Replace Trim's `finish` call with an in-place crop and one `Trim` record; keep `finish` for Duplicate.
- [x] Refuse Trim while a live session or an opacity/fill/mask-refine preview is active before mutating; validate the bounds first so a full-canvas or empty composite is a no-op; keep the current layer target when it resolves, otherwise select the top layer; clear the post-Trim selection.
- [x] Run only `trimIsUndoable`, then `tst_image_ops` and `tst_edit_refresh`.

## 5. #852 snapshot restore moves the cursor

- [x] Add the failing Qt regression tests: `tst_document_session::snapshotRestoreMovesCursor` and `tst_document_session::snapshotRestoreAfterUndoDropsRedo`.
- [x] Record one state labelled with the snapshot label after installing the snapshot; remove the direct `set_current_target` call.
- [x] Assert undo returns to the pre-restore state and redo returns to the snapshot; keep the snapshot list unchanged; restoring after undo discards the redo branch.
- [x] Run only those tests, then `tst_document_session`.

## Final

- [x] `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/verify-fast.sh`
- [x] `openspec validate --all --strict`
- [x] `openspec archive session-history-bugs`
- [ ] PR body lists the manual rows as a reviewer checklist (none are planned; PR not created yet)
