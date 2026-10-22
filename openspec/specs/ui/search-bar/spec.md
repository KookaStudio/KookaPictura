# ui/search-bar Specification

## Purpose
A keyboard search over every UI entry point: commands, tools, panels,
Preferences pages, and key panel controls. It finds an entry by name, shows
where it lives, and runs or reveals it. CS6 has no such feature; it is a
post-CS6 addition by owner decision and is labelled non-parity.

## Requirements

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

The index SHALL contain every registry command except `help.search` itself,
every tool, every panel that has a Window > Panels toggle, every real
Preferences page, and every row of the curated control table. A panel without
a toggle, such as the placeholder Styles panel, SHALL be found through its menu
command only. Each entry SHALL carry its location as a path, with the leaf
last, and its labels SHALL drop mnemonic ampersands, an inline shortcut, and
trailing ellipses. A command's path SHALL be its menu path; a panel SHALL
appear once; and every curated control SHALL name a live widget in the window.

#### Scenario: Every source is indexed [sb_sources]

- **WHEN** the index is built
- **THEN** it holds a command, a tool, a panel, a preference page, and a control entry
- **AND** no panel appears twice
- **AND** `help.search` is not indexed

#### Scenario: Labels are cleaned [sb_labels]

- **WHEN** a label carries a mnemonic `&`, a doubled `&&`, an inline tab shortcut, or one or more trailing ellipses
- **THEN** the indexed label drops the mnemonic, keeps one literal `&`, and drops the shortcut and every trailing ellipsis

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

#### Scenario: Any whitespace separates words [sb_whitespace]

- **WHEN** the query's words are separated by a tab or a newline
- **THEN** they match as separate words

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
nothing on them, and Alt+Enter SHALL still open their menu chain. A disabled
menu item cannot be made active, so it SHALL NOT be highlighted, and reveal
SHALL NOT enable it. Up and Down SHALL move the selection one row, stopping at
the first and last rows. The bar SHALL close after a run or a reveal.

#### Scenario: Enter runs the selected entry [sb_run]

- **WHEN** a document is open, the user searches `warp`, and presses Enter
- **THEN** a Warp transform session starts
- **AND** the search bar closes

#### Scenario: Panels, preferences and controls run [sb_run_targets]

- **WHEN** the user presses Enter on a hidden panel's entry
- **THEN** the panel is shown and outlined
- **WHEN** the user presses Enter on a Preferences page's entry
- **THEN** Preferences opens on that page
- **WHEN** the user presses Enter on a drop-down control's entry
- **THEN** the drop-down's list opens
- **WHEN** the user presses Enter on an options-bar control's entry
- **THEN** the control's tool becomes active and its field takes the focus with its text selected

#### Scenario: A stub does not run [sb_stub]

- **WHEN** the selected result is a stub command
- **THEN** its row is greyed
- **AND** Enter leaves the bar open and runs nothing
- **WHEN** the user presses Alt+Enter on it
- **THEN** the bar closes and the stub's menu chain opens

#### Scenario: Alt+Enter reveals the menu chain [sb_reveal]

- **WHEN** the user selects `Edit > Transform > Warp` and presses Alt+Enter
- **THEN** the Edit menu and its Transform submenu open
- **AND** Warp is the Transform submenu's active item

#### Scenario: Alt+Enter outlines a tool, panel or control [sb_reveal_place]

- **WHEN** the user presses Alt+Enter on a tool, a panel, or a control entry
- **THEN** the toolbox slot, the panel, or the control is outlined
- **AND** the active tool is unchanged and a drop-down's list stays closed

#### Scenario: Up and Down move the selection [sb_navigate]

- **WHEN** the result list has several rows and the user presses Up on the first row
- **THEN** the first row stays selected
- **WHEN** the user presses Down past the last row
- **THEN** the last row stays selected

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

#### Scenario: Losing focus closes the bar [sb_focus]

- **WHEN** the query field loses focus to a popup
- **THEN** the bar stays open
- **WHEN** the query field loses focus to another widget
- **THEN** the bar closes

### Requirement: Look and placement

The search bar SHALL be centred horizontally over the canvas, in its upper
third, and styled from the design-system tokens like a floating panel.

#### Scenario: The bar sits over the canvas [sb_look]

- **WHEN** the bar opens with a document shown
- **THEN** it is centred horizontally over the canvas, near the top third, styled like a floating panel
