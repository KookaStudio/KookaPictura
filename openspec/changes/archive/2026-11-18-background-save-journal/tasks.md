# Tasks

## 1. Tests before implementation

- [x] 1.1 Extend `tst_offthread_jobs::saveKeepsTheEventLoopLive` to assert repainting, and add `nativeSaveCallersWriteSameBytes`, `backgroundSavePreferencePersists`, `backgroundSaveAcceptsInputAndShowsProgress`, and `backgroundSaveAsCompletesOnOrigin`. Verify the test slots cover the menu entry points with a held worker.
- [x] 1.2 Add `tst_offthread_jobs::backgroundSaveUpdatesTheSavedHistoryState`, `backgroundSaveRejectsConflictingWrites`, `backgroundSaveReportsUnconfirmedPublication`, `closeRechecksAfterBackgroundSave`, and `destructiveActionsResolveBackgroundSave`. Add `pictura-app::native_save_distinguishes_publication_failures`. Verify the assertions cover both sides of rename and the Save / Cancel close decisions.
- [x] 1.3 Add `pictura-app::recovery_batches_sync_during_continuous_edits`, `journal_io_failure_keeps_durable_prefix`, `journal_replay_stops_at_first_invalid_record`, and `interrupted_checkpoint_publication_preserves_journal_suffix`. Verify the fixtures model partial writes, ordering errors, and interruption at each publication boundary.
- [x] 1.4 Update `tst_recovery::autosaveSnapshotsOnlyUnsavedChanges`, `saveAndCloseDropTheSnapshot`, and `recoverReopensAnOrphanedDocument` for first-dirty checkpoints, quiesced cleanup, and an explicit legacy fixture. Add `largeCheckpointUsesPsb`, `journalRecoversCommittedDocumentState`, `journalSkipsUncommittedChanges`, and `disablingRecoveryDrainsPendingWrites`. Verify the tests cover native pixels and metadata, legacy discovery, and cleanup with a held writer.
- [x] 1.5 Add `tst_recovery::journalFailureReportsDegradedProtection`, `journalTailFailureRecoversValidPrefix`, `failedRecoveryTransferKeepsOrphanChain`, and `staleBackgroundSaveLeavesJournalRecoverable`. Verify the process-kill fixture skips normal teardown and asserts that the file and recovered edit diverge as expected.
- [ ] 1.6 Add `pictura-codec::recovery_resources_replace_duplicate_blocks` and `ordinary_native_saves_preserve_autosave_resource_bytes`, plus `tst_recovery::checkpointResourcesMatchTheContainer` and `generatedResourcesNeverDriveRecoveryOrLeak`. Add `tst_recovery::recoveredResourcesPreserveImportedBytes`. Verify every planned automated test is declared with `python3 scripts/check-test-plan.py` before implementation.

- [x] 1.7 Add `tst_offthread_jobs::uncertainPublicationProtectsCleanState`, `metadataOnlyEditDuringSaveRemainsDirty`, and `backgroundSaveRecordingKeepsDispatchOrder`, plus `tst_recovery::metadataOnlyEditSurvivesRecovery`, `legacyScannerCannotDeleteVersionedOrphan`, and `firstStoreSyncsAncestorEntries`. Add `tst_document_windows::closeDirtyDiscardClosesTab` and `closeDirtySaveClosesTab`. Verify recording success and failure by replay, recovery after a process interruption, old-scanner isolation, and fault injection for every newly created ancestor entry.

- [x] 1.8 Add `tst_open_save::untitledSaveRequestsPath`, `saveAsDefaultsToImportedFormat`, `layeredFlatSaveWarnsAndFlattens`, `layeredRasterDefaultsToPsd`, and `layeredRasterSaveOpensSaveAs`. Drive the dialogs offscreen and assert each complete outcome.

## 2. Native save worker and shell

- [x] 2.1 Add phase reporting and durable publication outcomes to the existing Rust save worker and safe I/O helper. Update the saved native identity, including persisted session metadata, at same-generation completion. Maintain a sticky prompt and recovery obligation for uncertain publication until resolved. Verify with the existing native worker tests and the injected publication failure checks at the final build.
- [x] 2.2 Route interactive native Save and Save As to coordinator-owned jobs. Preserve the synchronous completion API, reserve and resolve recording positions in dispatch order, capture command origin with the job, and claim destinations across native write entry points. Verify stale and replacement completion, undo/redo dirty tracking, conflicts, and byte equality.
- [x] 2.3 Persist Save in Background alongside the recovery settings. Wire phase progress into the originating tab and its visible document status. Verify the preference and progress scenarios and the existing event-loop test.
- [x] 2.4 Resolve pending saves before Close, Revert, quit, and restart recheck dirty state. Keep the existing save-and-wait path for unsaved-changes prompts. Verify close and lifecycle tests with success, failure, and Cancel.

## 3. Checkpoints and durable recovery journal

- [x] 3.1 Add checkpoint-only resource framing and durable, bounded original raw-section
  preservation and replacement on recovery, then use the actual PSD/PSB container in checkpoint publication and discovery. Verify foreign block preservation, Unicode serialization, PSB recovery, and generated-path isolation.
- [x] 3.2 Add the versioned, bounded journal record codec and forward document delta capture. Observe committed mutations, metadata-only session changes, and history navigation, reuse unchanged tiles, and use a checkpoint for changes that require a new baseline. Verify valid-prefix replay and the committed-state and preview checks.
- [x] 3.3 Implement the ordered recovery writer with timed and size-triggered syncing, durable sequence tracking, and degraded-protection reporting. Bound queued allocations and reuse the session encode cache. Verify batching during continuous edits, clean shutdown behavior, and the append/sync failure cases.
- [x] 3.4 Publish checkpoint generations and retained journal suffixes through the durable manifest. Isolate versioned storage from legacy scanners, sync new ancestor directory entries, read legacy manifests, validate local references, and hold the orphan lock during final recovery decisions. Verify interrupted publication, legacy and partial recovery, memory refusal, and retained orphan data after transfer failure.
- [x] 3.5 Make recovery cleanup revision-aware and invalidate outstanding writes on discard, current successful save, and preference disable. Update the #637 roadmap entry for the shipped behavior. Verify save/close cleanup and the process-kill race.

## 4. Integration verification

The tests for task 1.6 are written. The app and Qt test executables built with
the 80% CPU budget. Strict OpenSpec validation passed. CI will rebuild, run
the automated tests, and check the guards. Large-document observations and
the manual power-loss scenario remain reviewer checks.

- [ ] 4.1 After code and tests are complete, build once with at most 80% of available CPUs. Run the focused Qt and Rust checks from the test plan. Verify the journal cadence and memory behavior under a large document workload and record the observations.
- [ ] 4.2 Exercise the manual power-loss scenario on an appropriate machine or VM, or record it as a pending reviewer check. Include every manual row in the eventual PR body.
- [ ] 4.3 Archive the completed change with its spec deltas, then run `bash scripts/verify-fast.sh` against the completed build. Verify strict OpenSpec validation, the archive guard, and that the archived artifacts and roadmap match the shipped change.
