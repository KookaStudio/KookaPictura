# Spec Delta

## MODIFIED Requirements

### Requirement: Changes apply on OK and persist as overrides

The system SHALL apply the pending changes on OK, so the menu actions and
tool keys use them at once, and discard them on Cancel. The preferences store
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
- **THEN** the new window's action carries both bindings, and the preferences
  store holds only that override
