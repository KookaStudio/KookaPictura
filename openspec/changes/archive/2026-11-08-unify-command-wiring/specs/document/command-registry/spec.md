# Spec Delta

## MODIFIED Requirements

### Requirement: Declarative command table
The system SHALL define every menu command once in a declarative table authored in
Rust, keyed by a stable string identifier, carrying the menu path, label, default
shortcut, implemented flag, and checkable flag. The C++ shell SHALL
consume that catalogue and SHALL NOT maintain a second hand-authored command
table or a second hand-authored frozen identifier list. The menu bar SHALL be
built from this table rather than from hand-assembled `QAction`s. The frozen
command identifiers SHALL be generated from the Rust catalogue, and a drift check
SHALL fail when the generated C++ header differs from the catalogue. A command
whose action is not a menu-bar command SHALL be declared with no menu path; it
SHALL NOT appear in the menu bar but SHALL remain available to panel surfaces and
the shortcut editor. A handler SHALL be registered against the command's frozen
identifier. Path-to-identifier derivation SHALL be limited to Rust data-catalogue
leaves (the adjustment catalogue and filter descriptors). A derived identifier
that the catalogue does not contain SHALL be rejected: the bridge SHALL return an
empty identifier and emit a warning naming the path.

#### Scenario: Menu bar is generated from the table
- **WHEN** the application frame is constructed
- **THEN** each top-level menu and child item corresponds to an entry in the command table

#### Scenario: Command identifiers are stable
- **WHEN** a command's label or shortcut is changed in the table
- **THEN** its identifier is unchanged and dispatch by identifier still resolves it

#### Scenario: No second C++ command table exists [cr_rust_only]
- **WHEN** the repository is inspected
- **THEN** the command entries and frozen identifiers exist only in the Rust catalogue and its generated C++ header, not in a hand-authored C++ table

#### Scenario: Frozen identifier header matches [cr_id_drift]
- **WHEN** the Rust catalogue and the generated C++ identifier header are compared
- **THEN** every identifier name and value matches, and a mismatch fails the check

#### Scenario: Inspection output is unchanged [cr_list_commands]
- **WHEN** the control server's `list_commands` method is called before and after the catalogue move
- **THEN** the command id, path, label, implemented, enabled, and checked values are unchanged

#### Scenario: A panel command is absent from the menu bar [cr_panel_command_no_menu]
- **WHEN** the application frame is constructed
- **THEN** a panel-only command is absent from the menu bar but present in the command table and the shortcut editor

#### Scenario: A derived identifier is validated [cr_id_derived_validated]
- **WHEN** a caller derives a command identifier from a path that the catalogue does not contain
- **THEN** the bridge returns an empty identifier and emits a warning naming the path

### Requirement: Dynamic command labels
The system SHALL update labels whose meaning depends on state without rebuilding
the menu. The registry's label provider SHALL be the source for the menu bar, a
panel row menu, and a panel flyout, which SHALL show the same label for the same
command. The undo command SHALL read "Undo" or "Redo" according to the current
history state. Duplicate and Delete SHALL read per kind and selection size
("Duplicate Layer…"/"Duplicate Layers…"/"Duplicate Group…" and the matching
Delete labels), and Merge SHALL read "Merge Down" or "Merge Layers" per the
selection.

#### Scenario: Undo label tracks history
- **WHEN** the document has no undoable step
- **THEN** the edit-history menu item reflects that no undo is available

#### Scenario: Label updates after an edit
- **WHEN** a document edit is recorded and the Edit menu is reopened
- **THEN** the history command label reflects the new state

#### Scenario: Surfaces agree on a dynamic label [cr_dynamic_label_surfaces]
- **WHEN** the Layer menu, the Layers row menu, and the Layers flyout are opened for a multi-selection
- **THEN** Duplicate and Delete read the plural labels on all three

## ADDED Requirements

### Requirement: Modal command metadata

The command catalogue SHALL record which commands open a modal dialog or reach the
unsaved-changes prompt. The control server SHALL refuse such a command from that
metadata before invoking its handler, and the C++ shell SHALL NOT keep a separate
hand-authored modal list. The request-scoped dialog guard SHALL remain in place
for a dialog reached by any other path.

#### Scenario: A declared modal command is refused [cr_modal_metadata]
- **WHEN** a client dispatches a command the catalogue marks modal
- **THEN** the response is `refused` and the handler does not run

#### Scenario: A non-modal command dispatches [cr_modal_not_flagged]
- **WHEN** a client dispatches an implemented, enabled command the catalogue does not mark modal
- **THEN** the handler runs

### Requirement: Panel command surfaces

Every command offered by a panel menu, flyout, or footer that maps to a document
command SHALL resolve to a registry command: its action SHALL carry the command's
identifier, and its enabled, checked, and label state SHALL come from the
registry. Rename (an inline tree edit) and the Color Label submenu MAY remain
panel controls. The Background menu rows SHALL map to registry ids. A panel
action that is not a menu-bar command SHALL be declared in the catalogue with no
menu path. Triggering a panel action SHALL dispatch its identifier. The Keyboard
Shortcuts editor SHALL list an implemented panel command grouped by its panel. A
panel preference that does not act on the document MAY remain a panel-local
control.

#### Scenario: A row action is a registry command [cr_panel_row_action]
- **WHEN** a panel menu row that maps to a document command is opened
- **THEN** its action carries the registry command identifier and its enabled, checked, and label state equals the registry's

#### Scenario: A panel action takes a shortcut [cr_panel_shortcut]
- **WHEN** the Keyboard Shortcuts editor lists application menu commands
- **THEN** an implemented panel action is listed under its panel

#### Scenario: A preference is not a command [cr_panel_preference]
- **WHEN** a panel menu shows a preference toggle that does not act on the document
- **THEN** it does not appear in the command list
