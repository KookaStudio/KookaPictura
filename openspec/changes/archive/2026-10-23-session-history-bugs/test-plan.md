# Test plan

Every scenario in the delta specs has a row. Rows in the first table are the
change; rows in the second table are existing behavior that must not regress.

## Delta scenarios

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Successful command adds a history state | Commit a filter or adjustment, then undo | The pre-command document and selection return and the depth increases by one | `pictura-app::one_commit_refreshes_captures_and_versions` |
| Failed command leaves history untouched | Dispatch a command with an unknown kind or no document | The history depth and stack are unchanged | `pictura-app::a_refused_mutation_records_nothing` |
| `eh_append_existing` | Make two edits and add a named snapshot, then trim the document | The two prior states and the snapshot remain, and Trim is the current state | `tst_image_ops::trimIsUndoable` |
| `eh_snapshot_undo` | Make edit A, snapshot it, make edit B, restore the snapshot, press Undo | The document, selection, and history cursor return to B | `tst_document_session::snapshotRestoreMovesCursor` |
| `eh_snapshot_redo` | From the state above, press Redo | The document, selection, and layer target equal the restored snapshot | `tst_document_session::snapshotRestoreMovesCursor` |
| `eh_snapshot_after_undo` | Make edit A, snapshot it, make edit B, undo to A, then restore the snapshot | One state is appended after the undone B is discarded; Undo returns to the state current before the restore and Redo returns to the snapshot | `tst_document_session::snapshotRestoreAfterUndoDropsRedo` |
| `eh_trim_undoable` | Edit a document, add a snapshot, select a layer target and a region, run Image > Trim, press Undo | The pre-Trim pixels, selection, layer target, and history depth return; the snapshot remains | `tst_image_ops::trimIsUndoable` |
| `eh_trim_noop` | Run Trim when the content fills the canvas and on an empty composite | No state is added and the document, selection, dirty state, and layer target are unchanged | `tst_image_ops::trimDocument` |
| A mutating command marks modified | Commit a filter, adjustment, document op, or selection command | `is_dirty()` is true | `pictura-app::one_commit_refreshes_captures_and_versions` |
| Save clears modified | Save a modified document successfully | `is_dirty()` is false | `tst_document_api::saveAndReopenRoundTrip` |
| Open clears modified | Open a document | `is_dirty()` is false | `tst_document_api::saveAndReopenRoundTrip` |
| `ml_history_navigation` | Save, edit, undo to the saved state, then redo, jump to the edit, and restore a snapshot | `is_dirty()` is true at every destination other than the saved state | `tst_document_session::historyNavigationMarksDirty` |
| `ml_return_to_saved` | Save, edit, then undo | `is_dirty()` is false at the saved state | `pictura-app::returning_to_the_saved_state_clears_dirty` |
| `ml_saved_state_dropped` | Save, then make enough new edits or trigger enough bounded drops to evict the saved state | `is_dirty()` stays true and navigation cannot return to clean | `pictura-app::a_dropped_saved_state_stays_dirty` |
| Revert reloads from disk | Revert a modified document with a path | The document matches the file and reports unmodified | `tst_document_session::revertIsUndoable` |
| Revert unavailable when untitled | Query Revert for a document with no path | Revert is unavailable | `tst_document_session::revertUnavailableWhenUntitled` |
| `dl_revert_undoable` | With an intact file, save PDF options, add a named snapshot, rotate the document, start a paint stroke, Revert, then Undo | The prior states and snapshot remain and one `Revert` state is appended; path, PDF options, and source format survive; Undo restores the pre-Revert pixels and Redo is saved | `tst_document_session::revertIsUndoable` |
| `dl_revert_failure_keeps` | Truncate or delete the file, or change a protected PDF's password, then Revert | Revert fails and the document, pixels, dirty state, path, history, and live session are unchanged | `tst_document_session::failedRevertKeepsDocument`, `tst_import_pdf::wrongPasswordRevertKeepsTheDocument` |
| `ds_replace_cancels` | Begin a paint stroke, filter preview, or transform, then Revert, New, or Open | The installed document has none of the outgoing session's pixels and no session is live | `pictura-app::replacing_a_document_cancels_every_session`, `tst_document_session::revertIsUndoable` |
| `ds_replace_clears` | Replace a document carrying a mask target, saved path, ruler, pending previews, a canvas proxy, and PDF options | The outgoing pointers and previews are cleared and the generation changed; New clears PDF options while Revert keeps path, password, options, and source format | `pictura-app::replacement_clears_outgoing_state`, `tst_document_session::revertIsUndoable` |
| `ds_pdf_password` | Open a protected PDF, then New or Duplicate; separately Revert a protected PDF | New/Duplicate clear the password; the PDF open installs it; Revert reuses and keeps it | `pictura-app::pdf_password_follows_the_incoming_document`, `tst_import_pdf::protectedPicturaPdfKeepsLayers` |
| `ds_reselect_after_replace` | Deselect a same-size selection, then replace the document and query Reselect | No prior selection is reported and applying Reselect changes nothing | `pictura-app::reselect_is_unavailable_after_same_size_replacement` |
| `ds_replace_history` | Open, Open Image, Open As Smart Object, New, or Duplicate a document | The history is exactly the incoming Open or Open As Smart Object state, one New state, or one Duplicate state, and that is the only undo target | `pictura-app::replacement_installs_incoming_history` |
| `ds_stale_async` | Deliver an open or filter result carrying a stale generation or request id | The late result is dropped and the installed document and history are unchanged | `pictura-app::a_stale_async_install_is_discarded` |
| `ds_replace_signals` | Replace through Open, New, Duplicate, or Revert with no live session | Open/New/Duplicate keep their current signal policy; Revert emits one full change and adds only its one state | `tst_document_session::replacementSignalsMatchTheEntryPoint`, `tst_document_session::revertIsUndoable` |
| `ds_revert_history` | Revert an intact file into a document with prior states and named snapshots | The prior states and snapshots remain and one `Revert` state is appended as current; Undo restores the pre-Revert document | `tst_document_session::revertIsUndoable` |
| `ds_revert_failure` | Revert a missing or corrupt file, or a protected PDF whose password changed | The document, pixels, selection, dirty state, path, history, and live session are unchanged | `tst_document_session::failedRevertKeepsDocument`, `tst_import_pdf::wrongPasswordRevertKeepsTheDocument` |
| `lock_layer_menu` | With a Free Transform pending, inspect and dispatch the Layer menu | Layer commands are disabled and refuse dispatch; they return after commit | `tst_transform_lock::layerMenuIsLockedWhilePending` |
| `lock_layers_panel` | With a Perspective session pending, inspect the Layers panel | The panel is disabled and returns after cancel | `tst_transform_lock::layersPanelIsGreyedWhilePending` |
| `lock_document_edits` | With a lifted transform pending, dispatch Edit > Fill/Stroke, call the direct fill/stroke/filter APIs, clipboard edits, assign/convert profile, add an adjustment, and run an image op; then commit the transform | Each edit is refused, the document and history are unchanged, and the transform's own commit records one state | `tst_transform_lock::documentEditsDuringPendingTransform` |
| `lock_controls_box` | With the Move tool's controls box in an active gesture, inspect Edit > Fill and a filter | Both remain enabled and the controls session stays live | `tst_transform_lock::documentEditsDuringPendingTransform` |

## Regression guard

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `et_one_owner` | Commit a layer change through the transaction | One state, one revision bump, and the declared compositing | `pictura-app::one_commit_refreshes_captures_and_versions` |
| `ds_one_slot` | Begin healing, then begin a paint stroke | The paint stroke is refused and no history state is added | `pictura-app::a_second_session_is_refused` |
| `ds_cancel_restores` | Begin a selection move and cancel it | The selection returns to its pre-drag value | `pictura-app::cancelling_every_session_restores_state` |
| `in-place history` | Undo after a filter with a GPU/CPU backend switch | The display equals a full recomposite within the 1-LSB GPU parity contract | `pictura-app::undo_display_from_snapshot_composite_equals_full_recomposite` |
| `trim existing behavior` | Trim a transparent canvas around a painted block, then trim again, then trim an empty composite | The canvas resizes to the content and both later trims record nothing | `tst_image_ops::trimDocument` |
| `save round trip` | Save a new document, reopen it, and read the path and mode | The round trip succeeds and the document is clean | `tst_document_api::saveAndReopenRoundTrip` |
| `non-lifted filter commit` | Begin a non-lifted Free Transform, then commit a filter | The filter lands and the transform stays live | `pictura-app::a_filter_commit_leaves_a_live_transform` |
