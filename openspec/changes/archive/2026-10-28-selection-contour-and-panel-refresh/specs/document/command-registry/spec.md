# Spec Delta

## ADDED Requirements

### Requirement: Region-driven command refresh is coalesced

The shell SHALL coalesce command-table recomputations caused by region
presentations to at most one per 120 ms. A notifying committed edit SHALL keep
its synchronous recomputation from the edit's notification and is not delayed by
this coalescing.

#### Scenario: Region presentations recompute the command table once per interval [cr_preview_coalesced]

- **WHEN** several region presentations arrive faster than the coalescing
  interval
- **THEN** the command table's enablement, checked state, and labels are
  recomputed at most once per 120 ms

#### Scenario: A notifying edit still recomputes immediately [cr_preview_commit_unaffected]

- **WHEN** a notifying document edit commits while region presentations are
  pending
- **THEN** the command table is recomputed once from the edit's notification
  without waiting for the coalescing interval
