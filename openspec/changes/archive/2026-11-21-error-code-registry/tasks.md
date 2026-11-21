# Tasks

Write each group's test cases before its implementation. Keep compilation and test execution in the final verification step.

## 1. Shared registry and unwind guard

- [x] 1.1 Add the `pictura-error` crate scaffold and write `operation_status_registry_is_stable`, `unknown_status_preserves_its_number`, and `guard_preserves_results_and_contains_panics`. Check that each asserts the corresponding scenario in the test plan.
- [x] 1.2 Implement the checked `ErrorCode` registry, raw `OpStatus` representation, canonical keys, typed source chain and unwind guard using workspace `thiserror`. Verify against the three registry tests, including the C-calling-convention return and a subsequent successful call.

## 2. Native Open

- [x] 2.1 In `tst_loading_tab`, add `nativeOpenReportsPerRequestErrorKinds`, `nativeOpenPanicCompletesAndAllowsRetry`, and `nativeOpenCancelDiscardsLateFailure`. In `tst_control_limits`, add `nativeOpenMemoryRefusalIsPerRequest` for both control routes and overlapping requests. Verify the fixtures correlate terminal statuses with their request tokens and can time out on missing completion without hanging the suite.
- [x] 2.2 Keep file I/O, codec refusal, and memory-budget feedback tied to the native request in the loader. Guard staged native Open work and route successful, failed, and cancelled results through the typed Qt completion. Carry request-owned memory feedback through both ControlServer routes, preserving synchronous recovery callers and protocol codes. Verify through the four Open cases and the existing loading-tab and memory-governor suites.

## 3. Native Save

- [x] 3.1 In `tst_offthread_jobs`, add `nativeSaveDistinguishesCodecAndIoErrors`, `nativeSavePanicBeforeRenameCleansUp`, `nativeSavePanicAfterRenameProtectsDocument`, and `nativeSaveCopyPanicAfterRename`. Add Rust test `native_save_replaced_session_panic` using the generation seam. Verify the injected faults exercise the actual encoder, temporary writer, and post-rename sync stages used by native Save.
- [x] 3.2 Preserve typed codec and I/O causes at the native stream boundary. Keep temporary-file cleanup safe during unwind and record successful rename before the parent sync step. Verify the separate status and file contents in the save cases.
- [x] 3.3 Carry save status and publication outcome through the Rust and Qt adapters for native Save, Save As and Save a Copy. Use copy-specific uncertainty feedback without changing the source session. Verify the save cases plus existing `tst_save_gif_copy` and `pictura-app::copy_intent_skips_path_and_dirty` coverage, background save, document generation, action recording, and recovery coverage.

## 4. Shared whole-document commands

- [x] 4.1 In `tst_document_jobs`, add `rotationPanicLeavesDocumentAndHistoryIntact`, `validationRefusalClearsPreviousResult`, and `staleOrdinaryWorkerDoesNotPublish`. Exercise a real invalid preflight after a success, and hold an ordinary worker across a newer edit and session replacement. Extend `cancellingAFlattenChangesNothing` to assert the structured cancellation and the absence of failure feedback. Check both rotation execution modes against the test plan.
- [x] 4.2 Stage synchronous work, guard both engine paths, preserve typed job outcomes and publish only the current successful result. Capture generation, revision and history identity for every shared worker and suppress stale publication without overwriting newer status. Route early preflight and immediate outcomes to the Qt command adapter, distinguishing invalid arguments, admission refusals, and no-ops. Retain the existing progress and presentation paths. Verify the four planned command cases and the existing success and progress cases.

## 5. Shared filter and adjustment commit worker

- [x] 5.1 In `tst_offthread_jobs`, add `filterAndAdjustmentPanicsPreserveUndo`, `lockedLayerReportsFilterRefusal`, and `adjustmentDialogOkReportsPanic`, activating the real dialog OK. Check that the panic fixtures reach staged history capture and the adjustment engine independently and exercise the retained states.
- [x] 5.2 Preserve the live undo stack while the guarded worker stages a document and captures history, then publish both together on success. Return a typed refusal or panic through the shared filter and adjustment Qt path and display operation-specific failure feedback for adjustment dialog OK. Verify all three planned cases and the existing off-thread filter and adjustment behavior.

## 6. Integration and release checks

- [x] 6.1 After all code and tests are complete, run the budgeted build once and the registry tests and affected Qt suites, including an optimized GUI build with unwind enabled. Fix build errors and rebuild only if code changed. Run `python3 scripts/check-test-plan.py` with the change active and `openspec validate --all --strict`.
- [x] 6.2 Update the #645 roadmap item once shipped and complete the active change checklist.

After the checklist is complete, archive the change through OpenSpec and verify all five capability deltas are applied. Run strict validation and the full budgeted verification gates after archive; when the verified build is fresh, use the fast gate without rebuilding.
