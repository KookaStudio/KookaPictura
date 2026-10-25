# Spec Delta

## Purpose

Defines the engine-owned tool catalogue: the one ordered table of every tool's
identity, label, group, default shortcut, cursor and hints. The toolbox,
shortcuts, hint bar, search and control server read from it rather than from a
shell-only copy.

## ADDED Requirements

### Requirement: The engine owns the tool catalogue

The engine SHALL publish one ordered catalogue with one row per tool (71 rows).
Each row SHALL carry:

- the tool's stable id name (also its icon and cursor id stem `tool.<name>`);
- its CS6 label;
- its flyout group;
- its default single-letter shortcut, or none;
- its cursor shape and hotspot;
- its status hint;
- its modifier-key hints, each with its key, text and optional command id;
- whether it is implemented.

Id names SHALL be unique, and every member of one flyout group that has a
default letter SHALL share it. Some tools have no letter: Add, Delete and Convert
Anchor Point, and Blur, Sharpen and Smudge. The list of all tools and the
implemented set SHALL be derived from the catalogue, with no second hand-kept
list.

#### Scenario: Catalogue rows are unique and grouped [tool_catalogue_rows]

- **WHEN** the catalogue is enumerated
- **THEN** it has 71 rows with unique id names, and within each group every
  member that has a letter has the same one

#### Scenario: Implemented set derives from the catalogue [tool_catalogue_implemented]

- **WHEN** the implemented tools are listed
- **THEN** they are exactly the catalogue rows flagged implemented, in catalogue
  order

### Requirement: The shell tool table mirrors the catalogue order

The shell's tool enumeration SHALL have one value per catalogue row in catalogue
order. A mismatch in count, order or id name SHALL fail a test rather than
silently misroute a tool. The shell SHALL read every per-tool field it shows from
the catalogue. Its tool handlers, options bars, toolbox rendering and user
shortcut overrides SHALL stay in the shell.

#### Scenario: Enumeration and catalogue agree [tool_catalogue_order]

- **WHEN** each shell tool value is compared with the catalogue row at the same
  index, using an enumerator-to-name table kept independently of the catalogue
- **THEN** the counts are equal and every enumerator's name matches its row

#### Scenario: Toolbox, hints and shortcuts are unchanged [tool_catalogue_consumers]

- **WHEN** the toolbox, the tool hint bar, the Info panel tool row, the Keyboard
  Shortcuts dialog and the search bar list tools after the move
- **THEN** their labels, groups, flyout order, shortcuts, hints and cursors equal
  their previous output

#### Scenario: Self-test tool guards keep their exit codes [tool_catalogue_selftest]

- **WHEN** both headless self-test runs execute
- **THEN** the unimplemented-tool guard (98), `keys_shown` (116), `shift_plain`
  (117) and `healing_tools` (536, which also checks the picker Lab helper) pass with unchanged exit codes
