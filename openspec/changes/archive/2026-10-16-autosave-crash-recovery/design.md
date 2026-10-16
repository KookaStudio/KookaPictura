# Design: autosave-crash-recovery

## Store layout

```text
$XDG_STATE_HOME/kooka-pictura/recovery/
  <pid>-<launch ms>/      one per running instance
    lock                  QLockFile, stale only when its PID is gone
    <id>.psd              latest snapshot of document <id>
    <id>.json             {"name", "originalPath"}
```

`<id>` is a per-instance counter assigned when a document is added, so a
renamed or re-saved tab keeps its file. The lock uses `setStaleLockTime(0)`:
staleness is decided by PID (and process name) only, so a long-running
instance's session can never be taken over because of its age.

An orphan scan try-locks every other session directory; success proves the
owner is gone. The lock is released straight away so the session stays
discoverable if the user picks **Later**.

## Snapshot write

`recovery_write` clones the document (layer and composite planes are
`Arc`-backed copy-on-write, so the clone is cheap and live edits never race
it) and encodes it on a `std::thread`: `refresh_native_composite`,
`save_view`, `write_psd` (PSB past the PSD size limit), then a synced sibling
temp file renamed into place. The view keeps the `JoinHandle`;
`recovery_wait` joins it, and the frame waits before deleting a snapshot,
closing a document, or shutting down. A document that is mid-stroke is
skipped until the next tick.

Change tracking is the view's `changed` signal: a document is re-snapshotted
only after a change since its last snapshot, and only while it is dirty. A
tick that finds a clean document drops its snapshot.

## Lifecycle

- `main.cpp` enables recovery only for interactive launches and posts
  `offerRecovery` after the window shows.
- The frame destructor runs `shutdownRecovery` (a clean exit; a crash never
  runs it), which waits for writes and deletes the session.
- Recover reopens each orphaned snapshot through `recovery_open` (open, clear
  the path, mark dirty), snapshots the recovered documents into the new
  session, waits, and only then deletes the orphaned sessions.

## Path resources

Path resource data is the bare path records of a `vmsk` block (no version or
flags), so `vector_mask.rs` exposes `encode_path_records` /
`decode_path_records` and both formats share them. On write, the section is
re-emitted unchanged when its path resources already decode to the document's
paths (an unedited file keeps its bytes); otherwise every 1025 / 2000-2997
resource is replaced. Coordinates are 8.24 fractions of the document size, so
a round trip is exact to within ~1e-5 px.
