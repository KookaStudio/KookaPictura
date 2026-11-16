# Spec Delta

## MODIFIED Requirements

### Requirement: A long operation runs as a cancellable job

An operation that mutates the document and that the application runs as a
cancellable job SHALL do its work on a worker thread while the GUI thread keeps
processing paints and timers. The wait cursor SHALL appear as soon as the job
starts. A progress window with a Cancel button SHALL appear only when the job
outlasts 250 ms. The window SHALL show the job's progress value (0 to 1000) and
SHALL NOT let user input reach the document until the job has landed. Cancel
and Esc SHALL both request cancellation. The wait cursor SHALL be restored when
the job lands. A non-mutating export batch SHALL be exempt from the worker-thread
requirement: it MAY run through the same modal progress window while pumping
the event loop, because it has no document state to roll back, provided the
wait cursor, delayed window, progress value, and Cancel/Esc behaviour still
apply.

#### Scenario: A quick job shows only the wait cursor [lop_quick_job]

- **WHEN** a cancellable job finishes in under 250 ms
- **THEN** the wait cursor was set while it ran, no progress window was shown, and the cursor is restored afterwards

#### Scenario: A slow job shows the progress window [lop_window_delay]

- **WHEN** a cancellable job is held in flight by its test seam for longer than 250 ms
- **THEN** the `jobProgress` window becomes visible with a Cancel button, and a repeating timer keeps firing while the job runs

#### Scenario: Esc cancels a job [lop_esc_cancels]

- **WHEN** Esc is pressed on a visible `jobProgress` window
- **THEN** the job's cancel callback runs, as it does for the Cancel button

#### Scenario: A non-mutating export reports progress without a worker [lop_export_pumped]

- **WHEN** a Zoomify or Export Layers batch runs through the modal progress window
- **THEN** the window reports progress and Cancel stops the batch between tiles or files, and the document is unchanged
