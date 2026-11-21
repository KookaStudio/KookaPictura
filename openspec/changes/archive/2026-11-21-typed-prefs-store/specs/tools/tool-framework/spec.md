# Spec Delta

## MODIFIED Requirements

### Requirement: Shift-key tool switching preference

The system SHALL store a `Use Shift Key For Tool Switch` preference that defaults
to on and SHALL gate the `Shift` requirement for cycling a tool group on it. The
preference SHALL persist in the preferences store. Legacy session migration
SHALL import the value when present in schema version 4 or later. Missing values
SHALL load the default. Other session fields SHALL continue to round-trip.

#### Scenario: The preference defaults to on [m40_session]

- **WHEN** the application preference is read with no saved value
- **THEN** `Use Shift Key For Tool Switch` is on

#### Scenario: The preference round-trips [m40_session]

- **WHEN** the preference is changed, the session is saved, and the store is reloaded
- **THEN** the changed value is restored alongside the other session fields

#### Scenario: An older store loads the default [m40_session]

- **WHEN** a schema-3 store with neither new field is loaded
- **THEN** the column count is 1 and the preference is on
