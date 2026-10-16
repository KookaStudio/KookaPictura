## ADDED Requirements

### Requirement: Recovery session store

An interactive launch SHALL create its own recovery session directory under
`$XDG_STATE_HOME/kooka-pictura/recovery/`, held by a lock that is stale only
when its owning process is no longer running. A clean exit SHALL delete the
directory. The self-test, `--headless`, and `--control` runs SHALL NOT create
one.

#### Scenario: A clean exit leaves nothing behind

- **WHEN** the `tst_recovery` test enables recovery, snapshots a modified document, and destroys the window
- **THEN** the session directory existed with its lock while the window lived, is gone afterwards, and no orphan remains

#### Scenario: A live session is not an orphan

- **WHEN** the `tst_recovery` test scans for orphans while another session directory's lock is held
- **THEN** neither the scanning session nor the locked one is reported, and the other session is reported once its lock is released

### Requirement: Autosave snapshots

With Automatically Save Recovery Information on, the app SHALL write, at the
chosen interval, a PSD snapshot of every open document that has unsaved
changes made since its last snapshot, without changing the document's file,
path, or dirty state, and without blocking the UI on the encode. Saving or
closing a document SHALL delete its snapshot. Turning the preference off SHALL
delete every snapshot and stop writing them.

#### Scenario: Only unsaved changes are snapshotted

- **WHEN** the `tst_recovery` test runs the autosave tick on an unmodified document, after drawing a Work Path on it, and again with no further change
- **THEN** the first tick writes nothing, the second writes one snapshot, and the third does not write it again

#### Scenario: Save and close drop the snapshot

- **WHEN** the `tst_recovery` test snapshots two modified documents, saves one, and closes the other
- **THEN** the save deletes the first snapshot and the close deletes the second snapshot and its manifest

### Requirement: Recover after an unclean exit

At launch, when orphaned sessions hold snapshots, the app SHALL offer to
Recover, Discard, or keep them for later. Recover SHALL reopen each snapshot
as an untitled document with unsaved changes named `<name>-Recovered` (before
the extension), keep its layers and paths, write it into the new session's
store, and delete the orphaned session. Discard SHALL delete the orphaned
sessions; Later SHALL keep them.

#### Scenario: A crashed document comes back

- **WHEN** the `tst_recovery` test leaves a snapshot of a 64×48 Untitled-1 with a Work Path in a lock-less session and recovers it in a new window
- **THEN** one document opens as `Untitled-1-Recovered`, 64×48, dirty, with no file path and its Work Path, the orphaned session is deleted, and the new session holds its snapshot

### Requirement: File Handling autosave preference

Edit > Preferences > File Handling SHALL offer "Automatically Save Recovery
Information Every" (default on) with the CS6 intervals 5, 10, 15, 30 minutes,
and 1 hour (default 10 minutes), persisted across launches.

#### Scenario: Changing the interval and turning it off

- **WHEN** the `tst_recovery` test opens File Handling, picks 30 minutes, then clears the checkbox
- **THEN** the page shows the defaults on open, the frame and the session store take 30 minutes, and turning it off is persisted and deletes the open document's snapshot
