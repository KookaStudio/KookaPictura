# Spec Delta

## MODIFIED Requirements

### Requirement: One transaction commits every document edit

The app SHALL commit every document edit through a single edit-transaction interface that takes a history label and a mutation. The mutation SHALL run inside the transaction, so that one transaction owns the document change and its history state: a mutation that reports unchanged SHALL commit no history state. A mutation that refuses MUST detect the refusal before it writes, so the refused edit leaves the document unchanged and the transaction captures no history state for it. The mutation SHALL declare one outcome:

- unchanged;
- metadata-only or otherwise non-compositing (selection, name, lock, colour label, channel list, guides, annotations, slices, file metadata, or resolution);
- already-presented (a preview already presented the change);
- one or more bounded document rectangles;
- the whole document;
- a link-invalidating change.

The mutation SHALL report the outcome of the change it made. The transaction SHALL derive the compositing, linkage, history, revision, and dirty work from that outcome and SHALL return the outcome to the caller. Command code MUST NOT hand-write the refresh-then-capture sequence.

#### Scenario: A committed edit is refreshed and captured once [et_one_owner]

- **WHEN** a command commits a document change through the transaction
- **THEN** the declared compositing runs, exactly one history state is captured, and the document is left dirty with its content revision advanced

#### Scenario: The outcome is reported to the caller [et_reports_outcome]

- **WHEN** the mutation returns an outcome
- **THEN** the transaction returns the same outcome so the command can report whether and how the document changed

#### Scenario: A failed mutation leaves no change and no history [et_mutation_inside]

- **WHEN** a mutation runs inside the transaction and refuses before writing, or changes nothing
- **THEN** the document is unchanged, no history state is captured, and no change signal is emitted

## ADDED Requirements

### Requirement: A committed edit presents the change once

A committed edit whose outcome notifies SHALL present the change to the shell exactly once. An outcome that does not notify — a bounded region without notification, or an already-presented commit — SHALL present nothing from the transaction. A command handler MUST NOT present a document edit again when every successful outcome its command can commit notifies. A command handler whose command can commit a successful outcome that does not notify SHALL keep its own presentation. A command that changes only metadata or only the selection SHALL present its change, so the history state and the command table reflect it without the caller asking for a refresh.

#### Scenario: A committed edit notifies once [et_notify_once]

- **WHEN** a command commits a document edit whose outcome notifies
- **THEN** the shell is notified exactly once, and no second notification follows it for that edit

#### Scenario: A handler presents nothing after a presented edit [et_no_double_present]

- **WHEN** a menu command commits a document edit that the transaction has already presented
- **THEN** the command handler presents nothing further, and the window is refreshed once for that edit

#### Scenario: A metadata edit presents its change [et_meta_presented]

- **WHEN** a command commits a metadata-only edit, such as saving a selection into a channel
- **THEN** the shell is notified, so the history state and the command table reflect the change without the caller asking for a refresh
