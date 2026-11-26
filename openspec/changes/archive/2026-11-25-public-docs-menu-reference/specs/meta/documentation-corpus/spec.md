## ADDED Requirements

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
