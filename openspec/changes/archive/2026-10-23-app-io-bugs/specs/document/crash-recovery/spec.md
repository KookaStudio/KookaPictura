# Spec Delta: document/crash-recovery

## MODIFIED Requirements

### Requirement: Recover after an unclean exit

At launch, when orphaned sessions hold snapshots, the app SHALL offer to
Recover, Discard, or keep them for later. Recover SHALL reopen each snapshot
as an untitled document with unsaved changes named `<name>-Recovered` (before
the extension), keep its layers and paths, and write it into the new session's
store. A successfully recovered document's snapshot and manifest SHALL then be
removed from the orphaned session. A document whose snapshot cannot be opened
SHALL NOT be deleted: the app SHALL move its snapshot and manifest into
`$XDG_STATE_HOME/kooka-pictura/recovery/quarantine/<session>/` and continue
with the remaining documents. The orphaned session directory SHALL be deleted
after every document has been recovered or quarantined; when a quarantine move
or the write of a recovered document's new snapshot fails, the session directory
SHALL be kept rather than discarded. The `quarantine` directory
SHALL NOT be scanned as an orphaned session, so its contents are never discarded
as an empty session. Quarantined snapshots SHALL NOT be offered as recoverable
documents on the next launch. Discard SHALL delete the orphaned sessions; Later
SHALL keep them.

#### Scenario: A crashed document comes back

- **WHEN** the `tst_recovery` test leaves a snapshot of a 64×48 Untitled-1 with a Work Path in a lock-less session and recovers it in a new window
- **THEN** one document opens as `Untitled-1-Recovered`, 64×48, dirty, with no file path and its Work Path, the orphaned session is deleted, and the new session holds its snapshot

#### Scenario: A partial recovery quarantines the failed document

- **WHEN** an orphaned session holds one readable snapshot and one unreadable snapshot and Recover runs
- **THEN** the readable document opens as `-Recovered`, the unreadable snapshot and its manifest exist under the recovery store's `quarantine/<session>/` directory, the orphaned session is deleted, a second orphan scan reports no session for the quarantine directory, and the next launch offers no recovered copy of either document
