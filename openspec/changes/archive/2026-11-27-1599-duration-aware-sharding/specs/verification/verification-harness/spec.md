# Spec Delta

## MODIFIED Requirements

### Requirement: Non-goal guard
The system SHALL provide `scripts/guard.sh` that fails when tracked source under
`crates/` references `.8bf` plugin binaries, when an `artboard*` file appears
under `crates/`, or when `docs/` is modified without a `TASK-ALLOWS-DOCS` marker,
and SHALL exit 0 when none of those conditions hold. It SHALL also run
`scripts/check-qt-test-waits.py`, so a non-zero fixed wait in the Qt Test sources
without a reason comment fails the guard on a C++-only change.

#### Scenario: .8bf reference under crates/
- **WHEN** a tracked `crates/` source contains an `.8bf` reference
- **THEN** the guard exits non-zero and prints the offending lines

#### Scenario: artboard artifact under crates/
- **WHEN** a file named `artboard*` exists under `crates/`
- **THEN** the guard exits non-zero and names the file

#### Scenario: docs modified without a marker
- **WHEN** `docs/` differs from the guard base and no commit carries `TASK-ALLOWS-DOCS`
- **THEN** the guard exits non-zero and lists the changed files

#### Scenario: an unjustified fixed wait fails the guard [vh_wait_guard]
- **WHEN** a Qt Test source holds a non-zero `qWait` call with no reason comment on its line or the line above
- **THEN** the guard exits non-zero and names the file and line

#### Scenario: Clean tree
- **WHEN** no non-goal artifact is present, docs are unchanged or allowed, and every fixed wait is justified
- **THEN** the guard exits 0
