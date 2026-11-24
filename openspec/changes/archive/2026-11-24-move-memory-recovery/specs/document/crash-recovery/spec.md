# document/crash-recovery Specification

## ADDED Requirements

### Requirement: A recovery snapshot reserves only its own memory

A queued recovery snapshot SHALL reserve only the memory it adds beyond the live
document — its tile tables and non-plane metadata — not the pixel planes it
shares copy-on-write with the live document. The checkpoint encoder's working
memory SHALL be gated separately when it writes. A snapshot that shares a large
document SHALL NOT make a later edit's memory gate refuse work that allocates no
new plane.

#### Scenario: A shared snapshot fits a room below its plane bytes [cr_recovery_cow_reserve]
- **WHEN** a recovery snapshot of a 196 MiB flat document is queued in a room below the document's plane bytes but above its tables and metadata
- **THEN** the snapshot is queued rather than refused
