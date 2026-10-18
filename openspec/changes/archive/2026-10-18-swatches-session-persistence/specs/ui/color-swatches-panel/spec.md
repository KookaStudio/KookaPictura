## ADDED Requirements

### Requirement: Swatches persist across sessions

The Swatches panel's contents SHALL persist in the application session store and
SHALL be restored on the next launch. A store without swatch data SHALL open the
default palette; a stored empty panel SHALL stay empty. Saving an unrelated
setting SHALL preserve the stored swatches.

#### Scenario: Added swatches survive a restart [sw_persist_restart]

- **WHEN** a swatch is added and a new window is created
- **THEN** the panel contains the added swatch

#### Scenario: An empty panel survives a restart [sw_persist_empty]

- **WHEN** every swatch is removed and a new window is created
- **THEN** the panel is empty and the default palette is not restored

#### Scenario: An older store opens the default palette [sw_persist_default]

- **WHEN** the session store has no swatch data
- **THEN** the panel holds the default palette

#### Scenario: An unrelated save keeps the swatches [sw_persist_unrelated_save]

- **WHEN** another setting is saved without touching the panel
- **THEN** the stored swatches are unchanged
