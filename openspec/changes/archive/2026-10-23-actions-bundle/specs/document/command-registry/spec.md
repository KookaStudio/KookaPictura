# Spec Delta

## MODIFIED Requirements

### Requirement: Declarative command table
The system SHALL define every menu command once in a declarative table authored in
Rust, keyed by a stable string identifier, carrying the menu path, label, default
shortcut, implemented flag, and checkable flag. The C++ shell SHALL consume that
catalogue and SHALL NOT maintain a second hand-authored command table or a second
hand-authored frozen identifier list. The menu bar SHALL be built from this table rather than
from hand-assembled `QAction`s. The frozen command identifiers SHALL be generated
from the Rust catalogue, and a drift check SHALL fail when the generated C++
header differs from the catalogue.

#### Scenario: Menu bar is generated from the table
- **WHEN** the application frame is constructed
- **THEN** each top-level menu and child item corresponds to an entry in the command table

#### Scenario: Command identifiers are stable
- **WHEN** a command's label or shortcut is changed in the table
- **THEN** its identifier is unchanged and dispatch by identifier still resolves it

#### Scenario: No second C++ command table exists [cr_rust_only]
- **WHEN** the repository is inspected
- **THEN** the command entries and frozen identifiers exist only in the Rust catalogue and its generated C++ header, not in a hand-authored C++ table

#### Scenario: Frozen identifier header matches [cr_id_drift]
- **WHEN** the Rust catalogue and the generated C++ identifier header are compared
- **THEN** every identifier name and value matches, and a mismatch fails the check

#### Scenario: Inspection output is unchanged [cr_list_commands]
- **WHEN** the control server's `list_commands` method is called before and after the catalogue move
- **THEN** the command id, path, label, implemented, enabled, and checked values are unchanged

## ADDED Requirements

### Requirement: Command recording and replay hooks

The command layer SHALL provide one execution path that can report an executed
command and its parameter values to a recording sink while recording is active,
and SHALL execute a recorded command with supplied parameter values without
opening that command's normal UI. The recording hook SHALL run only after the
command reports success; a `void()` handler that does not report success SHALL
NOT be recorded. Dispatching a recordable parameterless command that reports
success SHALL be recordable as a step with no parameters. A recorded command with
no registered executor SHALL be refused with a named diagnostic rather than
silently skipped. The control server's `dispatch_command` path SHALL use the same
recording and execution hooks as a menu dispatch.

#### Scenario: Dispatch records while recording [cr_record_dispatch]

- **WHEN** a recordable command reports success while recording is active
- **THEN** the recording sink receives the command identity and its parameter values

#### Scenario: A failed or refused command is not recorded [cr_record_refused]

- **WHEN** a recordable command handler reports failure or is refused
- **THEN** no recording event reaches the sink

#### Scenario: Recorded command replays without UI [cr_replay_executor]
- **WHEN** a recorded command is played and an executor is registered for it
- **THEN** the executor runs with the recorded parameters and no dialog opens unless the step's dialog state is on

#### Scenario: Missing executor is named [cr_replay_missing]
- **WHEN** a recorded command has no registered executor
- **THEN** replay is refused with a diagnostic naming the command and the step

#### Scenario: Control-server dispatch shares the path [cr_control_dispatch]
- **WHEN** the control server dispatches a recordable command while recording is active
- **THEN** the same recording sink receives the command and its parameters
