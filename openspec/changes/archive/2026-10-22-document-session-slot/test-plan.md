# Test plan — the live edit session is one owned value

Two tables. The first is one row per spec scenario of the change's delta, with
the reason each row is red before the change. The second is the regression
guard: the behaviour the slot move must not disturb. Every row of the second
table is green today and must stay green.

## The delta's scenarios

| Scenario | Steps | Expected | Red before the change | Coverage |
|---|---|---|---|---|
| `ds_slot_kind` | Begin a Free Transform session, then ask the document session for the live kind and for every other kind's state through the slot | The session reports the Transform kind, and the slot holds no state for any other kind of session | Yes — the assertion goes through the slot's per-variant accessors, which do not exist until phase A, so the test does not compile | `pictura-app::the_slot_names_the_live_session_kind` |
| `ds_slot_one_state` | Open a filter preview on a layer's pixels, then ask the slot for the mask preview and cancel the session | The slot holds no mask filter preview, and cancelling restores the layer's pre-preview pixels | Yes for the first half — the assertion goes through the slot's filter-preview accessor, which does not exist until phase A; the cancel-restores half is green today and is the regression guard | `pictura-app::only_the_live_sessions_state_exists`, `pictura-app::impl_filters::cancel_restores_bit_identical` |
| `ds_slot_drag_handle` | Press a transform drag on a corner handle, read the handle over the bridge, then release and read it again | The handle is the corner's index while the drag is live, and -1 after the release | Yes — the bridge accessor does not exist until phase B, so the suite does not compile | `tst_document_session::theTransformDragHandleComesFromTheSession` |

## The regression guard

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ds_one_slot` | Begin a healing gesture, then begin a paint stroke | The paint stroke is refused, the healing gesture stays live, and no history state is added | `pictura-app::a_second_session_is_refused` |
| `ds_update_begin` | Update a filter preview repeatedly, then begin a mode preview, then begin a magnetic lasso and attach lasso points to it | Exactly one session stays live in each case, and the update lands on it | `pictura-app::update_begins_keep_one_session` |
| `ds_replace_target` | Begin a Free Transform on one layer, then on another | The first session ends without history and one transform is live on the second layer | `pictura-app::a_transform_on_another_target_replaces_the_session`, `tst_transform_api::quadAndPreviewMatrixMatchTheLegacyFormat` |
| `ds_transform_mode_switch` | Begin a Free Transform on a path, then begin a Skew session on it | The first session ends, one session is live in the new mode, and no history state is added | `pictura-app::a_transform_mode_switch_replaces_the_session` |
| `ds_failed_transform_begin` | Begin a transform that trims the layer and then fails to build its preview | The layer's pre-trim rectangle is restored and no session is live | `pictura-app::a_failed_transform_begin_leaves_the_layer_unchanged`, `tst_transform_controls::hiddenLayerRefusesTransforms` |
| `ds_replace_cancels` | Replace the document while a paint stroke is live; the replacement calls the same cancel path the per-kind cancel tests drive | The installed document holds none of the outgoing stroke's pixels and no session is live | `pictura-app::replacing_a_document_cancels_every_session` |
| `ds_replace_clears` | Replace a document that carries a mask edit target, a selected saved path, a ruler, mask solo/overlay, an isolation set, pending previews, and PDF options | The mask target and saved path are unset, the ruler, isolation, solo/overlay, and pending previews are gone, and the generation changed | `pictura-app::replacement_clears_outgoing_state` |
| `ds_undo_cancels` | Begin a session, then call the cancel path `undo` takes | The session is gone and no history state is added; the history restore itself is covered by the history tests | `pictura-app::undo_cancels_every_session`, `tst_document_session::undoAndRedoCancelTheTransformSession` |
| `ds_redo_cancels` | Begin a session, then call the cancel path `redo` takes | The session is gone and no history state is added; the history restore itself is covered by the history tests | `pictura-app::redo_cancels_every_session`, `tst_document_session::undoAndRedoCancelTheTransformSession` |
| `ds_jump_cancels` | Begin a filter preview, then call the cancel path a history jump takes | The preview is discarded and no history state is added | `pictura-app::jump_cancels_a_filter_preview` |
| `ds_snapshot_cancels` | Begin a mode preview, then call the cancel path a snapshot restore takes | The preview is discarded and no history state is added | `pictura-app::snapshot_restore_cancels_the_session` |
| `ds_cancel_restores` | Begin a selection move and cancel it | The selection and the drag state are back to their pre-drag values | `pictura-app::cancelling_every_session_restores_state` |
| `ds_cancel_no_history` | Cancel a live session | No history state is added | `pictura-app::cancelling_records_no_history` |
| `ds_cancel_noop` | Cancel with no session live | Nothing changes and nothing is presented | `pictura-app::cancelling_without_a_session_is_a_noop` |
| `ds_report_shape_path` | Abort a path gesture on a shape layer's outline | The outline is restored and the abort asks for a rebuild | `pictura-app::a_shape_path_abort_requests_a_rebuild` |
| `ds_headless_session` | Drive the document session with no QObject: install, record, undo | The edit is recorded, the undo steps back, and the document stays installed | `pictura-app::a_session_runs_without_a_qobject` |
| `ds_inplace_handlers` | Replace the document in place and let the handlers react | The tool handlers' own gesture state is reset | `tst_document_session::inPlaceRestoreResetsToolHandlers` |
| `ds_tool_switch_cancel` | Switch tools during a cancellable gesture | The gesture is cancelled | `tst_document_session::switchingToolsCancelsCancellableGestures` |
| `ds_tool_switch_commit` | Switch tools during a commit-on-deactivate gesture | The gesture commits | `tst_document_session::switchingToolsCommitsCommitOnDeactivate` |
| `ds_replace_signals` | Replace the document through an entry point | The entry point emits the same signals it emits today | `tst_document_session::replacementSignalsMatchTheEntryPoint` |
| The paint session's present frame | Paint a stroke and present it before the flush, then flush | The dabs accumulate until the flush and the stroke's lifecycle supersedes them | `pictura-app::present_accumulates_until_flush_and_the_stroke_lifecycle_supersedes_it` |
| The reduced-level preview | Paint a stroke too large for the raster budget, then rebuild the frame | The preview snapshots the stored level and stays small, and a full rebuild drops it | `pictura-app::the_preview_snapshots_the_stored_level_and_stays_small`, `pictura-app::reset_pyramid_drops_a_live_preview` |
| A stroke's commit pixels | Commit a stroke that touched several rectangles | The committed pixels equal a full recomposite of the document | `pictura-app::a_multi_rect_stroke_commit_equals_a_full_recomposite` |
| The frame mid-stroke | Recomposite while a stroke is live | The pyramid matches a full recomposite of the working document | `pictura-app::mid_stroke_pyramid_matches_a_full_recomposite_of_the_working_document` |
| A filter commit with a live transform | Begin a transform, then commit a filter | The filter lands and the transform still owns the slot | `pictura-app::a_filter_commit_leaves_a_live_transform` |
| Cancelling a stroke with another session live | Make a mode preview live, then cancel a stroke | There is no stroke to cancel and the mode preview still owns the slot | `pictura-app::cancelling_a_stroke_leaves_another_session_live` |
| The paint timing | Run the paint scenarios before and after the change | The dab and present timings do not regress | manual: the timing is read from the app's own profile output, so no test asserts it |
