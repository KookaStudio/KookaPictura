# ui/search-bar Specification

## Purpose

A keyboard search over every UI entry point: commands, tools, panels,
Preferences pages, and key panel controls. It finds an entry by name, shows
where it lives, and runs or reveals it. CS6 has no such feature; it is a
post-CS6 addition by owner decision and is labelled non-parity.

## ADDED Requirements

### Requirement: Search command and shortcut

The application SHALL register `Help > Search…` as the command `help.search`
with the default shortcut `Shift+Space`. Running it SHALL open the search bar
over the canvas with an empty query and the query field focused. The binding
SHALL be editable in Edit > Keyboard Shortcuts like any other command.

#### Scenario: Shift+Space opens the bar [sb_open]

- **WHEN** the main window has focus outside any text input and the user presses Shift+Space
- **THEN** the search bar is shown with an empty query
- **AND** its query field has keyboard focus

#### Scenario: Status bar hint follows the binding [sb_hint]

- **WHEN** the window is built
- **THEN** the status bar shows a `Search` hint carrying the command's current shortcut
- **WHEN** the command is rebound
- **THEN** the hint shows the new shortcut

### Requirement: Text input and Space pan keep their keys

Shift+Space SHALL NOT open the search bar while a text field or a type edit
has focus; the key SHALL type a space there instead. A bare Space SHALL still
start the canvas Space pan, and a Space pressed with Shift SHALL NOT start it.

#### Scenario: A text field keeps Shift+Space [sb_textfield]

- **WHEN** a line edit has focus and the user presses Shift+Space
- **THEN** the line edit receives a space
- **AND** the search bar stays closed

#### Scenario: A type edit keeps Shift+Space [sb_typeedit]

- **WHEN** a type edit is open on the canvas and the user presses Shift+Space
- **THEN** the typed text gains a space
- **AND** the search bar stays closed

#### Scenario: Space pans and Shift+Space does not [sb_space]

- **WHEN** a document is open and the user presses Space
- **THEN** the canvas enters Space pan
- **WHEN** the user presses Space with Shift held
- **THEN** the canvas does not enter Space pan

### Requirement: Searchable index

The index SHALL contain every registry command, every tool, every panel, every
real Preferences page, and every row of the curated control table. Each entry
SHALL carry its location as a path, with the leaf last. A command's path SHALL
be its menu path; a panel SHALL appear once; and every curated control SHALL
name a live widget in the window.

#### Scenario: Every source is indexed [sb_sources]

- **WHEN** the index is built
- **THEN** it holds a command, a tool, a panel, a preference page, and a control entry
- **AND** no panel appears twice

#### Scenario: Curated controls resolve [sb_controls]

- **WHEN** each curated control is looked up in the window
- **THEN** its container and its control widget both exist

### Requirement: Matching and ranking

A query SHALL match an entry when every whitespace-separated word occurs,
case-insensitively, in the entry's label, a path segment, or a keyword. Results
SHALL rank by exact label, then label prefix, then word prefixes in the label,
then substrings of the label, then matches needing the path or keywords; and
within a tier, runnable entries first, then implemented ones, then shorter
paths.

#### Scenario: Warp finds the transform command [sb_warp]

- **WHEN** the user types `warp`
- **THEN** the first result is `Edit > Transform > Warp`

#### Scenario: Words combine and rank [sb_rank]

- **WHEN** the user types two words that occur only together in one entry's path
- **THEN** only entries containing both words are listed
- **AND** an exact label match ranks above a label prefix, which ranks above a match in the path only

### Requirement: Run and reveal

Enter or a click on a result SHALL run it: a command runs as from its menu, a
tool becomes active, a panel is shown, a preference page opens, and a control
is brought up and focused. Alt+Enter SHALL reveal the result: a command's menu
chain opens on its item, and a tool, panel, or control is outlined in place.
Stub and disabled entries SHALL be listed greyed; Enter and a click SHALL do
nothing on them, and Alt+Enter SHALL still reveal them. The bar SHALL close
after a run or a reveal.

#### Scenario: Enter runs the selected entry [sb_run]

- **WHEN** a document is open, the user searches `warp`, and presses Enter
- **THEN** a Warp transform session starts
- **AND** the search bar closes

#### Scenario: A stub does not run [sb_stub]

- **WHEN** the selected result is a stub command
- **THEN** its row is greyed
- **AND** Enter leaves the bar open and runs nothing

#### Scenario: Alt+Enter reveals the menu chain [sb_reveal]

- **WHEN** the user selects `Edit > Transform > Warp` and presses Alt+Enter
- **THEN** the Edit menu and its Transform submenu open
- **AND** Warp is the Transform submenu's active item

### Requirement: Dismissal

The search bar SHALL close on Escape, on a press outside it, and when its
query field loses focus to anything other than a popup.

#### Scenario: Escape and an outside press close the bar [sb_dismiss]

- **WHEN** the bar is open and the user presses Escape
- **THEN** the bar closes
- **WHEN** the bar is reopened and the user presses outside it
- **THEN** the bar closes
- **WHEN** the bar was closed with Escape and the user presses Shift+Space again
- **THEN** the bar opens again

### Requirement: Look and placement

The search bar SHALL be centred horizontally over the canvas, in its upper
third, and styled from the design-system tokens like a floating panel.

#### Scenario: The bar sits over the canvas [sb_look]

- **WHEN** the bar opens with a document shown
- **THEN** it is centred horizontally over the canvas, near the top third, styled like a floating panel
