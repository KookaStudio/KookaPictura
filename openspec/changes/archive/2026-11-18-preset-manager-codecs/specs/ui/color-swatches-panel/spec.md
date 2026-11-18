# Color Swatches Panel

## MODIFIED Requirements

### Requirement: Swatches persist across sessions

The Swatches panel's contents SHALL persist in the authoritative native current
list and SHALL be restored on the next launch. When both that list and legacy
swatch data are absent the default palette SHALL open; a stored empty panel
SHALL stay empty. Legacy session data SHALL migrate without loss. Saving an
unrelated setting SHALL preserve the stored swatches.

#### Scenario: Added swatches survive a restart [sw_persist_restart]

- **WHEN** a swatch is added and a new window is created
- **THEN** the panel contains the added swatch

#### Scenario: An empty panel survives a restart [sw_persist_empty]

- **WHEN** every swatch is removed and a new window is created
- **THEN** the panel is empty and the default palette is not restored

#### Scenario: An older store opens the default palette [sw_persist_default]

- **WHEN** neither the current list nor the legacy session store has swatch data
- **THEN** the panel holds the default palette

#### Scenario: An unrelated save keeps the swatches [sw_persist_unrelated_save]

- **WHEN** another setting is saved without touching the panel
- **THEN** the stored swatches are unchanged
