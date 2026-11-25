# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Help exposes a Debug Toolset toggle`
- TO: `### Requirement: Help exposes a Pictura Metrics toggle`

## MODIFIED Requirements

### Requirement: Help exposes a Pictura Metrics toggle

Help ▸ Developer SHALL contain a checkable `Pictura Metrics` command, enabled with
no document open. The toolset SHALL be off at every launch, and its state and the
panel position SHALL NOT be persisted.

#### Scenario: The command is checkable and off at startup [dbg_menu_entry]

- **WHEN** the main window is constructed with no document
- **THEN** `help.developer.debugToolset` is enabled, checkable and unchecked, its menu
  text reads `Pictura Metrics`, and no debug panel is visible

#### Scenario: State is not persisted [dbg_not_persisted]

- **WHEN** the toolset is enabled, the panel is moved, and a new main window is
  constructed
- **THEN** the new window has the toolset off and writes no setting for it
