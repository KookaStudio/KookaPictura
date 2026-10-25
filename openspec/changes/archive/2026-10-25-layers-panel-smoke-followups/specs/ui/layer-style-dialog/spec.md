## ADDED Requirements

### Requirement: Group Blending Options

The Layer Style dialog SHALL open for a group when the row menu's Blending
Options… is chosen. It SHALL open on the Blending Options page. The ten effect
rows SHALL remain visible but disabled, with a not-implemented tooltip, until the
compositor renders group effects.

#### Scenario: A group opens Blending Options only

- **WHEN** Blending Options… is chosen for a group
- **THEN** the dialog opens on page 0 and every effect row is disabled
