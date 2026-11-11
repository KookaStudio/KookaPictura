# document/long-operation-progress Specification

## Purpose
Keeps the window responsive while a long operation runs: the work moves to a
worker thread behind a delayed progress window with Cancel, the engine reports
progress and honours cancellation at row boundaries, and a cancel leaves the
document and history exactly as they were.

## Requirements

### Requirement: A long operation runs as a cancellable job

An operation that the application runs as a cancellable job SHALL do its work on
a worker thread while the GUI thread keeps processing paints and timers. The wait
cursor SHALL appear as soon as the job starts. A progress window with a Cancel
button SHALL appear only when the job outlasts 250 ms. The window SHALL show the
job's progress value (0 to 1000) and SHALL NOT let user input reach the
document until the job has landed. Cancel and Esc SHALL both request
cancellation. The wait cursor SHALL be restored when the job lands.

#### Scenario: A quick job shows only the wait cursor [lop_quick_job]

- **WHEN** a cancellable job finishes in under 250 ms
- **THEN** the wait cursor was set while it ran, no progress window was shown, and the cursor is restored afterwards

#### Scenario: A slow job shows the progress window [lop_window_delay]

- **WHEN** a cancellable job is held in flight by its test seam for longer than 250 ms
- **THEN** the `jobProgress` window becomes visible with a Cancel button, and a repeating timer keeps firing while the job runs

#### Scenario: Esc cancels a job [lop_esc_cancels]

- **WHEN** Esc is pressed on a visible `jobProgress` window
- **THEN** the job's cancel callback runs, as it does for the Cancel button

### Requirement: Engine work reports progress and honours cancellation

An engine operation that runs under a job token SHALL check the token's cancel
flag at row or tile boundaries and SHALL stop when the flag is set. A cancelled
operation SHALL report that it was cancelled and SHALL leave its document
unchanged. The token's progress value SHALL never decrease while the operation
runs, and SHALL reach 1000 when the operation completes.

#### Scenario: Progress rises to completion [lop_progress_monotonic]

- **WHEN** a layer transform runs under a job token and a sampler records the progress value after every row
- **THEN** the recorded values never decrease and the last one is 1000

#### Scenario: A cancelled engine operation changes nothing [lop_engine_cancel]

- **WHEN** a layer transform runs under a job token whose cancel flag is already set
- **THEN** the transform reports that it was cancelled, and the document's layer pixels and rect are identical to the input

### Requirement: Cancel leaves the document and history unchanged

When a cancellable job that changes the document is cancelled, the document
SHALL be pixel-identical to its state before the job started, and the history
SHALL have the same states and current index. A job that completes SHALL record
the same history states that the synchronous operation records.

#### Scenario: A cancelled job adds no history state [lop_cancel_unchanged]

- **WHEN** a document-changing job is cancelled while in flight
- **THEN** the composite is pixel-identical to the one before the job, and the history count and index are unchanged

#### Scenario: A large document stays responsive [lop_large_document]

- **WHEN** a long operation runs on a 17000 × 17000 document
- **THEN** the canvas keeps repainting, the progress bar advances, and Cancel returns control within a second
