# Spec Delta

## ADDED Requirements

### Requirement: A named-snapshot restore discards the retained effect

Restoring a named snapshot SHALL discard the retained last-effect record, so
`Edit ▸ Fade…` is unavailable and applying Fade cannot resolve the pre-restore
layer path in the restored document.

#### Scenario: A named-snapshot restore disables Fade [fade_snapshot_restore_disables]

- **WHEN** a filter has been committed and a named snapshot is restored
- **THEN** the retained effect record is discarded and `Edit ▸ Fade…` is disabled
