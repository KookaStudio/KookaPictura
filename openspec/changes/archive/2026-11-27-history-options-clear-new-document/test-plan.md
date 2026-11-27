# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `hp_options_dialog` | Open `History Options…` from the History panel menu | Each checkbox reflects the stored option value | `tst_history_options::optionsDialogShowsCurrentValues` |
| `hp_options_persist` | Change the values, accept, reload the session store, reopen the dialog | The accepted values are shown | `tst_history_options::optionsPersistAcrossRestart` |
| `hp_clear_keep_snapshots` | Make several edits, add a snapshot, run plain Clear History | One current state remains, the snapshot remains, pixels unchanged | `tst_history_options::clearKeepsSnapshots` |
| `hp_clear_purge` | Make several edits, add a snapshot, run Alt Clear History | One current state remains, no snapshot remains, undo unavailable | `tst_history_options::altClearDropsSnapshots` |
| `hp_new_document_state` | Edit twice, select the middle state, choose New Document | A new untitled document shows the middle state's pixels and one `Duplicate State` state | `tst_history_options::newDocumentFromState` |
| `hp_new_document_snapshot` | Add a snapshot, select it, choose New Document | A new untitled document shows the snapshot's pixels and one `Duplicate State` state | `tst_history_options::newDocumentFromSnapshot` |
| `hp_new_document_source` | Create a new document from a state and inspect the source | Source pixels, current state, history list, and snapshots are unchanged | `tst_history_options::newDocumentLeavesSourceUnchanged` |
| `hp_snapshot_dialog` | Turn the prompt option off, click the Create Snapshot button | A `Snapshot N` row appears with no dialog | `tst_history_options::snapshotButtonAutomatic` |
| `hp_nonlinear` | Turn non-linear on, step back twice, make an edit | The new state is appended and the later states remain | `tst_history_options::nonLinearAppend`, `pictura-app::nonlinear_capture_keeps_later_states` |
| `hp_nonlinear_undo` | With an appended non-linear state, undo | The previous recorded state is restored, not the branch point | `tst_history_options::nonLinearUndoOrder`, `pictura-app::nonlinear_undo_follows_recording_order` |
| `hp_visibility_off` | Turn visibility undo off, toggle a layer eye | Pixels change and the history count is unchanged | `tst_history_options::visibilityUndoOff` |
| `hp_visibility_on` | Turn visibility undo on, toggle a layer eye, undo | One state is added and undo restores the eye | `tst_history_options::visibilityUndoOn` |
| `hp_auto_first` | With the option on, create/open a document | `Snapshot 1` is listed | `tst_history_options::autoFirstSnapshotOnAdd` |
| `hp_auto_save` | With the option on, save a document | A named snapshot of the saved state is added | `tst_history_options::autoSaveSnapshot`, `pictura-app::save_finish_adds_snapshot_when_enabled` |
| `hp_materialize` | Materialize a retained middle state | The result equals that state; live document, cursor, and snapshots are unchanged | `pictura-app::materialize_middle_state_is_read_only` |
| `hp_engine_clear` | Clear a history with states and snapshots | One state and every snapshot remain; undo is unavailable | `pictura-app::clear_keeps_snapshots` |
| `hp_engine_purge` | Purge a history with states and snapshots | One state remains and no snapshot remains | `pictura-app::purge_drops_snapshots` |
| Successful command adds a history state | Run a tracked edit | The pre-edit state is undoable and depth grows by one | `pictura-app::labels_track_captures_and_undo_redo_position` |
| Failed command leaves history untouched | Run an edit rejected by validation | Depth and stack are unchanged | `tst_history_options::invalidEditLeavesHistoryUnchanged` |
| `eh_append_existing` | Record an edit on a document that already has history | Prior states and snapshots remain and undo returns to the pre-edit state | `pictura-app::labels_track_captures_and_undo_redo_position` |
| Oldest state falls off the bounded stack | Capture 21 states | The first state is gone and undo returns the state before the second | `pictura-app::depth_bounded_at_20_and_drops_oldest` |
| New op invalidates redo | Undo, then capture with non-linear off | Redo returns false | `pictura-app::new_capture_truncates_redo` |
