# Design

## Context

The Rust `SaveJob` already freezes a copy-on-write document, reserves memory,
shares its session encode cache, and checks history identity, content revision,
and document generation at completion. The Qt coordinator currently calls it
through `runBusyJob`, then updates the active index captured before waiting.
Close, Revert, update restart, and control callers depend on a completed
boolean result. Recovery currently discovers legacy manifest/PSD pairs in
locked per-instance directories and writes snapshots through a separate worker.

## Goals / Non-Goals

**Goals**

- Carry the originating document's identity from save request to completion.
- Recover committed native document state and retire recovery data only after
  the replacement is durable.

**Non-Goals**

- Persistent undo stacks, restoration of transient session state, and a
  general action replay format.
- New workflows for raster/PDF saves, Save a Copy, Smart Object commits,
  crash reporting, or cloud documents.

## Decisions

### Start and finish interactive saves in the coordinator

Keep the existing worker and introduce an interactive dispatch path from the
File Save and Save As commands after their existing format, path, and options
resolution. Synchronous callers continue through the completed-result path.
Track the job by a stable document handle, generation, destination, and
request origin. Reserve a pending position in the recording session at dispatch, storing the
original command and destination. On success fill that position in dispatch
order even if recording stopped or switched; on failure remove it. Playback
waits for unresolved positions. Hold a claimed destination through completion and compare paths using
the normalized basename and resolved parent when available. Recheck any
applicable job before Close, Revert, quit, or restart presents its existing
unsaved-changes decision.

The worker reports phase boundaries after composite preparation, encoding, and
publication. Qt uses weighted percentages for those phases and renders them
from job state in the tab and visible document's lower-left status area.
Persist the preference alongside the existing File Handling state. Removing
input exclusion from the existing nested wait loop was considered. A job
with an explicit completion has a clearer lifetime and protects synchronous
callers from reentrant UI actions.

