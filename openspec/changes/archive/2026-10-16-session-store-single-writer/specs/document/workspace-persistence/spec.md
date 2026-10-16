# workspace-persistence Specification

## MODIFIED Requirements

### Requirement: Session state store

The system SHALL persist the opaque dock layout, the interface brightness level,
and the panel-column state to `$XDG_STATE_HOME/kooka-pictura/state.json`, writing
atomically through a temporary file so a failed write never leaves a torn store.
The store SHALL carry a schema version, and at schema version 5 SHALL add
`panelRailMode`, `railWidth`, `autoCollapseIconic`, `autoShowHidden`, and the
per-group collapsed, minimized, order, and visibility state. At schema version 6
the system SHALL persist the per-column layout (each column's side and order
with its groups' order, visibility, minimized, and collapsed state nested per
column). At schema version 7 the system SHALL add a width to each per-column layout
entry, and SHALL seed a column's width from the legacy top-level `railWidth` when
a loaded store has no per-column width. At schema version 8 the system SHALL add a
`railMode` to each per-column layout entry, SHALL restore each column's own rail
mode, and SHALL seed a column's rail mode from the legacy top-level
`panelRailMode` when a loaded store has no per-column rail mode. A store that is
missing a field or older than the current version SHALL load the defaults, a store
with no per-column layout SHALL load a single right-hand column built from the
legacy per-group state, and unknown keys SHALL survive a load-then-write cycle.
The store SHALL be written on every quit path.

Each write SHALL stamp the store with a per-launch writer token and write time,
and a write SHALL be refused when the store's token belongs to another launch
and its write time is not older than this launch's start, so a stale instance
cannot overwrite a store a newer instance wrote. A refused frame save SHALL also
leave the active-workspace snapshot unwritten. A store without a writer token
SHALL be adopted and rewritten with this launch's token.

#### Scenario: Session state is restored on launch

- **WHEN** the application starts and a valid session state exists
- **THEN** the saved layout, brightness, and panel-column state are applied

#### Scenario: Missing session state

- **WHEN** no session state file exists
- **THEN** the application starts with the default layout and default brightness

#### Scenario: Corrupt session state

- **WHEN** the session state file cannot be parsed
- **THEN** the application starts with defaults and does not crash

#### Scenario: Atomic write

- **WHEN** the session state is saved
- **THEN** the store is written to a temporary file and renamed over the previous store

#### Scenario: An older store loads the panel-column defaults [m41_session]

- **WHEN** a schema-4 store with none of the version-5 fields is loaded
- **THEN** the panel-column mode is `normal`, auto-collapse and auto-show are
  off, and the existing keys are preserved

#### Scenario: The per-column layout round-trips [m43_session]

- **WHEN** a multi-column layout is saved and the store is reloaded
- **THEN** each column's side and order plus its groups' order, visibility,
  minimized state, collapsed state, and rail mode are restored

#### Scenario: A version-5 store loads a single right-hand column [m43_session]

- **WHEN** a schema-5 store with a top-level per-group state and no per-column
  layout is loaded
- **THEN** the workspace shows one right-hand column containing those groups and
  the existing keys are preserved

#### Scenario: A version-6 store loads a default column width [lpr_v7]

- **WHEN** a schema-6 store whose per-column entries carry no width is loaded
- **THEN** each column loads the default width and the legacy top-level
  `railWidth` seeds the primary column's width

#### Scenario: A version-7 store seeds the per-column rail mode [lpr_v8_railmode]

- **WHEN** a schema-7 store whose per-column entries carry no `railMode` is loaded
- **THEN** each column loads the legacy top-level `panelRailMode` as its mode

#### Scenario: A per-column rail mode is applied after restart [lpr_column_mode]

- **WHEN** a non-primary column's `normal`/`iconic` mode is changed, the
  application is quit, and it is relaunched
- **THEN** each column's mode after launch equals its stored mode, measured after
  the first layout

#### Scenario: A newer store is not overwritten [spr_stale_refused]

- **WHEN** a second launch writes the store after a first launch started, and
  the first launch then saves
- **THEN** the first launch's save is refused and the second launch's store is
  byte-identical afterwards

#### Scenario: A store from before the launch is adopted [spr_stale_adopted]

- **WHEN** the store's writer token is another launch's but its write time
  predates this launch's start
- **THEN** the save succeeds and the store carries this launch's token

#### Scenario: A store without a token is adopted [spr_legacy_adopted]

- **WHEN** the store carries no writer token
- **THEN** the save succeeds and stamps the store with this launch's token

## ADDED Requirements

### Requirement: Self-test isolation of the session store

A self-test run SHALL redirect `XDG_STATE_HOME` to a temporary directory before
the frame is constructed and SHALL abort with a non-zero exit code when that
directory cannot be created, rather than falling back to the user's real store.
The self-test SHALL NOT leave its temporary state directory behind.

#### Scenario: A self-test run writes only its temp store [spr_selftest_isolated]

- **WHEN** the application runs `--headless --self-test`
- **THEN** the session store is written under the temporary state directory and
  the user's store is untouched

#### Scenario: A temp-dir failure aborts [spr_selftest_no_fallback]

- **WHEN** the self-test's temporary state directory cannot be created
- **THEN** the run fails with a non-zero exit code instead of writing the real
  store

#### Scenario: No temp store is left behind [spr_selftest_clean]

- **WHEN** a self-test run exits
- **THEN** its temporary state directory has been removed
