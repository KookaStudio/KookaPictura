# ui/keyboard-shortcuts-editor Specification

## Purpose
`Edit > Keyboard Shortcuts…` opens CS6's shortcut editor for the application
menus and the tools. Changes apply live and persist as overrides of the
default set.

## Requirements

### Requirement: The editor lists menu commands and tools

The system SHALL open the Keyboard Shortcuts dialog from `Edit > Keyboard
Shortcuts…` (Alt+Shift+Ctrl+K) and from `Window > Workspace > Keyboard
Shortcuts & Menus…`. Its Shortcuts For control SHALL offer Application Menus
(every menu command, grouped by its menu path, with its current shortcut) and
Tools (every tool with its letter).

#### Scenario: Menu commands are listed with shortcuts [ks_lists_menus]

- **WHEN** the dialog opens on Application Menus
- **THEN** `File > New…` is listed under File with `Ctrl+N`

#### Scenario: Tools are listed with letters [ks_lists_tools]

- **WHEN** Shortcuts For is switched to Tools
- **THEN** the Move tool is listed with `V`

### Requirement: Any binding can be remapped in place

The system SHALL open a key field, already holding keyboard focus, in the
Shortcut cell of the row the user selects, so the next key chord pressed is
taken as that row's new binding. The chord SHALL stay pending until Accept,
until another row is chosen, or until OK. Undo SHALL drop a pending chord,
and otherwise step back through accepted edits. A menu command SHALL hold any
number of bindings, one row each: Add Shortcut SHALL open a new blank row for
the command, Delete Shortcut SHALL remove the selected binding, and Use
Default SHALL restore the command's default bindings. A tool shortcut SHALL be
a single letter A–Z other than the window's reserved D, F, Q, and X.

#### Scenario: Typed keys reach the selected row [ks_focus_capture]

- **WHEN** the File > New row is selected and Ctrl+Alt+N is pressed, then
  Accept
- **THEN** the row shows `Ctrl+Alt+N`, and the key field still has focus

#### Scenario: A command gains a second binding [ks_add_binding]

- **WHEN** Add Shortcut is pressed on File > New, Ctrl+Shift+F7 is typed and
  accepted
- **THEN** File > New lists `Ctrl+N` and `Ctrl+Shift+F7`, and OK gives its
  action both shortcuts

#### Scenario: Delete removes one binding [ks_delete_binding]

- **WHEN** the second of two bindings is selected and Delete Shortcut is
  pressed
- **THEN** only the first binding remains

#### Scenario: Use Default restores the default [ks_use_default]

- **WHEN** a command's shortcut was changed and Use Default is pressed on its
  row
- **THEN** the row shows the command's default shortcut

### Requirement: Conflicts are reported in the dialog

Conflicts SHALL be checked across both lists: a tool letter also owns its
Shift+letter cycle key. When a pending chord is already bound to another
command, or is a tool letter (plain or Shift+) held by a tool outside the
edited tool's group, the system SHALL show
that in the dialog's information area together with Accept and Go To
Conflict and Undo Changes. Accept (or Accept and Go To Conflict, which also
selects the other command) SHALL move the binding, removing it from the other
command or tool group. Undo Changes, or choosing another row without
accepting, SHALL keep both previous bindings.

#### Scenario: Accepting a conflict steals the shortcut [ks_conflict_accept]

- **WHEN** Ctrl+N is typed on `File > Open...` and Accept is pressed
- **THEN** `File > Open...` shows `Ctrl+N` and `File > New…` shows none

#### Scenario: Undo Changes keeps the old shortcut [ks_conflict_undo]

- **WHEN** a conflicting chord is typed and Undo Changes is pressed
- **THEN** both rows keep their previous shortcuts

#### Scenario: A menu command cannot silently share a tool key [ks_cross_list]

- **WHEN** Shift+L is typed on `File > Open...`
- **THEN** the notice names the Lasso tools, and accepting gives Open the
  key and removes L from the lasso group, so the key never has two owners

#### Scenario: A tool letter moves between groups [ks_tool_letter]

- **WHEN** the Move tool is given `M` and the conflict is accepted
- **THEN** the Move tool's letter is `M` and no marquee tool keeps `M`

#### Scenario: A modified or reserved tool key is refused [ks_tool_refused]

- **WHEN** Ctrl+M, F2, or X is typed on the Move tool's row
- **THEN** the row keeps `V` and the dialog explains why

### Requirement: Workspace keys stay the workspace's

The system SHALL refuse to assign, in either list, a key the workspace binds
itself: `[`, `]`, Shift+`[`, Shift+`]`, D, X, F, Q, Tab, Shift+Tab, Delete,
Backspace, Shift+F1, and Shift+F2. The dialog SHALL explain the refusal. A
menu command that holds a tool key (from a store written before this check)
SHALL keep it, and the window SHALL not bind that key to the tool as well.

#### Scenario: A workspace key is refused [ks_workspace_refused]

- **WHEN** X is typed on `File > Open...`
- **THEN** the row keeps `Ctrl+O` and the notice says X belongs to the
  workspace

#### Scenario: A stale override stays unambiguous [ks_stale_override]

- **WHEN** the store gives `File > Open...` Shift+L while the lassos keep L
- **THEN** exactly one shortcut in the window owns Shift+L (Open's), and
  plain L still selects the lassos

### Requirement: Changes apply on OK and persist as overrides

The system SHALL apply the pending changes on OK, so the menu actions and
tool keys use them at once, and discard them on Cancel. The session store
SHALL persist only the shortcuts that differ from the defaults, and SHALL
reapply them on the next launch.

#### Scenario: OK applies live [ks_ok_applies]

- **WHEN** `File > New…` is given `Ctrl+Alt+N` and OK is pressed
- **THEN** the File > New action's shortcut is `Ctrl+Alt+N`

#### Scenario: Cancel discards [ks_cancel_discards]

- **WHEN** a shortcut is changed and Cancel is pressed
- **THEN** every action keeps its previous shortcut

#### Scenario: Overrides survive a relaunch [ks_persisted]

- **WHEN** File > New is given a second binding with OK and the main window
  is recreated
- **THEN** the new window's action carries both bindings, and the session
  store holds only that override
