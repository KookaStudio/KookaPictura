# Proposal

## Why

Native PSD and PSB saves already encode on a worker, but the shell defers input until the write finishes. Crash recovery currently restores the last timed snapshot, so a crash can lose up to a recovery interval of newer edits. [Issue #637](https://github.com/KookaStudio/KookaPictura/issues/637)

## What Changes

- Add a persisted **Save in Background** preference, enabled by default. Interactive PSD and PSB Save and Save As allow editing and tab switching during a write, with progress in the document tab and lower-left status area. Close, quit, and restart honor the save result and any edits made after the saved revision.
- Keep the existing recovery preference and checkpoint interval. Add a per-document journal of committed changes, synced in batches by a worker. Recovery restores the latest valid state and keeps the current treatment of unreadable files and memory refusals.
- Write PSD image resources 1086 and 1087 into Pictura recovery checkpoints. Preserve imported blocks on ordinary native saves and use the recovery manifest for discovery.
- Use the existing PSD/PSB worker for native saves. Raster and PDF saves, Save a Copy, and Smart Object commits retain their existing completion behavior. Crash reporting and cloud storage are outside this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/document-lifecycle` gains editable native saves, document progress, and completion rules for in-flight saves.
- `document/crash-recovery` gains durable journaling and revision-aware checkpoints, cleanup, and recovery.
- `codec/psd-image-resources` gains the checkpoint-only generation policy for resources 1086 and 1087.

## Impact

- The import/save coordinator, File Handling preferences, session settings, document tab, and window status area handle interactive save jobs and their outcomes.
- `pictura-app` reuses the Rust save worker, edit and history boundaries, and per-instance recovery store. The checkpoint and journal protocol uses a versioned store isolated from legacy scanners and reads legacy snapshots.
- `pictura-codec` reuses the PSD/PSB writers and the image-resource framing helpers. The planned implementation uses standard-library I/O and existing workspace dependencies.
