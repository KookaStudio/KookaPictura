# Spec Delta

## MODIFIED Requirements

### Requirement: The editor lists menu commands and tools

The system SHALL open the Keyboard Shortcuts dialog from `Edit > Keyboard
Shortcuts…` (Alt+Shift+Ctrl+K) and from `Window > Workspace > Keyboard
Shortcuts & Menus…`. Its Shortcuts For control SHALL offer Application Menus
(every menu command, grouped by its menu path, with its current shortcut) and
Tools (every tool with its letter). Application Menus SHALL also list every
implemented panel command that acts on the document, grouped under its panel.
A panel preference that does not act on the document SHALL NOT be listed.

#### Scenario: Menu commands are listed with shortcuts [ks_lists_menus]

- **WHEN** the dialog opens on Application Menus
- **THEN** `File > New…` is listed under File with `Ctrl+N`

#### Scenario: Tools are listed with letters [ks_lists_tools]

- **WHEN** Shortcuts For is switched to Tools
- **THEN** the Move tool is listed with `V`

#### Scenario: Panel commands are listed under their panel [ks_lists_panel_commands]

- **WHEN** the dialog opens on Application Menus
- **THEN** an implemented Actions, History, Character, Paragraph, Adjustments, or Layers panel command is listed under that panel