On same-generation success record the written history and persisted metadata
identity as the saved anchor, even if a newer state is displayed. Retain the
content-revision stale check when deciding whether the live state is current,
and recompute dirty at completion using the live save identity. A later navigation back to the
written state can clear dirty even if the completed save initially left the
document dirty. Use the existing sibling-temp writer with file sync before
rename and parent-directory sync afterwards. Preserve a distinct
post-rename sync error so the UI can report that publication is uncertain.
Set a sticky protection obligation on the originating document, independent of
history identity; prompt for it and retain or create recovery for the latest
live committed state even if formerly clean. Only a durable save of the current
state, explicit discard, or a successful Revert after the prompt resolves it. [Linux fsync(2)](https://man7.org/linux/man-pages/man2/fsync.2.html)

### Journal forward document changes with a separate sequence

Observe committed changes at `DocumentSession` capture and successful history
restore boundaries, and explicitly observe persisted session metadata mutations
such as Vanishing Point planes that bypass those boundaries. Include their native
persisted-state identity in save snapshots, dirty comparison and journal ordering. Compare consecutive committed native-recoverable states.
Use an independent monotonic sequence because a move can retain content
revision and history navigation can revisit an old history identity. Capture
immutable references on the edit path. Do plane comparison, payload encoding,
and filesystem I/O on the recovery worker. Reuse the history plane and
tile identity queries, with an explicit map of plane slots and layout checks
against the predecessor. Histories' in-memory tile records swap sides when
navigated, so the journal stores forward replacement data.

Use forward tile replacements where the native layout and metadata can be
reconstructed exactly, storing authoritative native samples where they differ
from preview samples. Publish a whole-state checkpoint at the corresponding
sequence for structural, metadata, mode, or other changes for which a delta
cannot be represented faithfully. Treat such a checkpoint as a sequence
boundary and hold later records until its predecessor can be resolved.
Recovery applies changes to a decoded document and rebuilds its composite as
needed before exposing the recovered state. Replaying menu commands would
depend on UI state, undo history, external files, and implementation versions.
Encoding the entire document for every stroke would make ordinary brush edits
pay the full checkpoint cost.

### Batch sync and publish checkpoints in generations

Use one ordered writer per recovery session and a bounded, versioned record
frame with length, chain identity, sequence, predecessor, and checksum.
Track the last durable sequence separately from queued changes. Start a
checkpoint on the first dirty commit. Use the configured autosave interval
and a segment-size threshold to compact thereafter. Target approximately
two seconds from the first pending record to an attempted sync, with an
earlier flush at a batch-size limit. This is a starting point for measurement.
Queue lag, initial checkpoint time, disk latency, and failed I/O affect when
protection is durable. Snapshot-only recovery leaves more edits uncovered.
Syncing each edit before returning would add disk latency to editing. SQLite
WAL offers mature transactional batching and would add a storage dependency
while still requiring a document payload format. A single append writer fits
the existing per-instance store. [SQLite WAL](https://sqlite.org/wal.html)

Place versioned sessions in `recovery-v2/`, a sibling outside the legacy
`recovery/` scanner; scan both roots and preserve the existing legacy quarantine
policy in the existing scanner-excluded quarantine directory, using collision-safe
session names for both stores. Never delete unsupported versions.
Sync every newly created root and session entry through an existing durable
ancestor before acknowledging the first checkpoint or a transfer. A sync failure
keeps the source and reports degraded protection.

Write each checkpoint to a new generation-specific name using the actual
native container. Sync the checkpoint and any new journal segment before
publishing a versioned manifest with the exact checkpoint sequence and local
file references. Sync the directory after creating segment names and after
replacing the manifest. Only then retire the old generation and journal
prefix. The manifest switch must preserve the suffix above its checkpoint
sequence. Readers validate that references are local to the session, decode
only supported versions, and apply whole verified records in order. A bad
tail is reported and replay stops at the valid prefix. Recover into a new
session checkpoint and retire the orphan only after that checkpoint is
durable. Hold the orphan's lock while recovering, discarding, or quarantining
its files so simultaneous launches cannot resolve the same orphan twice.

Process cleanup and writer completion through the same per-document lifecycle.
Manual save completion can clear data only for a durably saved current
revision. A stale save leaves the chain in place. Discard and disabling
recovery invalidate the writer's document epoch before scheduling cleanup so
that in-flight jobs cannot republish deleted files.

### Generate PSD autosave metadata at the checkpoint edge

Durably store the original complete raw resource section in a bounded,
validated sidecar published with each checkpoint, including malformed or
unparsed suffix bytes. If the sidecar cannot be read or validated, retain the
recovery chain and report the failure; do not expose an incomplete recovered
document or retire its source. On recovery, replace the entire temporary
checkpoint resource section with the original sidecar bytes before replay.
This removes generated blocks while restoring imported blocks in their original
order, with duplicates intact; do not subsequently strip blocks by ID.

Use existing image-resource framing helpers on the checkpoint copy. Frame the
parseable checkpoint section without an unparsed suffix so the generated pair is
readable. Replace any prior instances of the two IDs and emit the final checkpoint
path and a Pictura-defined `PSD` or `PSB` value as big-endian PSD Unicode
strings. Preserve the rest according to the existing image-resource contract.
Discovery uses the validated manifest and checkpoint header. Ordinary native
saves keep the existing opaque round-trip behavior. Adobe defines the resource
IDs and string framing and leaves the format token semantics unspecified.
[Adobe PSD specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/)

## Risks / Trade-offs

- Rapid edits can pin old tiles while the worker falls behind. Bound
  outstanding data through the memory governor, retain the last durable
  chain, and report degraded protection. Resume with a newer complete
  checkpoint if the pending chain cannot be retained.
- Large checkpoints and native saves share encode resources. Exercise both
  at once and check that the save's allocation reservation and the shared
  encode cache remain valid. Choose the batching and rotation thresholds
  using observed throughput and memory use.
- Power loss can interrupt any file or directory sync. Use deterministic
  fault injection for publication boundaries and a disposable machine or VM
  with controlled write caches to exercise abrupt power loss.

## Migration Plan

Read an unversioned manifest and its sibling PSD as a checkpoint-only chain.
Write the new manifest format only for new recovery work. Preserve legacy
orphans until explicit discard or successful transfer. An older build scans only the legacy root and cannot remove versioned sessions;
a later current build discovers them. Recovery of a legacy orphan transfers its
state into a durable versioned session before removing the legacy source.
