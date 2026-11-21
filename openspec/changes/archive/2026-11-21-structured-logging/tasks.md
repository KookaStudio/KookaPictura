# Tasks

## 1. Write the acceptance tests

- [x] 1.1 Add and register `tst_structured_logging` with `routesRustAndQtRecords`, `appliesTargetAndCategoryRules`, `keepsPrivateFieldsOutOfFile`, `isolatesStartupModes`, and `flushesAndPreservesFatalExit`. Verify the CTest registration and scenario names against the test plan.
- [x] 1.2 Add `pictura-app::diagnostic_rotation_keeps_seven_dates` and `pictura-app::diagnostic_writer_failure_and_saturation`. Verify the fixtures use a controlled clock and injected failure and stall conditions.
- [x] 1.3 Add and register `tst_history_log` with `preferencesPersistAndCancelCleanly`, `disablingStopsNewRecords`, `verbosityFollowsCommittedActivity`, `capturesOnlyCommittedEdits`, `routesSelectedDestinations`, `failedTextSinkKeepsMetadata`, and `reportsTextQueueAndDrainsOnQuit`. Verify the cases exercise app save and edit entry points and assert on the actual outputs.
- [x] 1.4 Add `reopensHistoryAcrossFormats`, `saveSnapshotsKeepPendingRecords`, `metadataAppendFailureKeepsImageSaveable`, `fileInfoEditsDuringSave`, `webExportHonorsMetadataChoice`, and `webJpegOverflowKeepsSourceRecords` to `tst_history_log`, plus `pictura-codec::standard_raster_xmp_round_trips`. Verify the test-plan guard resolves every planned test.

## 2. Implement application diagnostics

- [x] 2.1 Add the tracing dependencies and the app-owned subscriber, bounded file writer, rotation, fallback, and two-second shutdown drain with stderr-only timeout reporting. Update dependency notices and verify the license gate, retention, concurrent output, and sink-failure cases.
- [x] 2.2 Wire diagnostic startup and teardown through the resolved mode and state root. Add Qt severity, category, and fatal handling through the Rust bridge. Verify startup isolation, protocol output, target/category filters, and shutdown with `tst_structured_logging`.
- [x] 2.3 Emit typed open, save, and GPU outcome events and capture dependency diagnostic metadata. Verify shared output and field redaction with `tst_structured_logging`.

## 3. Implement History Log capture

- [x] 3.1 Add General preference controls, destination validation, and session persistence. Verify defaults, chooser cancellation, and restored settings with `preferencesPersistAndCancelCleanly`.
- [x] 3.2 Capture lifecycle and committed edit events outside undo snapshots. Pass supported parameters with the correct successful command and document. Verify verbosity, disabled capture, and no entries from refused or canceled edits with `tst_history_log`.
- [x] 3.3 Append independently to the chosen text destination. Report every rejected, failed, or undrained record with bounded batched notices and drain close and quit rows in order with a two-second deadline. Verify routing, continued metadata saves after a text failure, and the queue and shutdown behavior in `reportsTextQueueAndDrainsOnQuit`.

## 4. Implement History Log file persistence

- [x] 4.1 Merge bounded history text into `photoshop:History` and expose its multiline value in File Info. Import bounded JPEG APP1 and TIFF tag 700 XMP while preserving other resources. Verify codec fixtures and the four-format File Info round trip.
- [x] 4.2 Stage history for native and supported raster saves, preflight packet limits, and acknowledge only the included records on successful publication of the source. Rebase saved rows after File Info edits with equal History and honor effective History changes, including change then restoration. Verify concurrent, failed, copy, oversized, and unsupported-packet saves with `tst_history_log`.
- [x] 4.3 Stage History Log XMP for Save for Web and pass it through the existing metadata filter. Apply the encodable-packet fallback and warning to ordinary and sliced full-XMP JPEG overflow. Verify all metadata levels for JPEG and both PNG variants, both JPEG export paths at the size boundary, and the later source save.

## 5. Verify the integrated change

- [x] 5.1 Update the #643 roadmap entry and verify that it matches the shipped scope. After all code and tests are written, build using the repository CPU-budget wrapper, run the suites in the test plan, and validate the change. Include the manual CS6 check in the reviewer checklist.

After the implementation tasks are complete, archive the change and run the
repository verification gates on the archived tree.
