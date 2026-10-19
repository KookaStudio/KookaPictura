# Spec Delta

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
`panelRailMode` when a loaded store has no per-column rail mode. At schema
version 9 the system SHALL record the Tools column as an atomic per-column entry
with a `tools` marker and no groups. At schema version 10 the system SHALL add
`floating` and `floatGeometry` to each per-column entry and `floatGeometry` to
each floating group entry, covering widget columns, groups, and the Tools
column; a restored overlay's position and size SHALL be clamped to the available
screen or frame and to the overlay minimum. Overlays SHALL be placed after the
frame's stored window geometry is restored, and in-window child overlays SHALL
be placed after the first show so they are interpreted against the final frame
rather than the pre-restore window size. A store missing a field SHALL load
that field's documented default while preserving every field it already has, a
store with no per-column layout SHALL load a single right-hand column built from
the legacy per-group state, a store without the version-10 fields SHALL load
every panel docked, and unknown keys SHALL survive a load-then-write cycle. The
store SHALL be written on every quit path.

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

- **WHEN** a multi-column layout with floating groups and columns is saved and the
  store is reloaded
- **THEN** each column's side, order, width, rail mode, floating state, overlay
  geometry, and group state are restored

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

#### Scenario: A floating group is restored after restart [spr_floats_group]

- **WHEN** a panel group is floating, the application is quit, and it is
  relaunched
- **THEN** the group is floating again at its stored position and size

#### Scenario: A floating column is restored after restart [spr_floats_column]

- **WHEN** a whole widget column is floating, the application is quit, and it is
  relaunched
- **THEN** the column is floating again at its stored position and size

#### Scenario: A floating Tools column is restored after restart [spr_floats_tools]

- **WHEN** the Tools column is floating, the application is quit, and it is
  relaunched
- **THEN** the Tools column is floating again with its content-driven size at
  its stored position

#### Scenario: A store without float markers docks everything [spr_floats_legacy]

- **WHEN** a session store written before the floating layout fields is loaded
- **THEN** every panel and column opens docked and no overlay is created

#### Scenario: Restored overlay geometry is clamped [spr_floats_clamped]

- **WHEN** a stored overlay rectangle is larger than the available screen or
  frame, or smaller than the overlay minimum
- **THEN** the restored overlay is clamped to the available bounds and minimum
  before it is shown

#### Scenario: A restored overlay keeps its position in the restored frame [spr_floats_startup_position]

- **WHEN** a floating group or column is stored on the right of a frame whose
  window geometry is restored on launch, including a maximized frame
- **THEN** after relaunch the overlay is placed against the restored frame
  geometry and keeps its stored position instead of being clamped against the
  pre-restore window size
