# Tasks

Implementation and test authoring are tracked below. The CMake build passed
with the 80% CPU budget. CI will run the automated checks. The licensed CS6
interoperability check is tracked in the PR reviewer checklist.

## 1. Format evidence and tests

- [x] 1.1 Record byte-layout sources, the exact chosen versions and mapped fields, and independent CS6 sample recipes for each Adobe variant, especially `.tpl` and `.csf`. Verify that each format in the codec spec has an evidenced implementation target and a generated fixture recipe.
- [x] 1.2 Write `preset_libraries_decode_supported_payloads`, `preset_libraries_report_partial_import`, and `preset_libraries_reject_invalid_input` in `pictura-codec`. Verify each Adobe preset adapter has fixture assertions for usable values and failure behavior.
- [x] 1.3 Write `preset_native_sets_round_trip`, `preset_swatches_round_trip_exchange`, and `csf_round_trips_supported_settings` in `pictura-codec`. Verify the tests assert native fidelity, swatch quantization bounds and `.csf` profile-reference handling.
- [x] 1.4 Add and register `tst_preset_manager`. Write `opensWithEightTypesAndViewModes`, `editsDuplicateNamesByIdentity`, `loadsReplacesAndResets`, `commitsOnlyAcceptedImports`, and `ignoresCancelledOrSupersededImport`. Verify coverage includes opening without a document, two-window synchronization, and cancelled or superseded operations.
- [x] 1.5 Write `savesSelectedSetsAtomically`, `migratesAndPersistsCurrentSets`, and `discoversLibrariesAcrossPreferenceReset` for the new Qt suite. Write `emptyNativeSetLoadAndReplace` and `migrationResetSurvivesRestart` for empty Save Set → Load and Replace, migrated swatches → preference reset → restart, and failed-reset-write cases. Verify they use isolated state and data directories and exercise both restart and failed-commit behavior.
- [x] 1.6 Write `appliesImportedSwatchBrushAndToolOptions`, `appliesImportedGradientStyleAndContour`, `appliesImportedPatternsAndCustomShape`, and `libraryEditsPreserveStrokeAndDocumentContent`. Verify each managed type has a consumer assertion and the open-stroke and reopened-document cases check results.

- [x] 1.7 Write `swatchPersistenceRegressions`, `currentPresetListsRoundTrip`, `currentPresetListsAbsent`, `currentPresetListsInvalid`, `currentPresetListsExportFailure`, and `currentPresetListsRetry` Qt tests for the modified backup scenarios, including current lists with empty and absent types, old backups, unreadable and oversized exports, failed staging and an interrupted apply retried before list load. Preserve the existing backup and swatch-persistence regression cases.

## 2. Codec and swatch vertical slice

- [x] 2.1 Add bounded type dispatch, diagnostics and the versioned native set envelope under `pictura-codec`. Verify native order, Unicode, empty sets, opaque records, and rejection of malformed framing with the codec cases in the test plan.
- [x] 2.2 Implement ACO and ASE import and supported export. Record the supported versions, color spaces, and conversion precision in the codec documentation and verify the swatch import and exchange tests.
- [x] 2.3 Add application-scoped current lists, default datasets, generation-based persistence and legacy swatch migration, with reset staging all defaults and atomically switching the active manifest so legacy data cannot reappear. Verify `migratesAndPersistsCurrentSets` and the existing swatch-panel regression tests.
- [x] 2.4 Add the command, Qt model and eight pages, view modes, list editing and shared Swatches panel actions. Register the shell sources explicitly and verify `opensWithEightTypesAndViewModes` and `editsDuplicateNamesByIdentity`.
- [x] 2.5 Add background import and export, the Load/Replace/Reset commands, discovery, preference reset integration and Save Set. Verify the library commands, stale import, atomic save and discovery tests.

- [x] 2.6 Extend portable settings export, pre-import backup, staged validation and startup apply to native current-list files, exporting effective lists for all eight types, including defaults and empty sets. Bound complete backups and report unreadable inputs; preserve older backups' absent-section semantics and stage a complete candidate generation and atomically switch its active manifest; report failures and load the prior generation, retrying before list load on a later startup. Verify the settings-backup tests.

## 3. Remaining adapters and consumers

- [x] 3.1 Implement `.grd` and `.shc` adapters and wire imported values through gradient and contour controls. Verify the codec payload cases and `appliesImportedGradientStyleAndContour` for their supported values.
- [x] 3.2 Implement `.pat` and `.asl` adapters and wire the common list into Pattern Stamp, Pattern Fill and Layer Style consumers. Apply style effects via the normal transaction and embed any used pattern with a document-local identity. Verify imported tile pixels, applied effect values and PSD reopen behavior.
- [x] 3.3 Implement ABR 10.1 sampled brush decoding and apply the decoded tip and mapped parameters through the existing picker and stroke path. Verify the imported brush affects coverage and a pending stroke keeps its captured sample.
- [x] 3.4 Implement `.csh` vector decoding and connect the Custom Shape picker to path geometry. Verify imported controls and bounds reach the resulting editable path. Keep the default-picker regressions green.
- [x] 3.5 Implement the supported `.tpl` variant and apply its supported tool settings through the existing options flow. Verify changing an imported tool preset changes the actual options and documents unsupported settings in import diagnostics.
- [x] 3.6 Implement bounded `.csf` read and write for the evidenced variant, exposing profile references and structured diagnostics to the future #569 caller. Verify `csf_round_trips_supported_settings` and record the codec's supported fields and writer limits. Keep the documented format matrix current as each adapter lands.

## 4. Integration evidence and shipping

- [x] 4.1 Build the application and Qt test targets with the repository's 80% CPU budget.
- [ ] 4.2 Run automated verification in CI, including the Rust and Qt test plans, the PSD pattern, ATN, brush, swatch and shape regressions, and the repository gates.
- [ ] 4.3 Run `preset_codec_cs6_oracle` on a licensed CS6 installation and record the exact variant and outcome for each supported direction.
- [x] 4.4 Update the roadmap, sync the capability specs, and archive the change with its test plan.
