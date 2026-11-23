# Tasks

## 1. Interface and test suite

- [x] 1.1 Declare `ExportTarget`, `ExportBatchSpec` (including `destinationFailureText` and `emptyInfoText`), `ExportBatchRun` (including `failed`), and both `exportBatch` overloads in `crates/pictura-app/cpp/export_batch.h`; declare `layersExportBatchSpec` in `export_layers_dialog.h` and `layerCompsExportBatchSpec` in `layer_comps_export.h` with compiling definitions in their `.cpp` files; add a compiling `export_batch.cpp` stub and register `export_batch.{h,cpp}` in `cmake/sources/documents.cmake`; verify `bash scripts/with-build-lock.sh -- cmake --build build` links the shell and `python3 scripts/check-cmake-source-fragments.py` passes.
- [x] 1.2 Add `crates/pictura-app/cpp/tests/tst_export_batch.cpp` with the nine slots named in `test-plan.md`, register it in `crates/pictura-app/cpp/tests/CMakeLists.txt`, and verify `bash scripts/with-build-lock.sh -- cmake --build build --target tst_export_batch` builds the suite (assertions fail until the module is implemented). Every case creates a `PicturaMainWindow` and a document and passes its `PictureView*`; failure and dialog cases pass `parent = nullptr` so no modal dialog opens, and wording is asserted from the spec fields.

## 2. Lifecycle implementation

- [x] 2.1 Implement destination validation and its `destinationFailureText`, target parsing and empty-list `emptyInfoText`, `destination/name+suffix` composition, and the directory preflight in `export_batch.cpp`; verify `tst_export_batch::badDestinationWritesNothing`, `emptyTargetsWriteNothing`, `directoryTargetWritesNothing`, and `writesTargetsInOrder` pass.
- [x] 2.2 Implement the `runCancellableJob` step loop with `ExportBatchRun` counters (`written`/`failed`), completed-item progress (`1000` when `total == 0`), cancel-before-next-write, `QPointer` view/parent lifetime, stop-on-failure, and `failureText` reporting; verify all nine `tst_export_batch` slots pass with `QT_QPA_PLATFORM=offscreen bash scripts/with-build-lock.sh -- ctest --test-dir build -R '^tst_export_batch$' --output-on-failure`. The suite asserts `run.cancelled && !run.failed` for a cancel, `run.failed` plus no next file for view destruction, and the comps `failureText` for the failed second target.

## 3. Export Layers adapter

- [x] 3.1 Replace `exportLayers`'s `BatchState` and step/cancel closures with a `layersExportBatchSpec` builder that supplies `export_layer_targets`, `exportLayersSuffix`, `export_layer_file`, and its existing wording, and call `exportBatch(<builder>(...))`; verify `tst_export_layers` passes, including `layersCancelStopsBatch` and `layersCloseDuringDelayAbortsSafely`.
- [x] 3.2 Assert the unchanged empty-target silence, destination message, and directory/write-failure wording through the real layers builder: `tst_export_batch::badDestinationWritesNothing` checks `destinationFailureText == "The destination folder is not writable."` and `failureText(sample) == "Could not write layer <key> to <path>."`; verify `tst_export_layers::layersRefusesBadDestination`, `layersDirectoryTargetStopsBatch`, `layersWriteFailureStopsBatch`, and `layersLeavesDocumentUnchanged` pass.

## 4. Layer Comps adapter

- [x] 4.1 Replace `exportLayerComps`'s `BatchState` and step/cancel closures with a `layerCompsExportBatchSpec` builder that supplies `layer_comps_api::exportTargets`, `layerCompsSuffix`, `layer_comps_api::exportFile`, and its existing wording, including `emptyInfoText = "This document has no layer comps."`, and call `exportBatch(<builder>(...))`; verify `tst_layer_comps_export` passes, including `directoryTargetStopsBatch`, `filesLeaveDocumentUnchanged`, and `exportLeavesHistoryAndDirtyUnchanged`.
- [x] 4.2 Verify through the comps builder that a document closed mid-batch sets `run.failed`, writes no file for the next target, and reports `failureText(nextTarget) == "Could not write <nextTarget.path>."` (no more `Could not write .`); `tst_export_batch::closedDocumentStopsBatch` replaces only `write` and exercises this; verify it passes.

## 5. Retire the duplicate protocols and covered skips

- [x] 5.1 Delete the `cancelStopsBatch` and `writeFailureStopsBatch` slots from `tst_layer_comps_export.cpp` (their shared behaviour is now `tst_export_batch` over the production comps builder), and verify no `BatchState` remains with `rg -n 'BatchState' crates/pictura-app/cpp`.
- [x] 5.2 Run the three suites together with `QT_QPA_PLATFORM=offscreen bash scripts/with-build-lock.sh -- ctest --test-dir build -R 'tst_(export_layers|layer_comps_export|export_batch)$' --output-on-failure` and verify every acceptance test named in the issue passes.

## 6. Integration and archive

- [x] 6.1 Run `python3 scripts/check-test-plan.py` and `openspec validate export-batch-lifecycle --strict`, and verify both pass.
- [x] 6.2 Run `openspec archive export-batch-lifecycle`, then `openspec validate --all --strict`, and verify `openspec/specs/interop/file-export-batch/spec.md` exists and no change remains under `openspec/changes/`.
- [ ] 6.3 Run `bash scripts/verify-fast.sh` and verify it passes; confirm `ROADMAP.md` and `PARITY.md` need no edit because no feature coverage moved.
