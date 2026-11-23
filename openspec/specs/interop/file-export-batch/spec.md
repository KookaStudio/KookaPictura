# interop/file-export-batch Specification

## Purpose
Defines the lifecycle shared by file-per-target export commands: validation and
preflight before any write, ordered one-file-at-a-time iteration, completed-item
progress, cancellation, view lifetime, and stop-on-failure reporting.

## Requirements

### Requirement: Destination and targets are validated before the first write

The export batch SHALL refuse a destination that does not exist or is not
writable before writing any file, SHALL write nothing and return failure when
the supplied target list is empty (an empty-list message is the command's own
and is not a write failure), and SHALL refuse the whole batch before any write
when a computed output path is an existing directory. A refusal SHALL report the
failing target when one is known. Validation and preflight SHALL complete before
the progress window appears.

#### Scenario: A bad destination writes nothing [feb_bad_destination]

- **WHEN** the destination is missing or not writable
- **THEN** no file is written, no target is attempted, and the export reports failure using the command's destination message

#### Scenario: An empty target list writes nothing [feb_empty_targets]

- **WHEN** the export runs with no targets
- **THEN** no file is written and the export returns failure; any empty-list message is the command's own and is not a write failure

#### Scenario: A directory at a computed path stops the batch before writes [feb_directory_target]

- **WHEN** one computed output path is an existing directory
- **THEN** no file is written, the export reports failure naming that target, and no target is attempted

### Requirement: Targets are written once each in supplied order

The batch SHALL attempt each target at most once, in the supplied order, writing
`destination/name+suffix` for that target. A file completed before a cancel or a
failure SHALL remain in place, and no target SHALL be attempted after a cancel or
a failure.

#### Scenario: Every target becomes one file in order [feb_order]

- **WHEN** the batch runs over three targets
- **THEN** exactly three files are written, one per target, in the supplied order

#### Scenario: Completed files survive a stop [feb_partial_retained]

- **WHEN** the batch stops by cancellation or a failed write after one file is completed
- **THEN** the completed file remains in the destination

#### Scenario: A stop attempts no later target [feb_no_later_attempt]

- **WHEN** a write for the second target fails
- **THEN** the first file remains and the third target is never attempted

### Requirement: Progress counts completed items

The batch SHALL derive its progress from the number of completed targets and the
total target count. Reported progress SHALL never decrease while the batch runs
and SHALL reach completion once every target is written.

#### Scenario: Progress only increases [feb_progress_monotonic]

- **WHEN** the batch writes several targets
- **THEN** the reported progress value is non-decreasing and each written file advances it

#### Scenario: A finished batch reports completion [feb_progress_complete]

- **WHEN** every target has been written
- **THEN** the reported progress is at its maximum

### Requirement: Cancellation is observed before the next write

A cancellation request SHALL stop the batch before the next target is attempted.
A cancelled batch SHALL leave completed files in place, SHALL attempt no further
target, and SHALL NOT be reported as a write failure.

#### Scenario: A cancel between files keeps what was written [feb_cancel_between_files]

- **WHEN** the batch is cancelled while one file has just been completed and more targets remain
- **THEN** exactly that one file exists, no later target is attempted, and the result is a cancel rather than a failure

### Requirement: A closed document aborts the batch

When the document being exported is closed while the batch is running, the batch
SHALL stop before the next write, SHALL leave completed files in place, SHALL
attempt no further target, and SHALL report failure naming the target that could
not be written.

#### Scenario: Closing the document stops the batch [feb_view_destroyed]

- **WHEN** the document is closed between two targets
- **THEN** no file is written for the next target, completed files remain, and the export reports failure naming that target

### Requirement: A failed write stops the batch and reports the target

When writing a target fails, the batch SHALL stop, SHALL leave earlier files in
place, SHALL attempt no later target, and SHALL report failure using the message
the exporting command supplies for that target.

#### Scenario: A failed write reports and stops [feb_write_failure]

- **WHEN** writing the second target fails
- **THEN** the batch reports the second target using the exporter's failure message, keeps the first file, and attempts no third target
