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

### Requirement: Whole-document commands run as cancellable jobs

An interactive Image Size, Canvas Size, Rotate, Flip, mode conversion, bit-depth
or HDR conversion, Merge Down, Merge Layers, Merge Visible, Merge Clipping Mask,
or Flatten Image on a document of 4 MP or more SHALL run its engine on a worker
as a cancellable job, while the document and history stay unchanged until the
job lands. A completed job SHALL record the same single history state as the
synchronous command. A cancelled job SHALL leave the document pixel-identical
and the history count and index unchanged. A command on a smaller document, or
one not issued from the menus or panels, SHALL apply at once.

#### Scenario: A large rotation runs as a job [lop_whole_image_job]

- **WHEN** Image > Image Rotation > 90° CW runs on a 2400 × 1800 document whose job is held by the test seam
- **THEN** the progress window appears, and on landing the document is 1800 × 2400 with one added `"Rotate"` history state

#### Scenario: A cancelled whole-document command changes nothing [lop_whole_image_cancel]

- **WHEN** Flatten Image runs on a large two-layer document and Cancel is pressed while its job is held
- **THEN** the layer count, the history count and index, and the displayed pixels are unchanged, and the command applied afterwards adds one state

#### Scenario: A merge job reports the merged layer [lop_merge_job]

- **WHEN** a Merge Down job lands
- **THEN** the merged layer is the active target and the reported outcome names its path and the two layers it replaced

#### Scenario: A small document applies at once [lop_small_sync]

- **WHEN** a rotation runs on a 64 × 48 document
- **THEN** no progress window appears, the job delay seam is never reached, and the rotation has applied when the command returns

### Requirement: A job reports the progress its engine can count

A whole-document job SHALL report real progress when its engine can count its
work: resampled rows for Image Size, layers for Rotate and Flip, and composited
rows for Merge and Flatten. A job whose engine cannot weigh its passes (Canvas
Size and the mode, depth and HDR conversions) SHALL show a busy bar. A counted
engine SHALL check the cancel flag at each row, layer or band, and a cancelled
engine SHALL report that it was cancelled.

#### Scenario: Counted engines report every unit [lop_counted_progress]

- **WHEN** Image Size, Rotate, Flip, Merge Down and Flatten each run under a job token on a three-layer document
- **THEN** each reaches the last value before completion (999), so every declared unit was reported

#### Scenario: A job that cannot count shows a busy bar [lop_busy_bar]

- **WHEN** a cancellable job reports a negative progress value, as an 8-to-16-bit conversion on a large document does
- **THEN** the visible progress window shows a busy bar and never a percentage

#### Scenario: A cancelled engine stops early [lop_engine_stops]

- **WHEN** Image Size, Rotate and Flatten run under a job token whose cancel flag is already set
- **THEN** each reports that it was cancelled, without resampling a row, rotating a layer, or compositing a band
