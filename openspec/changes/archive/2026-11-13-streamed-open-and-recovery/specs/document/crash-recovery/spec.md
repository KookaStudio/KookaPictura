## MODIFIED Requirements

### Requirement: Recover after an unclean exit

At launch, when orphaned sessions hold snapshots, the app SHALL offer to
Recover, Discard, or keep them for later. Recover SHALL reopen each snapshot
as an untitled document with unsaved changes named `<name>-Recovered` (before
the extension), keep its layers and paths, and write it into the new session's
store. A successfully recovered document's snapshot and manifest SHALL then be
removed from the orphaned session.

A document whose snapshot open is refused for lack of memory SHALL NOT be
quarantined or deleted: its snapshot and manifest SHALL stay in the orphaned
session, the session directory SHALL be kept, and the next launch SHALL offer
that document again.

A document whose snapshot cannot be opened for any other reason SHALL NOT be
deleted: the app SHALL move its snapshot and manifest into
`$XDG_STATE_HOME/kooka-pictura/recovery/quarantine/<session>/` and continue
with the remaining documents.

The orphaned session directory SHALL be deleted after every document has been
recovered or quarantined. When a quarantine move or the write of a recovered
document's new snapshot fails, the session directory SHALL be kept rather than
discarded. The `quarantine` directory SHALL NOT be scanned as an orphaned
session, so its contents are never discarded as an empty session. Quarantined
snapshots SHALL NOT be offered as recoverable documents on the next launch.

When any document is not recovered, the app SHALL show a dialog that lists each
such document with its reason, and names the kept snapshot's path for a memory
refusal or the quarantine directory for an unreadable snapshot. The dialog
SHALL NOT block the recovery that shows it. Discard SHALL delete the orphaned
sessions; Later SHALL keep them.

#### Scenario: A crashed document comes back

- **WHEN** the `tst_recovery` test leaves a snapshot of a 64×48 Untitled-1 with a Work Path in a lock-less session and recovers it in a new window
- **THEN** one document opens as `Untitled-1-Recovered`, 64×48, dirty, with no file path and its Work Path, the orphaned session is deleted, the new session holds its snapshot, and no failure dialog is shown

#### Scenario: A partial recovery quarantines the failed document

- **WHEN** an orphaned session holds one readable snapshot and one unreadable snapshot and Recover runs
- **THEN** the readable document opens as `-Recovered`, the unreadable snapshot and its manifest exist under the recovery store's `quarantine/<session>/` directory, the orphaned session is deleted, a second orphan scan reports no session for the quarantine directory, the next launch offers no recovered copy of either document, and a failure dialog names the unreadable document and its quarantine directory

#### Scenario: A memory refusal is offered again [cr_memory_refused]

- **WHEN** an orphaned session holds a small readable snapshot and a 2048×2048 snapshot, and Recover runs with the governor's room pinned too small for the large one and no scratch space
- **THEN** the small document opens as `-Recovered` and its files leave the orphaned session; the large snapshot and its manifest stay in the session, which is not deleted, and nothing of it is quarantined; a failure dialog names the large document as refused for memory, with its snapshot path; and with room again the next recovery offer lists only the large document, and Recover opens it
