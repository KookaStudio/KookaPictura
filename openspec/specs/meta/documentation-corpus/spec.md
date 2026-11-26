# meta/documentation-corpus Specification

## Purpose
Binds the user-facing documentation to the application that ships: the README
and the roadmap describe only what the code backs, every claim is checkable
against a path in the tree, and every remaining gap is an open item that names
the issue tracking it.

## Requirements

### Requirement: The roadmap claims only what ships

`ROADMAP.md` SHALL mark a feature as shipped only when the tree implements it,
and every shipped feature SHALL be marked as shipped. Each shipped entry SHALL
name a path in the tree that backs the claim. The roadmap SHALL NOT use a
phase number as a substitute for a status.

#### Scenario: A shipped entry names a path that exists [dc_roadmap_shipped]

- **WHEN** a reader follows a shipped entry in `ROADMAP.md` to the path it names
- **THEN** the path exists in the tree and the code at that path implements the
  claimed feature

#### Scenario: An open entry names the issue that tracks it [dc_roadmap_open]

- **WHEN** a reader reads an unticked entry in `ROADMAP.md`
- **THEN** the entry names at least one GitHub issue, and that issue is tracked
  in the gap register at `docs/dev/doc-code-parity-gaps.md`

### Requirement: The README describes the application that ships

Every feature bullet in `README.md` SHALL name a path in the tree that backs
the claim. The README SHALL NOT claim a feature the tree does not implement,
and where a family is only partly implemented the bullet SHALL say so rather
than name the family whole.

#### Scenario: A feature bullet names a path that exists [dc_readme_bullet]

- **WHEN** a reader follows a feature bullet in `README.md` to the path it names
- **THEN** the path exists in the tree and the code at that path implements the
  claimed feature

### Requirement: The specification corpus does not deny the implementation

No file under `docs/` SHALL assert that the repository contains no source code,
no build files, or no dependencies, while the repository holds the
implementation the corpus specifies. A corpus file's status banner SHALL name
`docs/dev/doc-code-parity-gaps.md` as the place that records what shipped and
what did not. The corpus entry point SHALL describe itself as the behavioural
contract for the project and SHALL name where the implementation lives.

#### Scenario: A corpus file's banner points at the gap register [dc_corpus_banner]

- **WHEN** a reader reads the status banner of a file under `docs/`
- **THEN** the banner does not claim the repository has no code, and it names
  `docs/dev/doc-code-parity-gaps.md`

#### Scenario: The corpus entry point names the implementation [dc_corpus_entry]

- **WHEN** a reader opens `docs/README.md`
- **THEN** it describes the corpus as the behavioural contract for the project
  and names the directory holding the implementation, rather than claiming the
  repository has no code, no build files, and no dependencies

### Requirement: The gap register is the single index of what is missing

`docs/dev/doc-code-parity-gaps.md` SHALL remain the canonical register of the
gaps between the corpus and the code, carrying for each gap a status, an
evidence path, and the GitHub issue tracking it. A row whose evidence the tree
contradicts SHALL be corrected in the register rather than left to mislead the
roadmap that points at it.

#### Scenario: A roadmap item resolves to a register row [dc_register_index]

- **WHEN** a reader takes an unticked entry in `ROADMAP.md` and follows its
  issue into `docs/dev/doc-code-parity-gaps.md`
- **THEN** the issue appears in the register's Issue column, and the status the
  register records for that gap agrees with what the roadmap says about it

### Requirement: The roadmap and the corpus agree on what is not planned

A feature that the tree implements SHALL NOT be listed as a non-goal. A feature
the tree does not implement and that is not tracked by an issue SHALL be
listed as a non-goal with the reason, and a feature the tree does not
implement that is tracked by an issue SHALL be listed as open rather than as a
non-goal.

#### Scenario: A shipped feature is not a non-goal [dc_nongoal_shipped]

- **WHEN** a reader takes a feature listed under "Not planned" in `ROADMAP.md`
  and searches the tree for its implementation
- **THEN** no implementation contradicts the non-goal

#### Scenario: An absent non-goal is deliberate [dc_nongoal_absent]

