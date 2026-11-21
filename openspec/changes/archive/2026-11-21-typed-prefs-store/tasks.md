# Tasks

Complete test-authoring tasks 1.1, 2.1, 2.2, 2.6, 3.1, and 4.1 before starting the implementation tasks. Run the build after the code and tests are in place.

## 1. Typed preferences and recovery

- [x] 1.1 Add Rust cases `preference_layers_follow_precedence`, `preference_edits_merge_without_losing_unknown_keys`, `unchanged_preferences_do_not_rewrite_files`, `failed_preference_write_keeps_last_complete_file`, `future_preferences_remain_read_only`, `blocked_quarantine_preserves_original`, and `gpu_preference_round_trips_through_toml`. Use temporary roots and deterministic write failures.
- [x] 1.2 Add the app's Rust preferences module and document the new TOML dependency in the change result. Use typed defaults, validation, sparse overrides, schema handling, and retained unknown values. Verify the precedence and GPU round-trip cases.
- [x] 1.3 Add the write lock, edit merge, atomic write, last-good backup, and quarantine handling. Verify merge, no-op save, failure, quarantine, and read-only cases against the persisted bytes.

## 2. Shell integration and migration

- [x] 2.1 Add `tst_preferences_store` with `corruptPreferencesCannotRestoreAgentGrants` covering a staged v1 and v2 import, retries, reset, a later launch and an explicit fresh grant, `freshProfileUsesSeparateRoots`, `preferencesReachConsumersAfterRelaunch`, `localeDoesNotChangeStoredNumbers`, `mixedLegacyProfileSplits`, `interruptedMigrationRetries`, `deletedPreferencesStayReset`, `corruptPreferencesRecoverWithOneNotice`, `isolatedLaunchesProtectRealConfig`, `legacyGpuDefaultSurvivesMigration`, `shiftSwitchDefaultsToOn`, `shiftSwitchPersistsInPrefs`, and `legacyShiftSwitchDefaultsToOn`. Register the suite and exercise both stores with independent paths.
- [x] 2.2 Add `tst_session_store` cases `sessionAndPreferencesRestoreIndependently`, `missingOrCorruptSessionKeepsPreferences`, `failedSessionWriteKeepsPreviousFile`, `legacyPanelLayoutDefaultsSurviveSplit`, and `columnRailModePersistsIndependently`. Update its existing writer-token cases to assert JSON state, including byte preservation on refusal and token adoption on a tokenless store. Add `unavailableGpuWritesOffToPreferences` and `safeModeKeepsStoredGpuPreference` to `tst_gpu_backend` using a controlled backend outcome.
- [x] 2.3 Register the preferences bridge and resolve common profile paths. Extend scoped Qt fixtures and self-test isolation to the config store and keep `--state-home` isolated. Verify the isolation and fresh-profile scenarios.
- [x] 2.4 Split known preference keys from the legacy JSON using an explicit map, commit TOML before JSON cleanup, and persist the completion marker. Preserve unknown JSON, panel version fallbacks, and the stale state write guard. Verify mixed migration, interruption, deletion, and state restart behavior.
- [x] 2.5 Route frame and feature consumers through typed preferences, preserve apply timing, and submit only explicit edits. Adapt fixtures that intentionally seed current-format settings. Verify preference relaunch, GPU fallback, locale, shortcut persistence, agent opt-in, Camera Raw choice, Type, and update preferences against the applicable test-plan suites.

- [x] 2.6 Add `tst_control_live::separateConfigRootIsHardDenied` and `tst_control_policy::separateConfigRootIsHardDenied`, covering canonical and symlink paths for open, save, and save_as in every approval mode.
- [x] 2.7 Protect the configuration root in both control paths and verify the hard-deny cases.

## 3. Portable settings backups

- [x] 3.1 Add `preferenceImportReceiptsSurviveSharedConfig` covering separate state roots sharing a config root and an old pending v1 file without an identity, `preferenceImportReceiptPreservesLaterEdits` covering v1 and v2, section failure and pending removal failure, and `invalidPreferencesLeavePendingImportIntact`, `legacyBackupAppliesPresentPreferencesOnly`, and `failedPreferencesImportRetries` to `tst_settings_backup`. Extend the existing export, staging, absent-section, and round-trip cases to inspect the TOML payload and include an unknown TOML value.
- [x] 3.2 Write version 2 backups and validate preference payloads using the store's rules. Translate present version 1 keys during staging or startup and use the common preferences write path and atomic import receipt for apply and retry. Verify new and legacy round-trips and the existing workspace and preset backup cases.

## 4. Startup reset

- [x] 4.1 Add and register `tst_preferences_reset` with `startupFlagCancelKeepsProfile`, `startupFlagConfirmRestoresFactoryProfile`, `resetRefusesBusyProfileOrWriteFailure`, `interruptedResetRecoversBeforeLoad`, and `concurrentInterruptedResetRecoverySerializes`. Test the flag through the pre-window startup path and use a second process for lock contention.
- [x] 4.2 Wire the startup chord and Wayland flag to one confirmation before stored settings load. Acquire presence locks and publish discoverable recovery records across both affected roots, serialize interrupted recovery exclusively before normal startup, retain recoverable backups of the targeted data and any pending import, and handle partial publication or interruption. Verify cancel, confirm, refusal, and restart recovery cases.

## 5. Integration verification

- [x] 5.1 Build with the repository's CPU-budget wrapper and run the Rust, Qt, and self-test gates. Run strict OpenSpec validation and the scenario coverage guard. Record the test plan's manual rows in the PR reviewer checklist. The local Qt run had one GPU paint mismatch, which also reproduced in a checkout without this change. CI handles verification of the archived branch.
- [x] 5.2 Verify the held-key startup path on a supported desktop and the flag on Wayland, recording the observed behavior. Tick the shipped #641 roadmap item, update affected repository memories, and archive the completed change after validation.
