# Test plan — export-batch-lifecycle

Every scenario is automated in a new offscreen Qt Test suite, `tst_export_batch`.
Every case creates a `PicturaMainWindow` and a document and passes its
`PictureView*` (fake writers ignore it), because the module only calls `write`
while the view guard is alive. Failure and dialog cases pass `parent = nullptr`,
so no modal dialog opens and wording is asserted from the spec fields. The
destination case builds the production layers spec; the cancellation,
view-destruction, and write-failure cases build the production comps spec with
`layerCompsExportBatchSpec`, and each replaces only `write` where the test must
cancel or fail. The recording writer creates real files for successful targets
(and only the first in the failure case). No sleeps or timer races are involved.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `feb_bad_destination` | Build the real layers spec with `layersExportBatchSpec` over a two-target document and a missing destination; replace `write` with a counter | False; zero writes; `spec.destinationFailureText == "The destination folder is not writable."`; `spec.failureText(sample) == "Could not write layer <key> to <path>."` | `tst_export_batch::badDestinationWritesNothing` |
| `feb_empty_targets` | Run with empty target text | False; writer never called; no file; empty-list message is the command's own, not a write failure | `tst_export_batch::emptyTargetsWriteNothing` |
| `feb_directory_target` | Two targets; the second computed path is an existing directory | False; no file written at all; failure names the second target | `tst_export_batch::directoryTargetWritesNothing` |
| `feb_order` | Three targets with distinct names | Writer receives `destination/<name><suffix>` in order; three files exist | `tst_export_batch::writesTargetsInOrder` |
| `feb_partial_retained` | Cancel after the first comp; separately fail the second fake write | The first file still exists in both runs | `tst_export_batch::cancelBetweenFiles`, `tst_export_batch::writeFailureStopsBatch` |
| `feb_no_later_attempt` | Three comps; build the production comps spec and replace `write` to write target 1 and fail target 2 | Writer called exactly twice; third never attempted; one file remains | `tst_export_batch::writeFailureStopsBatch` |
| `feb_progress_monotonic` | Sample `progressPermille()` before each of four writes | Values are non-decreasing and rise per completed file | `tst_export_batch::progressOnlyIncreases` |
| `feb_progress_complete` | Complete a three-target batch | `written == total`, progress is maximum, result true | `tst_export_batch::finishedBatchReportsCompletion` |
| `feb_cancel_between_files` | Two comps; build the production comps spec and cancel from `write` after the first file | Exactly one comp file; second not attempted; `run.cancelled && !run.failed` | `tst_export_batch::cancelBetweenFiles` |
| `feb_view_destroyed` | Two comps; build the production comps spec and close the document from `write` after the first file | Next target not written; first file remains; `run.failed`; `spec.failureText(nextTarget) == "Could not write <nextTarget.path>."` | `tst_export_batch::closedDocumentStopsBatch` |
| `feb_write_failure` | Three comps; build the production comps spec and replace `write` with a fake that writes target 1, returns false on target 2, and records target 3 | One file remains; exactly two write calls; third never attempted; `spec.failureText(secondTarget) == "Could not write <secondTarget.path>."` | `tst_export_batch::writeFailureStopsBatch` |
