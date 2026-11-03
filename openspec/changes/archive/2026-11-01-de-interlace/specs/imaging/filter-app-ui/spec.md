# Spec Delta

## MODIFIED Requirements

### Requirement: Filter menu wiring

Every `Filter`-menu entry whose kind has a descriptor MUST be enabled and MUST dispatch its parameter dialog, or apply directly when it has no parameters. A dialog-opening entry's menu label MUST end with an ellipsis (`…`); a parameterless entry MUST not. Entries whose kind has no engine kernel MUST remain disabled. `Custom` is a normal parameterized kind and opens its dialog on the pass-through kernel. A filter command MUST be disabled when the active layer is not an unlocked normal pixel layer.

#### Scenario: An implemented entry is enabled and dispatches

- **WHEN** a document has an unlocked normal pixel layer active and the user opens the Filter menu
- **THEN** implemented entries are enabled and choosing one runs its filter or dialog

#### Scenario: Dialog entries carry an ellipsis

- **WHEN** the Filter menu is shown
- **THEN** every entry that opens a parameter dialog ends with `…`, and every parameterless entry does not

#### Scenario: An unimplemented entry stays disabled

- **WHEN** the user opens the Filter menu
- **THEN** entries with no engine kernel, such as `Filter > Extract…`, are disabled

#### Scenario: No pixel layer disables the filters

- **WHEN** the active layer is missing or is not an unlocked normal pixel layer
- **THEN** the filter commands are disabled
