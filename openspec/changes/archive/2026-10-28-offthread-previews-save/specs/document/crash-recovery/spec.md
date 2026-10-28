# Spec Delta

## ADDED Requirements

### Requirement: Recovery snapshots reuse the encode cache

A recovery snapshot SHALL encode through the document session's PSD encode
cache, so a snapshot written after a native save or another snapshot encodes
only the channels that changed since. Writing a snapshot SHALL NOT clear the
cache. The snapshot's own encode prunes entries it did not use like any other
successful encode, so a later encode may re-encode a channel the snapshot did
not use.

#### Scenario: A snapshot after a save encodes only changed layers [cr_snapshot_cache]

- **WHEN** a document is saved through the session cache, one layer is edited, and a recovery snapshot is written
- **THEN** the snapshot reports one layer with encoded channels, the unchanged layers count as hits, and the snapshot file reopens with the edited pixels
