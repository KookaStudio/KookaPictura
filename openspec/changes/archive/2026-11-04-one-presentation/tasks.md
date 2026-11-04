# Tasks

## 1. Presentation seam

- [x] 1.1 Add `Presentation`, `Rebuild`, `PresentStep`, `plan()`, and `present_core()` in `cxxqt_object/presentation.rs`; register `mod presentation;` in `cxxqt_object.rs` and the `#[cfg(test)] mod presentation_tests;` module in `presentation.rs`. Add the plan tests (`regions_precede_notify`, `one_notify_per_presentation`, `silent_region`, `notify_without_regions`, `rebuild_once`, `deferred_rebuild_plan`, `bounded_does_not_rebuild`, `bookkeeping_precedes_blit`, `rebuild_always_notifies`); verify `cargo test -p pictura_app presentation_tests` passes the plan tests.
- [x] 1.2 Add `PictureView::present()` that wraps `present_core` and emits the Qt signals; route every producer presentation through `present_core` and every Qt emission through `present` (the generated signals stay public). Verify the plan-to-presenter test passes and `cargo clippy -p pictura_app --all-targets -- -D warnings` is clean.

## 2. Edit transaction

- [x] 2.1 Extend the edit rows in `presentation_tests` (`regions_precede_notify` and `bookkeeping_precedes_blit` with the `apply_edit` producer case, `presenter_applies_bookkeeping`, `move_commit_keeps_revision`, `bounded_edit_equivalent`, `unbounded_edit_equivalent`); verify they fail before task 2.3. The all-five `one_value_for_every_producer` row is added in task 7.2.
- [x] 2.2 Split `composite_full` into `composite_into_doc` plus `reset_pyramid`; verify the full-composite counts in `damage_policy_tests` are unchanged.
- [x] 2.3 Add `capture_live_without_bookkeeping`; make `apply_edit` return `Presentation` with `declaration: Some(normalized Edit)`; switch `refresh_for` to `composite_into_doc`; delete `Applied`, `Effect`, and `effect_of`; update every direct caller and test (`picture.rs`, `tests_profiles/*`, `metrics_tests.rs`, `tool_recording_tests.rs`, `display_preview.rs`, `damage_policy_tests.rs`, `impl_paint/tool_recording/imp.rs`, `impl_selection.rs`, `layer_arrange/imp.rs`, `impl_transform/session/move_profile.rs`); verify the task 2.1 tests pass.
- [x] 2.4 Route `PictureView::edit`, `commit_move_edit`, and `preview` through `present_core`/`present`; make `preview_edit` return `Presentation` too; verify `tst_edit_refresh` and `tst_document_session` pass.

## 3. Session cancellation

- [x] 3.1 Add the session-cancel row and the `silent_region` producer case; verify they fail before task 3.2.
- [x] 3.2 Make `cancel_active_sessions` composite and apply its regions, then return `Presentation`; convert `cancel_path_gesture` (`session.rs`), `path_gesture_abort` (`paths/imp.rs`), `impl_history::restore`'s `changed_display` check, and `document_session/tests.rs`; delete `SessionChange`, `RegionBlit`, `merge`, and `changed_display`; verify the task 3.1 tests and `tst_document_session::regionBlitDoesNotEmitChanged` pass.

## 4. History restore

- [x] 4.1 Add the restore row and `restore_derives_dirty`; verify they fail before task 4.2.
- [x] 4.2 Add `PictureViewRust::restore_presentation` converting `Restored` to `Presentation` (region apply, mask-solo refresh, deferred rebuild); stop `DocumentSession::restore` writing dirty and advancing `content_revision`; update `show_restored` to call `present()` and keep `session_cancelled`/`history_restored`; verify the task 4.1 tests and `tst_present_cache` pass.

## 5. Filter and adjust producers

- [x] 5.1 Add the filter-commit and adjust-poll rows and the `deferred_rebuild_producer` case; verify they fail before tasks 5.2 and 5.3.
- [x] 5.2 Make `commit_finish` return `Presentation`, stop setting dirty/revision and calling `reset_pyramid` (the presenter rebuilds), delete `Refresh` and `Landed`, and update `filter_tools/jobs/imp/tests.rs`; verify it passes.
- [x] 5.3 Make `preview_poll` apply its pixels and return `Option<Presentation>`, stop advancing the revision, delete `AdjustRefresh`, delete the now-no-op filter-preview baseline guard, and update the Qt wrapper; verify `tst_adjustments_panel` and `tst_properties_preview` pass.

## 6. Preview twins and display fast path

- [x] 6.1 Add `opacity_and_fill_share_path` and `display_preview_uses_presenter`; verify they fail before task 6.2.
- [x] 6.2 Add `preview_layer_property`/`commit_layer_property` as Rust helpers returning `(i32, Presentation)`; keep the Qt wrapper signatures; route the display fast path through `present()`; verify the task 6.1 tests and `tst_layers_preview_coalescing` pass.

## 7. Deletion and integration

- [x] 7.1 Delete the remaining dead report types and update every call site; verify `cargo clippy -p pictura_app --all-targets -- -D warnings` is clean.
- [x] 7.2 Add the `one_value_for_every_producer` table row covering all five producers and run `cargo nextest run -p pictura_app`; verify it passes (655 passed, 37 skipped).
- [x] 7.3 Run `tst_document_session`, `tst_edit_refresh` (shards `_0`..`_3`), `tst_properties_preview`, `tst_layers_preview_coalescing`, and `tst_present_cache`; verify all pass.