- **WHEN** a reader takes a feature listed under "Not planned" in `ROADMAP.md`
  that the gap register records as a confirmed non-goal
- **THEN** the register agrees, and the feature's absence is deliberate rather
  than a gap that no issue tracks

### Requirement: The user guide is a complete menu reference

`public_docs/` SHALL contain one reference page per top-level menu (File, Edit,
Image, Layer, Type, Select, Filter, View, Window, Help), linked from a
**Menus** section of `public_docs/README.md`. Every command in the command
catalogue that has a menu path SHALL appear on its top-level menu's page with
the label and shortcut the running app shows. Commands the running app does not
implement SHALL be listed on the same page as not yet available. Commands that
only the Extended edition offers SHALL be marked Extended.

#### Scenario: Every menu command is documented [dc_menu_reference]

- **WHEN** a reader takes any command id with a menu path from the command
  catalogue
- **THEN** the top-level menu page in `public_docs/` names that command, and
  the page carries the id in its hidden command manifest

#### Scenario: A menu command shows its shortcut [dc_menu_shortcuts]

- **WHEN** a reader looks up a command that has a keyboard shortcut
- **THEN** the page shows that shortcut beside the command

#### Scenario: A disabled command is marked [dc_disabled_commands]

- **WHEN** a reader looks up a command the running app greys out
- **THEN** the page lists it under **Not yet available** rather than omitting it

### Requirement: The user guide covers Preferences and the status bar

`public_docs/` SHALL contain a Preferences reference that describes every page
of **Edit ▸ Preferences** — General, Interface, File Handling, Performance,
Cursors, Transparency & Gamut, Units & Rulers, Guides, Grid, & Slices,
Plug-ins, Type, 3D, Agent Control and Updates — and marks the pages that are
listed but disabled as not yet available. It SHALL contain a Status bar
reference that describes the readout menu (Document Sizes, Document
Dimensions, Measurement Scale, Scratch Sizes, Efficiency) and what each number
means.

#### Scenario: Every Preferences page is described [dc_preferences_reference]

- **WHEN** a reader opens the Preferences reference
- **THEN** every page name in the running app's Preferences list appears, with
  the disabled pages marked not yet available and Performance linked to its
  detailed page

#### Scenario: The status-bar readouts are explained [dc_status_reference]

- **WHEN** a reader opens the Status bar reference
- **THEN** every readout in the status bar's menu is named and its number or
  text is explained

### Requirement: The View extras are illustrated

The View menu reference SHALL describe each **View ▸ Show** extra — Selection
Edges, Target Path, Layer Edges, Guides, Grid, Count, Smart Guides, Slices,
Notes and Pixel Grid — and SHALL show a canvas screenshot of each one, captured
from Kooka Pictura and stored under `public_docs/images/`.

#### Scenario: Each View extra has a screenshot [dc_view_extras]

- **WHEN** a reader opens the View menu reference
- **THEN** each Show extra has a short description and an image that exists in
  `public_docs/images/`

### Requirement: A guard keeps the menu reference current

The repository SHALL provide a script that reads the command catalogue,
requires every command id with a menu path to appear in its page's hidden
manifest (or in `preferences.md` for a Preferences leaf), and requires every
id without a path to be listed with a reason in an allowlist. The script SHALL
have a self-test, SHALL fail on a manifest id that is not in the catalogue, on
an id filed under the wrong page, on a missing View-extra screenshot, and on a
missing Preferences or status-bar label. The guards workflow and the fast
verification gate SHALL run the script.

#### Scenario: The guard rejects an undocumented command [dc_command_guard]

- **WHEN** a catalogue command with a menu path is absent from its page manifest
  and from the allowlist
- **THEN** the guard fails and names the command id and page

#### Scenario: Internal commands need an allowlist reason [dc_command_guard_allowlist]

- **WHEN** a catalogue command has no menu path
- **THEN** the guard accepts it only when the allowlist names it with a
  non-empty reason

#### Scenario: The guard runs in CI [dc_guard_ci]

- **WHEN** the guards workflow runs on a pull request
- **THEN** it runs the guard's self-test and the guard against the checked-out
  tree
